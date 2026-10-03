//! memory: Session persistence for agent sessions + 6C Civilization store.

use agent_types::{CivEntry, WorkNode};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// A stored session record containing event history and state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub session_id: String,
    pub provider_name: String,
    pub model: String,
    pub goal: String,
    pub events: Vec<StoredEvent>,
    pub created_at: String,
}

/// A single event in the session history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredEvent {
    pub seq: u64,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub timestamp: String,
}

/// Trait for session persistence backends.
#[async_trait]
pub trait MemoryStore: Send + Sync {
    /// Save a session record.
    async fn save_session(&self, record: &SessionRecord) -> Result<()>;

    /// Load a session record by ID.
    async fn load_session(&self, session_id: &str) -> Result<Option<SessionRecord>>;

    /// List all stored session IDs.
    async fn list_sessions(&self) -> Result<Vec<String>>;

    /// Delete a session record.
    async fn delete_session(&self, session_id: &str) -> Result<()>;

    /// Append events to an existing session.
    async fn append_events(&self, session_id: &str, events: &[StoredEvent]) -> Result<()>;

    /// Check if the store is operational.
    fn is_available(&self) -> bool;
}

/// JSONL file-based memory store.
///
/// Each session is stored as `<session_id>.jsonl` in a directory.
/// Events are appended as individual JSON lines.
///
/// v1.2 hardening (global-audit MEM-1~4):
/// - MEM-1: `save_session` writes to a temp file then renames (atomic on the
///   same filesystem) so a crash mid-write can't truncate an existing record.
/// - MEM-2: all writes are serialized through an internal async Mutex, so
///   concurrent `save_session`/`append_events` on the same store can't
///   interleave or clobber each other.
/// - MEM-3: a per-session file size cap prevents unbounded disk growth.
/// - MEM-4: `load_session` tolerates corrupt lines (warn + skip) instead of
///   discarding the whole record on the first bad line.
pub struct JsonlMemoryStore {
    dir: PathBuf,
    /// MEM-2: serializes all mutating file operations.
    write_lock: tokio::sync::Mutex<()>,
}

/// MEM-3: refuse to grow a single session file beyond this size (64 MiB).
const MAX_SESSION_FILE_BYTES: u64 = 64 * 1024 * 1024;

impl JsonlMemoryStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        let dir = dir.into();
        std::fs::create_dir_all(&dir).ok();
        Self {
            dir,
            write_lock: tokio::sync::Mutex::new(()),
        }
    }

    fn session_path(&self, session_id: &str) -> PathBuf {
        self.dir.join(format!("{}.jsonl", session_id))
    }

    /// D-51：**有界读入**单个会话文件，返回 `(内容, 是否被截断)`。
    ///
    /// 此前 `load_session` 用 `std::fs::read_to_string` 把**整份文件**读进内存
    /// （无界读入第 6 落点；写侧 MEM-3 上限只是弱化、并非强制）。现走共享原语
    /// [`bounded_io::read_file_text_capped`]——排空到 EOF 但只保留前 `cap` 字节，
    /// 截断点退到合法 UTF-8 边界；截断**必须留痕**（不静默）。
    ///
    /// `cap` 显式传入以便单测用一个小上限验证截断路径（无需造 64 MiB 文件）。
    async fn read_session_text(&self, session_id: &str, cap: u64) -> Result<(String, bool)> {
        let path = self.session_path(session_id);
        let (text, truncated) = bounded_io::read_file_text_capped(&path, cap).await?;
        if truncated {
            tracing::warn!(
                "load_session({session_id}): session 文件超过 {cap} 字节上限，已截断读取——\
                 被截断的尾部事件不会加载（写侧 MEM-3 上限应已阻止此情形）"
            );
        }
        Ok((text, truncated))
    }
}

