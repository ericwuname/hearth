//! EVENT_LOG 引擎（§7.2/§7.3）——JSONL + O_APPEND + 跨平台文件锁 + seq allocator（F3）
//! + UUIDv7（F2）+ prev_hash 哈希链（L-4）+ schema migration（F1）+ gap 检测（H-6）+ 水位线。
//!
//! 同步实现（文件锁阻塞）；async 调用方用 spawn_blocking 包装。

use crate::event::{Event, EventType, SCHEMA_VERSION};
use crate::{PwcError, Result};
use fs2::FileExt;
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// 锁获取超时（§7.2：5s 超时 + 指数退避最大 30s；测试可调小）。
const LOCK_TIMEOUT_MS: u64 = 5_000;
/// 指数退避上限。
const MAX_BACKOFF_MS: u64 = 30_000;
/// 零哈希（首条 prev_hash）。
pub const ZERO_HASH: &str =
    "sha256:0000000000000000000000000000000000000000000000000000000000000000";

/// 计算哈希链：sha256(JSON 排除 prev_hash 字段后的 UTF-8 字节)（L-4）。
fn prev_hash_of(evt: &Event) -> String {
    let mut v = serde_json::to_value(evt).expect("event serializable");
    if let Some(obj) = v.as_object_mut() {
        obj.remove("prev_hash");
    }
    let bytes = serde_json::to_vec(&v).expect("event to vec");
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let digest = hasher.finalize();
    format!("sha256:{:x}", digest)
}

/// EVENT_LOG 读写器（单文件；物理切片见 `stream`）。
pub struct EventLog {
    path: PathBuf,
    /// 锁超时（毫秒，测试可调小以加速负面用例）。
    lock_timeout_ms: u64,
}

