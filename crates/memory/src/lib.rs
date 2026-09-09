#![allow(clippy::all, unused_mut)]
//! memory: Session persistence for agent sessions + 6C Civilization store.

use agent_types::{CivEntry, WorkNode};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
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

        let content = std::fs::read_to_string(&path)?;
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
}

// ── 6C v6.0: Civilization Store ──

/// Append-only JSONL store for civilization line entries.
pub struct CivilizationStore {
    path: PathBuf,
    entries: Mutex<Vec<CivEntry>>,
}

impl CivilizationStore {
    pub fn new(dir: &Path, filename: &str) -> Result<Self> {
        std::fs::create_dir_all(dir)?;
        let path = dir.join(filename);
        let mut entries = Vec::new();
        if path.exists() {
            let f = std::fs::File::open(&path)?;
            for line in BufReader::new(f).lines() {
                if let Ok(line) = line {
                    if let Ok(entry) = serde_json::from_str::<CivEntry>(&line) {
                        entries.push(entry);
                    }
                }
            }
        }
        if entries.len() > 1000 {
            entries = entries.split_off(entries.len() - 1000);
        }
        Ok(Self {
            path,
            entries: Mutex::new(entries),
        })
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
        let mut nodes = Vec::new();
        if path.exists() {
            let f = std::fs::File::open(&path)?;
            for line in BufReader::new(f).lines() {
                if let Ok(line) = line {
                    if let Ok(node) = serde_json::from_str::<WorkNode>(&line) {
                        nodes.push(node);
                    }
                }
            }
        }
        Ok(Self {
            path,
            nodes: Mutex::new(nodes),
        })
    }

    fn flush(&self) -> Result<()> {
        let nodes = self.nodes.lock().unwrap();
        let mut f = std::fs::File::create(&self.path)?;
        for n in nodes.iter() {
            writeln!(f, "{}", serde_json::to_string(n)?)?;
        }
        Ok(())
    }

    pub fn add(&self, mut node: WorkNode) -> Result<()> {
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
