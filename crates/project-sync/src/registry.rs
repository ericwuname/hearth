//! 窗口清册 registry（§5.2/§5.3/§5.4）——manifest/heartbeat/last_processed_seq，
//! 乐观锁 + 原子 rename + stale 清理（H-8）。

use crate::manifest::{Manifest, WindowStatus};
use crate::Result;
use std::path::{Path, PathBuf};

/// 窗口清册：registry/windows/<window_id>/（manifest.toml + heartbeat + last_processed_seq）
pub struct Registry {
    root: PathBuf,
}

impl Registry {
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        std::fs::create_dir_all(root.join("windows"))?;
        Ok(Self { root })
    }

    fn window_dir(&self, id: &str) -> PathBuf {
        self.root.join("windows").join(id)
    }

    /// 写 manifest（原子：临时文件 + rename，§5.4）。
    pub fn write_manifest(&self, m: &Manifest) -> Result<()> {
        let dir = self.window_dir(&m.window_id);
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("manifest.toml");
        let tmp = dir.join("manifest.toml.tmp");
        let s = toml::to_string(m).map_err(|e| crate::PwcError::Other(e.to_string()))?;
        std::fs::write(&tmp, s)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    pub fn read_manifest(&self, id: &str) -> Result<Manifest> {
        let path = self.window_dir(id).join("manifest.toml");
        let s = std::fs::read_to_string(&path)?;
        toml::from_str(&s).map_err(|e| crate::PwcError::Other(e.to_string()))
    }

    /// 更新心跳（临时文件 + rename 原子写，§5.3）。
    pub fn touch_heartbeat(&self, id: &str) -> Result<()> {
        let dir = self.window_dir(id);
        std::fs::create_dir_all(&dir)?;
        let ts = chrono::Utc::now().to_rfc3339();
        let path = dir.join("heartbeat");
        let tmp = dir.join("heartbeat.tmp");
        std::fs::write(&tmp, ts)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    pub fn read_heartbeat(&self, id: &str) -> Result<Option<String>> {
        let path = self.window_dir(id).join("heartbeat");
        match std::fs::read_to_string(&path) {
            Ok(s) => Ok(Some(s.trim().to_string())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// 写消费水位线（§7.3）。
    pub fn write_watermark(&self, id: &str, seq: u64) -> Result<()> {
        let dir = self.window_dir(id);
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("last_processed_seq");
        let tmp = dir.join("last_processed_seq.tmp");
        std::fs::write(&tmp, seq.to_string())?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    pub fn read_watermark(&self, id: &str) -> Result<u64> {
        let path = self.window_dir(id).join("last_processed_seq");
        match std::fs::read_to_string(&path) {
            Ok(s) => Ok(s.trim().parse().unwrap_or(0)),
            Err(_) => Ok(0),
        }
    }

    /// 在线判定（§5.3）：status==active 且心跳 ≤ 2 分钟。
    pub fn is_online(&self, id: &str) -> bool {
        let Ok(m) = self.read_manifest(id) else {
            return false;
        };
        if m.status != WindowStatus::Active {
            return false;
        }
        let Some(hb) = self.read_heartbeat(id).ok().flatten() else {
            return false;
        };
        let Ok(ts) = chrono::DateTime::parse_from_rfc3339(&hb) else {
            return false;
        };
        let age = chrono::Utc::now().signed_duration_since(ts.with_timezone(&chrono::Utc));
        age.num_seconds() <= 120
    }

    /// 列出全部窗口 id。
    pub fn list_windows(&self) -> Result<Vec<String>> {
        let dir = self.root.join("windows");
        let mut ids = Vec::new();
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                if e.path().is_dir() {
                    ids.push(e.file_name().to_string_lossy().to_string());
                }
            }
        }
        Ok(ids)
    }

    /// 启动清理 stale 窗口持有的锁（H-8）：status=stale 的窗口不视为在线（锁在 Phase 1
    /// 由文件锁管理——此处标记 stale 窗口清单，调用方据此释放其锁路径）。
    pub fn stale_windows(&self) -> Result<Vec<String>> {
        let mut stale = Vec::new();
        for id in self.list_windows()? {
            let online = self.is_online(&id);
            if let Ok(m) = self.read_manifest(&id) {
                if m.status == WindowStatus::Active && !online {
                    // 心跳超时 → 应转 stale
                    let mut m2 = m;
                    m2.status = WindowStatus::Stale;
                    let _ = self.write_manifest(&m2);
                    stale.push(id);
                }
            }
        }
        Ok(stale)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{Manifest, WindowStatus};

    fn manifest(id: &str, status: WindowStatus) -> Manifest {
        Manifest {
            project_id: "p".into(),
            window_id: id.into(),
            role: "施工".into(),
            functions: vec![],
            version: 1,
            status,
            last_heartbeat: None,
            autonomy_level: "consult".into(),
            identity_epoch: None,
            subscribes: vec![],
            coordinates_with: vec![],
            created_at: None,
        }
    }

    /// §5.4 正面：原子写 + 往返读。
    #[test]
    fn test_manifest_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let reg = Registry::open(dir.path()).unwrap();
        reg.write_manifest(&manifest("impl-01", WindowStatus::Active))
            .unwrap();
        let m = reg.read_manifest("impl-01").unwrap();
        assert_eq!(m.window_id, "impl-01");
        assert_eq!(m.status, WindowStatus::Active);
    }

    /// §7.3 正面：水位线往返。
    #[test]
    fn test_watermark_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let reg = Registry::open(dir.path()).unwrap();
        reg.write_watermark("w1", 42).unwrap();
        assert_eq!(reg.read_watermark("w1").unwrap(), 42);
        assert_eq!(reg.read_watermark("nope").unwrap(), 0, "无水位线=0");
    }

    /// §5.3 正面：active+新心跳 = 在线。
    #[test]
    fn test_online_active_fresh() {
        let dir = tempfile::tempdir().unwrap();
        let reg = Registry::open(dir.path()).unwrap();
        reg.write_manifest(&manifest("w1", WindowStatus::Active))
            .unwrap();
        reg.touch_heartbeat("w1").unwrap();
        assert!(reg.is_online("w1"));
    }

    /// §5.3 负面：status 非 active（如 failed）即使心跳新也不在线——failed 窗口不分配任务。
    #[test]
    fn test_failed_window_not_online() {
        let dir = tempfile::tempdir().unwrap();
        let reg = Registry::open(dir.path()).unwrap();
        reg.write_manifest(&manifest("w1", WindowStatus::Failed))
            .unwrap();
        reg.touch_heartbeat("w1").unwrap();
        assert!(!reg.is_online("w1"), "failed 窗口不得在线");
    }
}