#[async_trait]
impl MemoryStore for JsonlMemoryStore {
    async fn save_session(&self, record: &SessionRecord) -> Result<()> {
        // MEM-2: serialize concurrent writers.
        let _guard = self.write_lock.lock().await;

        let path = self.session_path(&record.session_id);
        // MEM-1: write to a temp file in the same directory, then rename.
        // `std::fs::rename` replaces the destination atomically on both Unix
        // and Windows when source and destination share a filesystem.
        let tmp_path = self.dir.join(format!("{}.jsonl.tmp", record.session_id));

        {
            let file = std::fs::File::create(&tmp_path)?;
            let mut writer = std::io::BufWriter::new(file);
            use std::io::Write;

            // Write session header
            let header = serde_json::json!({
                "type": "session_meta",
                "session_id": record.session_id,
                "provider_name": record.provider_name,
                "model": record.model,
                "goal": record.goal,
                "created_at": record.created_at,
            });
            writeln!(writer, "{}", serde_json::to_string(&header)?)?;

            // Write events
            for event in &record.events {
                let line = serde_json::json!({
                    "type": "event",
                    "seq": event.seq,
                    "event_type": event.event_type,
                    "payload": event.payload,
                    "timestamp": event.timestamp,
                });
                writeln!(writer, "{}", serde_json::to_string(&line)?)?;
            }

            writer.flush()?;
        }

        // MEM-3: refuse to install an oversized record.
        let tmp_size = std::fs::metadata(&tmp_path).map(|m| m.len()).unwrap_or(0);
        if tmp_size > MAX_SESSION_FILE_BYTES {
            let _ = std::fs::remove_file(&tmp_path);
            anyhow::bail!(
                "session record {} exceeds size cap ({} > {} bytes)",
                record.session_id,
                tmp_size,
                MAX_SESSION_FILE_BYTES
            );
        }

        std::fs::rename(&tmp_path, &path)?;
        Ok(())
    }

    async fn load_session(&self, session_id: &str) -> Result<Option<SessionRecord>> {
        let path = self.session_path(session_id);
        if !path.exists() {
            return Ok(None);
        }

        // D-51：有界读入（cap 与写侧 MEM-3 上限同值——写不进来的就读不出来）。
        let (content, _truncated) = self
            .read_session_text(session_id, MAX_SESSION_FILE_BYTES)
            .await?;
        let mut events = Vec::new();
        let mut meta: Option<(String, String, String, String, String)> = None;

        for (lineno, line) in content.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            // MEM-4: tolerate corrupt lines — warn and skip instead of
            // discarding the entire record via `?`.
            let val: serde_json::Value = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!(
                        "load_session({}): skipping corrupt line {}: {e}",
                        session_id,
                        lineno + 1
                    );
                    continue;
                }
            };
            match val["type"].as_str() {
                Some("session_meta") => {
                    // MEM-5: warn when required fields are missing instead of
                    // silently degrading to "".
                    if val["session_id"].as_str().is_none() {
                        tracing::warn!(
                            "load_session({}): session_meta line {} missing session_id",
                            session_id,
                            lineno + 1
                        );
                    }
                    meta = Some((
                        val["session_id"].as_str().unwrap_or("").to_string(),
                        val["provider_name"].as_str().unwrap_or("").to_string(),
                        val["model"].as_str().unwrap_or("").to_string(),
                        val["goal"].as_str().unwrap_or("").to_string(),
                        val["created_at"].as_str().unwrap_or("").to_string(),
                    ));
                }
                Some("event") => {
                    events.push(StoredEvent {
                        seq: val["seq"].as_u64().unwrap_or(0),
                        event_type: val["event_type"].as_str().unwrap_or("").to_string(),
                        payload: val["payload"].clone(),
                        timestamp: val["timestamp"].as_str().unwrap_or("").to_string(),
                    });
                }
                _ => {}
            }
        }

        match meta {
            Some((session_id, provider_name, model, goal, created_at)) => Ok(Some(SessionRecord {
                session_id,
                provider_name,
                model,
                goal,
                events,
                created_at,
            })),
            None => Ok(None),
        }
    }

    async fn list_sessions(&self) -> Result<Vec<String>> {
        // P1-2 (audit-fix): 不再吞 I/O 错误——目录不可读必须向上传播，
        // 否则 /readyz 探活永远 200（虚假绿）。
        let mut ids = Vec::new();
        for entry in std::fs::read_dir(&self.dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(id) = name.strip_suffix(".jsonl") {
                ids.push(id.to_string());
            }
        }
        Ok(ids)
    }

    async fn delete_session(&self, session_id: &str) -> Result<()> {
        let path = self.session_path(session_id);
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    async fn append_events(&self, session_id: &str, events: &[StoredEvent]) -> Result<()> {
        // MEM-2: serialize concurrent writers (same lock as save_session).
        let _guard = self.write_lock.lock().await;

        let path = self.session_path(session_id);

        // MEM-3: refuse to grow beyond the size cap.
        let current_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        if current_size > MAX_SESSION_FILE_BYTES {
            anyhow::bail!(
                "session file {} exceeds size cap ({} > {} bytes); refusing append",
                session_id,
                current_size,
                MAX_SESSION_FILE_BYTES
            );
        }

        let file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path)?;
        let mut writer = std::io::BufWriter::new(file);
        use std::io::Write;

        for event in events {
            let line = serde_json::json!({
                "type": "event",
                "seq": event.seq,
                "event_type": event.event_type,
                "payload": event.payload,
                "timestamp": event.timestamp,
            });
            writeln!(writer, "{}", serde_json::to_string(&line)?)?;
        }

        writer.flush()?;
        Ok(())
    }

    fn is_available(&self) -> bool {
        self.dir.exists()
    }
}