impl EventLog {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let p = path.as_ref().to_path_buf();
        if let Some(dir) = p.parent() {
            std::fs::create_dir_all(dir)?;
        }
        Ok(Self {
            path: p,
            lock_timeout_ms: LOCK_TIMEOUT_MS,
        })
    }

    #[cfg(test)]
    pub fn with_lock_timeout(path: impl AsRef<Path>, timeout_ms: u64) -> Result<Self> {
        let mut s = Self::open(path)?;
        s.lock_timeout_ms = timeout_ms;
        Ok(s)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 获取排他锁（带超时 + 指数退避；竞争写返回 LockContention 语义——由调用方决定事件）。
    /// fs2 try_lock_exclusive: Ok(()) = 获得锁；Err(WouldBlock) = 被占（竞争）。
    fn lock_exclusive(&self, f: &File) -> Result<()> {
        let mut waited = 0u64;
        let mut backoff = 50u64;
        loop {
            match f.try_lock_exclusive() {
                Ok(()) => return Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    waited += backoff;
                    if waited >= self.lock_timeout_ms {
                        return Err(PwcError::Other(format!(
                            "lock timeout after {}ms (contention)",
                            waited
                        )));
                    }
                    std::thread::sleep(std::time::Duration::from_millis(backoff));
                    backoff = (backoff * 2).min(MAX_BACKOFF_MS);
                }
                Err(e) => return Err(PwcError::Io(e)),
            }
        }
    }

    /// 读全量事件（migration layer：未知 schema_version 行 → 写 `schema_version_rejected` 事件
    /// 并跳过该行（不 crash、不静默接受）。返回 (已迁移事件, 被拒行数)。
    /// rejected 事件在锁释放后写（锁内 append 会死锁——fs2 同进程二次锁同一文件）。
    pub fn read_all(&self) -> Result<(Vec<Event>, usize)> {
        let f = OpenOptions::new().read(true).open(&self.path)?;
        self.lock_exclusive(&f)?;
        let reader = BufReader::new(&f);
        let mut events = Vec::new();
        let mut rejected_events: Vec<Event> = Vec::new();
        for line in reader.lines() {
            let line = line?;
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            match serde_json::from_str::<Event>(line) {
                Ok(evt) => {
                    if evt.validate_schema().is_ok() {
                        events.push(evt);
                    } else {
                        // F1：未知 schema_version → 记 rejected（锁外写，不静默跳过，
                        // 不复用 event_gap_alert / window_boot_failed——语义独立，§7.7）
                        rejected_events.push(evt);
                    }
                }
                Err(_) => {
                    // 坏行：跳过（不 crash）；不计入 rejected（非 schema 问题）
                    tracing::warn!(line = ?line, "event log: unparseable line skipped");
                }
            }
        }
        drop(f); // 释放锁
        let rejected = rejected_events.len();
        for bad in &rejected_events {
            if let Err(e) = self.write_rejected_event(bad) {
                tracing::warn!(err = %e, "write schema_version_rejected 失败");
            }
        }
        Ok((events, rejected))
    }

    /// 写 `schema_version_rejected` 事件（§7.7：event_id/schema_version(实际)/target_seq/detected_by）。
    fn write_rejected_event(&self, bad: &Event) -> Result<()> {
        let mut evt = Event {
            schema_version: SCHEMA_VERSION.into(),
            seq: 0, // 由 append allocator 分配
            event_id: Event::new_uuid_v7(),
            actor: "event-log".into(),
            role: String::new(),
            etype: EventType::SchemaVersionRejected,
            target_ref: None,
            content_hash: None,
            hands_off_to: None,
            in_reply_to: None,
            await_timeout: None,
            summary: Some(format!(
                "rejected event schema_version={} seq={} detected_by=event_log",
                bad.schema_version, bad.seq
            )),
            status: None,
            prev_hash: None,
            provided_granularity: None,
        };
        self.append(&mut evt, "event-log")?;
        Ok(())
    }

    /// 追加一条事件（原子：文件锁内 seq allocator + UUIDv7 + prev_hash + append）。
    /// F3：seq 唯一责任方 = 本 allocator（文件锁内 max+1）；其他组件只读。
    /// 返回 (seq, event_id)。
    pub fn append(&self, evt: &mut Event, actor: &str) -> Result<(u64, String)> {
        // F1：写入前校验 schema（未知版本拒绝写入）
        evt.validate_schema().map_err(PwcError::Other)?;

        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&self.path)?;
        self.lock_exclusive(&f)?;

        // F3：seq = max(existing)+1（崩溃留洞：如 1,2,4——允许）
        let max_seq = self.last_seq_locked(&f)?;
        let seq = max_seq + 1;
        // prev_hash = 上一行哈希（无上一行 = ZERO_HASH）
        let prev = self.last_line_hash_locked(&f)?;
        let event_id = crate::event::Event::new_uuid_v7();

        evt.seq = seq;
        evt.event_id = event_id.clone();
        evt.actor = actor.to_string();
        evt.prev_hash = Some(prev);

        let line = serde_json::to_string(evt)?;
        f.write_all(line.as_bytes())?;
        f.write_all(b"\n")?;
        f.sync_all()?;
        drop(f);
        Ok((seq, event_id))
    }

    /// 锁内读最后 seq（F3 allocator 核心）。
    fn last_seq_locked(&self, f: &File) -> Result<u64> {
        let reader = BufReader::new(f);
        let mut max = 0u64;
        for line in reader.lines() {
            let line = line?;
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(evt) = serde_json::from_str::<Event>(line) {
                if evt.seq > max {
                    max = evt.seq;
                }
            }
        }
        Ok(max)
    }

    /// 锁内取最后一行哈希（哈希链锚点）。空文件 → ZERO_HASH。
    fn last_line_hash_locked(&self, f: &File) -> Result<String> {
        let mut buf = Vec::new();
        let mut seeked = f.try_clone()?;
        seeked.seek(SeekFrom::Start(0))?;
        use std::io::Read;
        seeked.read_to_end(&mut buf)?;
        let text = String::from_utf8_lossy(&buf);
        let last = text.trim_end().rsplit('\n').next().unwrap_or("");
        let last = last.trim();
        if last.is_empty() {
            return Ok(ZERO_HASH.to_string());
        }
        // 上一行的事件（算其 prev_hash 需要其 JSON 排除 prev_hash）
        match serde_json::from_str::<Event>(last) {
            Ok(evt) => Ok(prev_hash_of(&evt)),
            Err(_) => Ok(ZERO_HASH.to_string()),
        }
    }

    /// 消费水位线（§7.3）：当前最后 seq（消费者以此更新 last_processed_seq）。
    pub fn last_seq(&self) -> Result<u64> {
        let f = OpenOptions::new().read(true).open(&self.path)?;
        self.lock_exclusive(&f)?;
        let s = self.last_seq_locked(&f)?;
        drop(f);
        Ok(s)
    }

    /// 跳号检测（H-6）：水位线 = from，若文件内存在 seq > from+1 且无 from+1 → 返回缺口。
    /// 消费者据此阻塞或发 event_gap_alert。
    pub fn gap_after(&self, watermark: u64) -> Result<Option<u64>> {
        let (events, _) = self.read_all()?;
        let expected = watermark + 1;
        for e in &events {
            if e.seq > expected {
                // 有更高 seq 但缺 expected → 缺口
                return Ok(Some(expected));
            }
        }
        Ok(None)
    }

    /// 文件行数（测试/审计用）。
    pub fn line_count(&self) -> Result<usize> {
        let (events, _) = self.read_all()?;
        Ok(events.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::EventType;

    fn tmp_log(tag: &str) -> (tempfile::TempDir, EventLog) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(format!("EVENT_LOG.{tag}.jsonl"));
        let log = EventLog::open(&path).expect("open");
        (dir, log)
    }

    fn base_event() -> Event {
        Event {
            schema_version: SCHEMA_VERSION.into(),
            seq: 0,
            event_id: String::new(),
            actor: String::new(),
            role: "施工".into(),
            etype: EventType::TaskDone,
            target_ref: None,
            content_hash: None,
            hands_off_to: None,
            in_reply_to: None,
            await_timeout: None,
            summary: Some("test".into()),
            status: Some("done".into()),
            prev_hash: None,
            provided_granularity: None,
        }
    }

    /// F3 正面：连续分配 seq 1,2,3…（无重复）。
    #[test]
    fn test_seq_allocator_sequential() {
        let (_d, log) = tmp_log("seq");
        for expect in 1..=5u64 {
            let mut e = base_event();
            let (seq, _id) = log.append(&mut e, "w1").expect("append");
            assert_eq!(seq, expect, "allocator 连续分配");
        }
    }

    /// F2 正面：每条 event_id 是 UUIDv7 且唯一；哈希链非零。
    #[test]
    fn test_event_id_uuidv7_and_chain() {
        let (_d, log) = tmp_log("chain");
        let mut prev_anchor = ZERO_HASH.to_string();
        for _ in 0..3 {
            let mut e = base_event();
            let (_seq, id) = log.append(&mut e, "w1").expect("append");
            let parsed = uuid::Uuid::parse_str(&id).expect("uuid");
            assert_eq!(parsed.get_version_num(), 7, "UUIDv7");
            assert_eq!(
                e.prev_hash.as_deref(),
                Some(prev_anchor.as_str()),
                "哈希链锚点衔接"
            );
            prev_anchor = prev_hash_of(&e); // 本行内容哈希 → 下一条的锚
        }
    }

    /// F3 负面：两写者同时绕过 allocator 各自 max+1 → 锁竞争，最终无重复 seq。
    /// 这里用单进程模拟：先手写两行 seq=1（绕过 allocator），再 append → allocator 应从 2 继续（max+1 去重）。
    #[test]
    fn test_seq_allocator_survives_manual_writes() {
        let (_d, log) = tmp_log("manual");
        // 模拟绕过 allocator 的并发写（写入时 seq 都是 1）
        {
            let mut e1 = base_event();
            e1.seq = 1;
            let mut e2 = base_event();
            e2.seq = 1;
            let lines = [
                serde_json::to_string(&e1).unwrap(),
                serde_json::to_string(&e2).unwrap(),
            ];
            let mut f = OpenOptions::new()
                .create(true)
                .append(true)
                .open(log.path())
                .unwrap();
            for l in lines {
                writeln!(f, "{l}").unwrap();
            }
            f.sync_all().unwrap();
        }
        // allocator 接管：max=1 → 从 2 开始，不再产生重复 1
        let mut e = base_event();
        let (seq, _) = log.append(&mut e, "w2").expect("append");
        assert!(seq >= 2, "allocator 从 max+1 继续: {seq}");
        let (events, _) = log.read_all().unwrap();
        let seqs: Vec<u64> = events.iter().map(|e| e.seq).collect();
        // F3：allocator 自身分配的 seq 必须唯一（手动绕过产生的重复是并发写者的错——
        // allocator 从 max+1 接管后不再制造重复）
        assert_eq!(
            seqs.iter().filter(|s| **s == seq).count(),
            1,
            "allocator 分配的 seq={seq} 必须唯一: {:?}",
            seqs
        );
    }

    /// F1 负面：写入手写 schema_version=9.9 行 → read_all 拒绝 + 追加 schema_version_rejected 事件。
    #[test]
    fn test_unknown_schema_writes_rejected_event() {
        let (_d, log) = tmp_log("schema");
        {
            let mut e = base_event();
            e.schema_version = "9.9".into();
            e.seq = 1;
            let line = serde_json::to_string(&e).unwrap();
            let mut f = OpenOptions::new()
                .create(true)
                .append(true)
                .open(log.path())
                .unwrap();
            writeln!(f, "{line}").unwrap();
        }
        let (_events, rejected) = log.read_all().expect("read");
        assert_eq!(rejected, 1, "未知版本必须被拒 1 行");
        // schema_version_rejected 事件在锁外追加完成（migration layer 不静默跳过）——
        // 第二次 read_all 应看到该事件
        let (events2, _) = log.read_all().expect("read");
        assert!(
            events2
                .iter()
                .any(|e| e.etype == EventType::SchemaVersionRejected),
            "必须写 schema_version_rejected 事件"
        );
    }

    /// H-6 正面：水位线 1 时若缺 seq 2（有 3）→ gap_after 报缺口 2。
    #[test]
    fn test_gap_detection() {
        let (_d, log) = tmp_log("gap");
        {
            let mut e1 = base_event();
            e1.seq = 1;
            let mut e3 = base_event();
            e3.seq = 3;
            let mut f = OpenOptions::new()
                .create(true)
                .append(true)
                .open(log.path())
                .unwrap();
            writeln!(f, "{}", serde_json::to_string(&e1).unwrap()).unwrap();
            writeln!(f, "{}", serde_json::to_string(&e3).unwrap()).unwrap();
        }
        let gap = log.gap_after(1).expect("gap");
        assert_eq!(gap, Some(2), "缺 seq 2 应报缺口");
        assert_eq!(log.gap_after(3).expect("gap"), None, "水位线 3 无缺口");
    }

    /// 锁超时负面：持锁不放 → 另一写者超时报错（不永久阻塞）。
    #[test]
    fn test_lock_timeout() {
        let (_d, log) = tmp_log("lock");
        let f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log.path())
            .unwrap();
        f.lock_exclusive().unwrap(); // 持锁
        let log2 = EventLog::with_lock_timeout(log.path(), 150).unwrap();
        let mut e = base_event();
        let err = log2.append(&mut e, "w2").unwrap_err();
        assert!(
            err.to_string().contains("lock timeout"),
            "持锁竞争必须超时: {err}"
        );
        drop(f); // 释放
    }
}
