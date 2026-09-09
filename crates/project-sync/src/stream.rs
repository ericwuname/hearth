//! EVENT_STREAM 逻辑/物理分层（P2-01，§7.6）——对外逻辑流，物理按月切片多文件。
//! 消费游标 last_processed_seq 跨切片单调递增；切片损坏/改名 → 显式报错（不静默读空）。

use crate::event::Event;
use crate::event_log::EventLog;
use crate::Result;
use std::path::{Path, PathBuf};

/// 逻辑 EVENT_STREAM：物理 = events/EVENT_LOG.YYYY-MM.jsonl（当月 = EVENT_LOG.jsonl 别名）。
/// 切片策略变更只改本层（物理），读写代码不感知。
pub struct EventStream {
    events_dir: PathBuf,
}

impl EventStream {
    pub fn open(events_dir: impl AsRef<Path>) -> Result<Self> {
        let dir = events_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&dir)?;
        Ok(Self { events_dir: dir })
    }

    /// 当月物理文件（P2-01：EVENT_LOG.jsonl = 当前月切片）。
    fn month_file(&self) -> PathBuf {
        let month = chrono::Utc::now().format("%Y-%m").to_string();
        self.events_dir.join(format!("EVENT_LOG.{month}.jsonl"))
    }

    fn legacy_file(&self) -> PathBuf {
        self.events_dir.join("EVENT_LOG.jsonl")
    }

    /// 打开当月切片（存在旧 EVENT_LOG.jsonl 则视为历史别名——P2-01 迁移：优先新切片名）。
    fn current_log(&self) -> Result<EventLog> {
        let month = self.month_file();
        if month.exists() {
            return EventLog::open(&month);
        }
        // 旧别名存在 → 用旧文件（读历史）；否则新切片
        let legacy = self.legacy_file();
        if legacy.exists() {
            EventLog::open(&legacy)
        } else {
            EventLog::open(&month)
        }
    }

    /// 追加事件到逻辑流（落到当月物理切片）。
    pub fn append(&self, evt: &mut Event, actor: &str) -> Result<(u64, String)> {
        self.current_log()?.append(evt, actor)
    }

    /// 跨切片消费：读全部物理切片（当月 + 历史归档 EVENT_LOG.YYYY-MM.jsonl），
    /// 按 seq 合并排序——游标跨切片单调。物理切片缺失/损坏 → 显式 Err（P2-01 负面）。
    pub fn read_stream(&self) -> Result<Vec<Event>> {
        let mut all: Vec<Event> = Vec::new();
        let mut found_any = false;
        let rd = std::fs::read_dir(&self.events_dir)?;
        for entry in rd.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("EVENT_LOG") && name.ends_with(".jsonl") {
                found_any = true;
                let (events, _) = EventLog::open(entry.path())?.read_all()?;
                all.extend(events);
            }
        }
        if !found_any {
            return Err("EVENT_STREAM: 无物理切片（目录为空/全被删）——不静默读空".into());
        }
        all.sort_by_key(|e| e.seq);
        Ok(all)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{EventType, SCHEMA_VERSION};

    fn base() -> Event {
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
            summary: Some("x".into()),
            status: None,
            prev_hash: None,
            provided_granularity: None,
        }
    }

    /// P2-01 正面：追加 → 读回（seq 单调）。
    #[test]
    fn test_stream_append_read() {
        let dir = tempfile::tempdir().unwrap();
        let s = EventStream::open(dir.path()).unwrap();
        let mut e = base();
        let (seq, _) = s.append(&mut e, "w1").unwrap();
        assert_eq!(seq, 1);
        let all = s.read_stream().unwrap();
        assert_eq!(all.len(), 1);
    }

    /// P2-01 负面：物理切片全删 → read_stream 显式报错（不静默读空）。
    #[test]
    fn test_stream_missing_slice_errors() {
        let dir = tempfile::tempdir().unwrap();
        let s = EventStream::open(dir.path()).unwrap();
        let mut e = base();
        let _ = s.append(&mut e, "w1").unwrap();
        // 删掉全部切片
        for entry in std::fs::read_dir(dir.path()).unwrap().flatten() {
            let _ = std::fs::remove_file(entry.path());
        }
        let err = s.read_stream().unwrap_err();
        assert!(
            err.to_string().contains("不静默读空"),
            "切片缺失必须显式报错: {err}"
        );
    }
}