// ── 6C v6.0: Civilization Store ──

/// D-143（2026-10-04, traecode）：`CivilizationStore` / `WorkLineStore` 持有的
/// **运行时数据文件**（`<uid>/{civ,workline}.jsonl`）的**读入**字节上限。
///
/// 取 64 MiB：与同 crate 的 [`MAX_SESSION_FILE_BYTES`]（会话文件）同量级——
/// 三者是同一族"应用自有的只增 JSONL 运行时数据文件"。远超正常规模
/// （civ 只留 1000 条；workline 是个人任务板），只把"失控增长"从 OOM 退化为
/// "截断 + 明确留痕"。
const MAX_STORE_FILE_BYTES: u64 = 64 * 1024 * 1024;

/// Append-only JSONL store for civilization line entries.
pub struct CivilizationStore {
    path: PathBuf,
    entries: Mutex<Vec<CivEntry>>,
}

impl CivilizationStore {
    pub fn new(dir: &Path, filename: &str) -> Result<Self> {
        std::fs::create_dir_all(dir)?;
        let path = dir.join(filename);
        let mut entries = Self::read_entries(&path, MAX_STORE_FILE_BYTES)?;
        if entries.len() > 1000 {
            entries = entries.split_off(entries.len() - 1000);
        }
        Ok(Self {
            path,
            entries: Mutex::new(entries),
        })
    }

    /// D-143：**有界读入**文明线 JSONL，返回解析成功的条目。
    ///
    /// 此前用 `BufReader::new(f).lines()` 把**整份文件**逐行读进 `Vec`——文件随
    /// `append` **只增**且写侧无上限，读侧因而无界（无界读入新落点；与 D-51 session /
    /// D-70 experience / D-86 archive 同族）。现走共享原语
    /// [`bounded_io::read_file_text_capped_std`]：只保留前 `cap` 字节，截断**留痕**。
    ///
    /// `cap` 显式传入以便单测用小上限验证截断路径（无需造 64 MiB 文件，仿 D-51）。
    /// 截断时只读到**前一段**——较新条目可能落在上限之外而丢失（写侧无上限，此为安全阀；
    /// 正常规模远不会触及）。
    fn read_entries(path: &Path, cap: u64) -> Result<Vec<CivEntry>> {
        if !path.exists() {
            return Ok(Vec::new());
        }
        let (text, truncated) = bounded_io::read_file_text_capped_std(path, cap)?;
        if truncated {
            tracing::warn!(
                path = %path.display(),
                cap,
                "civilization store 文件超过读取上限，仅基于前一段解析——\
                 较新条目可能未加载（写侧 append 无上限，此为安全阀）"
            );
        }
        let mut entries = Vec::new();
        for line in text.lines() {
            if let Ok(entry) = serde_json::from_str::<CivEntry>(line) {
                entries.push(entry);
            }
        }
        Ok(entries)
    }

    pub fn append(&self, entry: CivEntry) -> Result<()> {
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        let line = serde_json::to_string(&entry)?;
        writeln!(f, "{line}")?;
        self.entries.lock().unwrap().push(entry);
        Ok(())
    }

    pub fn recent(&self, limit: usize) -> Vec<CivEntry> {
        self.entries
            .lock()
            .unwrap()
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<CivEntry> {
        let q = query.to_lowercase();
        self.entries
            .lock()
            .unwrap()
            .iter()
            .rev()
            .filter(|e| {
                e.content.to_lowercase().contains(&q)
                    || e.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .take(limit)
            .cloned()
            .collect()
    }
}

// ── 6D v6.0: Work Line Store ──

/// JSONL-backed task/work board store.
pub struct WorkLineStore {
    path: PathBuf,
    nodes: Mutex<Vec<WorkNode>>,
}

impl WorkLineStore {
    pub fn new(dir: &Path, filename: &str) -> Result<Self> {
        std::fs::create_dir_all(dir)?;
        let path = dir.join(filename);
        let nodes = Self::read_nodes(&path, MAX_STORE_FILE_BYTES)?;
        Ok(Self {
            path,
            nodes: Mutex::new(nodes),
        })
    }

    /// D-143：**有界读入**工作线 JSONL（同 [`CivilizationStore::read_entries`] 口径）。
    ///
    /// 此前用 `BufReader::new(f).lines()` 读满整份文件且**无任何上限**（连"只留最后
    /// N 条"的弱化都没有）。现走 [`bounded_io::read_file_text_capped_std`]，截断留痕。
    fn read_nodes(path: &Path, cap: u64) -> Result<Vec<WorkNode>> {
        if !path.exists() {
            return Ok(Vec::new());
        }
        let (text, truncated) = bounded_io::read_file_text_capped_std(path, cap)?;
        if truncated {
            tracing::warn!(
                path = %path.display(),
                cap,
                "workline store 文件超过读取上限，仅基于前一段解析——\
                 较新节点可能未加载（此为安全阀）"
            );
        }
        let mut nodes = Vec::new();
        for line in text.lines() {
            if let Ok(node) = serde_json::from_str::<WorkNode>(line) {
                nodes.push(node);
            }
        }
        Ok(nodes)
    }

    fn flush(&self) -> Result<()> {
        let nodes = self.nodes.lock().unwrap();
        let mut f = std::fs::File::create(&self.path)?;
        for n in nodes.iter() {
            writeln!(f, "{}", serde_json::to_string(n)?)?;
        }
        Ok(())
    }

    pub fn add(&self, node: WorkNode) -> Result<()> {
        self.nodes.lock().unwrap().push(node.clone());
        self.flush()?;
        Ok(())
    }

    pub fn list(&self, category: Option<&str>) -> Vec<WorkNode> {
        let nodes = self.nodes.lock().unwrap();
        nodes
            .iter()
            .filter(|n| match category {
                Some("completed") => n.status == agent_types::WorkStatus::Completed,
                Some("pending") => n.status == agent_types::WorkStatus::Pending,
                _ => true,
            })
            .cloned()
            .collect()
    }

    pub fn update(
        &self,
        id: &str,
        progress: f32,
        status: Option<agent_types::WorkStatus>,
    ) -> Result<()> {
        let mut nodes = self.nodes.lock().unwrap();
        if let Some(n) = nodes.iter_mut().find(|n| n.id == id) {
            n.progress = progress;
            if let Some(s) = status {
                if s == agent_types::WorkStatus::Completed {
                    n.completed_at = Some(chrono::Utc::now().to_rfc3339());
                }
                n.status = s;
            }
            n.updated_at = chrono::Utc::now().to_rfc3339();
        }
        drop(nodes);
        self.flush()
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        let mut nodes = self.nodes.lock().unwrap();
        nodes.retain(|n| n.id != id);
        drop(nodes);
        self.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_p5_memory_save_and_load() {
        let tmp = tempfile::tempdir().unwrap();
        let store = JsonlMemoryStore::new(tmp.path());
        assert!(store.is_available());

        let record = SessionRecord {
            session_id: "test-session-1".into(),
            provider_name: "openai".into(),
            model: "gpt-4o".into(),
            goal: "test goal".into(),
            events: vec![
                StoredEvent {
                    seq: 0,
                    event_type: "phase".into(),
                    payload: serde_json::json!({"phase": "Init"}),
                    timestamp: "2025-01-01T00:00:00Z".into(),
                },
                StoredEvent {
                    seq: 1,
                    event_type: "done".into(),
                    payload: serde_json::json!({"ok": true}),
                    timestamp: "2025-01-01T00:00:01Z".into(),
                },
            ],
            created_at: "2025-01-01T00:00:00Z".into(),
        };

        store.save_session(&record).await.unwrap();

        let loaded = store.load_session("test-session-1").await.unwrap().unwrap();
        assert_eq!(loaded.session_id, "test-session-1");
        assert_eq!(loaded.provider_name, "openai");
        assert_eq!(loaded.events.len(), 2);
        assert_eq!(loaded.events[0].event_type, "phase");
        assert_eq!(loaded.events[1].event_type, "done");
    }

    #[tokio::test]
    async fn test_p5_memory_list_and_delete() {
        let tmp = tempfile::tempdir().unwrap();
        let store = JsonlMemoryStore::new(tmp.path());

        let record = SessionRecord {
            session_id: "s1".into(),
            provider_name: "o".into(),
            model: "m".into(),
            goal: "g".into(),
            events: vec![],
            created_at: "t".into(),
        };
        store.save_session(&record).await.unwrap();

        let list = store.list_sessions().await.unwrap();
        assert!(list.contains(&"s1".to_string()));

        store.delete_session("s1").await.unwrap();
        let list2 = store.list_sessions().await.unwrap();
        assert!(!list2.contains(&"s1".to_string()));
    }

    #[tokio::test]
    async fn test_p5_memory_load_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let store = JsonlMemoryStore::new(tmp.path());
        let result = store.load_session("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_p5_memory_append_events() {
        let tmp = tempfile::tempdir().unwrap();
        let store = JsonlMemoryStore::new(tmp.path());

        let record = SessionRecord {
            session_id: "s2".into(),
            provider_name: "o".into(),
            model: "m".into(),
            goal: "g".into(),
            events: vec![],
            created_at: "t".into(),
        };
        store.save_session(&record).await.unwrap();

        store
            .append_events(
                "s2",
                &[StoredEvent {
                    seq: 10,
                    event_type: "token".into(),
                    payload: serde_json::json!({"delta": "hello"}),
                    timestamp: "t2".into(),
                }],
            )
            .await
            .unwrap();

        let loaded = store.load_session("s2").await.unwrap().unwrap();
        assert_eq!(loaded.events.len(), 1);
        assert_eq!(loaded.events[0].event_type, "token");
    }

    /// MEM-4 regression: a corrupt line in the middle of the file must not
    /// discard the whole record — valid lines before/after still load.
    #[tokio::test]
    async fn test_v12_memory_corrupt_line_tolerated() {
        let tmp = tempfile::tempdir().unwrap();
        let store = JsonlMemoryStore::new(tmp.path());

        let record = SessionRecord {
            session_id: "s3".into(),
            provider_name: "o".into(),
            model: "m".into(),
            goal: "g".into(),
            events: vec![StoredEvent {
                seq: 0,
                event_type: "phase".into(),
                payload: serde_json::json!({"phase": "Init"}),
                timestamp: "t0".into(),
            }],
            created_at: "t".into(),
        };
        store.save_session(&record).await.unwrap();

        // Inject a corrupt (non-JSON) line, then a valid event after it.
        {
            use std::io::Write;
            let path = tmp.path().join("s3.jsonl");
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .unwrap();
            writeln!(f, "{{ this is not valid json").unwrap();
            writeln!(
                f,
                "{}",
                serde_json::json!({
                    "type": "event", "seq": 1, "event_type": "done",
                    "payload": {"ok": true}, "timestamp": "t1",
                })
            )
            .unwrap();
        }

        let loaded = store.load_session("s3").await.unwrap().unwrap();
        // Corrupt line skipped; both valid events survive.
        assert_eq!(loaded.events.len(), 2);
        assert_eq!(loaded.events[0].event_type, "phase");
        assert_eq!(loaded.events[1].event_type, "done");
    }

    /// 先红后绿（D-51）：会话文件读取必须**有界**。修复前 `load_session` 用
    /// `std::fs::read_to_string` 把整份文件读进内存——文件多大就吃多少内存。
    #[tokio::test]
    async fn test_d51_session_read_is_bounded() {
        let tmp = tempfile::tempdir().unwrap();
        let store = JsonlMemoryStore::new(tmp.path());

        // 远大于测试上限的文件（不必造 64 MiB——用小 cap 验证同一条路径）。
        std::fs::write(tmp.path().join("big.jsonl"), "x".repeat(4096)).unwrap();
        let (text, truncated) = store.read_session_text("big", 1024).await.unwrap();
        assert_eq!(
            text.len(),
            1024,
            "保留量必须被 cap 钳住（修复前等于整份文件长度）"
        );
        assert!(truncated, "超限必须报告截断（不静默）");

        // 未超限：原样返回且不标截断。
        std::fs::write(tmp.path().join("small.jsonl"), "hello").unwrap();
        let (text, truncated) = store.read_session_text("small", 1024).await.unwrap();
        assert_eq!(text, "hello");
        assert!(!truncated);
    }

    /// 先红后绿（D-143）：`CivilizationStore` / `WorkLineStore` 的运行时数据文件
    /// 读入必须**有界**。修复前两处 `new` 用 `BufReader::new(f).lines()` 把整份
    /// 文件逐行读进 `Vec`——文件多大就吃多少内存（写侧只增、无上限）。
    #[test]
    fn test_d143_civ_workline_read_is_bounded() {
        let tmp = tempfile::tempdir().unwrap();
        let count = 200usize;

        // ── civ：每行一条合法条目。cap 取文件的 1/4 → 只能解析到前一段。 ──
        let mk_civ = |i: usize| CivEntry {
            id: format!("c{i}"),
            author: agent_types::CivAuthor {
                provider_model: "m".into(),
                session_id: "s".into(),
                bridge_id: None,
            },
            content: format!("content-{i}-{}", "x".repeat(64)),
            category: agent_types::CivCategory::Insight,
            context: None,
            created_at: "t".into(),
            tags: vec![],
        };
        let civ_path = tmp.path().join("civ.jsonl");
        let mut body = String::new();
        for i in 0..count {
            body.push_str(&serde_json::to_string(&mk_civ(i)).unwrap());
            body.push('\n');
        }
        std::fs::write(&civ_path, &body).unwrap();
        let total = body.len() as u64;
        let cap = total / 4;
        let entries = CivilizationStore::read_entries(&civ_path, cap).unwrap();
        assert!(
            !entries.is_empty(),
            "前一段内仍有合法条目（有界读不得误伤）"
        );
        assert!(
            (entries.len() as u64) < count as u64,
            "必须**有界读入**：cap={cap} 字节 < 文件 {total} 字节 ⇒ 解析到的条目数必须\
             少于全部 {count} 条（修复前会读满整份、得到 {count} 条）"
        );

        // 未超限：完整解析（不误伤）。
        let small = tmp.path().join("small_civ.jsonl");
        std::fs::write(
            &small,
            format!("{}\n", serde_json::to_string(&mk_civ(0)).unwrap()),
        )
        .unwrap();
        let entries = CivilizationStore::read_entries(&small, 1024 * 1024).unwrap();
        assert_eq!(entries.len(), 1, "未超限必须完整解析");

        // ── workline：同一路径（read_nodes），修复前连"只留 N 条"的弱化都没有。 ──
        let mk_node = |i: usize| WorkNode {
            id: format!("w{i}"),
            parent_id: None,
            description: format!("node-{i}-{}", "y".repeat(64)),
            status: agent_types::WorkStatus::Pending,
            progress: 0.0,
            category: agent_types::WorkCategory::ShortTerm,
            assignee: None,
            deps: vec![],
            created_at: "t".into(),
            updated_at: "t".into(),
            completed_at: None,
            notes: vec![],
        };
        let wl_path = tmp.path().join("workline.jsonl");
        let mut body = String::new();
        for i in 0..count {
            body.push_str(&serde_json::to_string(&mk_node(i)).unwrap());
            body.push('\n');
        }
        std::fs::write(&wl_path, &body).unwrap();
        let total = body.len() as u64;
        let cap = total / 4;
        let nodes = WorkLineStore::read_nodes(&wl_path, cap).unwrap();
        assert!(!nodes.is_empty(), "前一段内仍有合法节点（不误伤）");
        assert!(
            (nodes.len() as u64) < count as u64,
            "workline 必须**有界读入**（修复前读满整份、得到 {count} 条）"
        );

        let small = tmp.path().join("small_wl.jsonl");
        std::fs::write(
            &small,
            format!("{}\n", serde_json::to_string(&mk_node(0)).unwrap()),
        )
        .unwrap();
        let nodes = WorkLineStore::read_nodes(&small, 1024 * 1024).unwrap();
        assert_eq!(nodes.len(), 1, "未超限必须完整解析");
    }
}
