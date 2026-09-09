//! 受限文件 API（§7.5，M-1/M-4）——canonicalize + 项目根前缀校验，拒绝符号链接逃逸与越界。
//! 身份防伪（§7.5）：本模块不接收 actor 参数——写路径调用方自行从本机 manifest 取
//! window_id（对外 API 拒显式传入伪造）。

use crate::Result;
use std::path::{Component, Path, PathBuf};

/// 受限文件 API：所有 SSOT 读写经此，路径必须解析到 project_root 内。
pub struct RestrictedFs {
    project_root: PathBuf,
    /// canonicalize 后的项目根（防 symlink 逃逸的比较基准）。
    canonical_root: PathBuf,
}

impl RestrictedFs {
    pub fn new(project_root: impl AsRef<Path>) -> Result<Self> {
        let project_root = project_root.as_ref().to_path_buf();
        let canonical_root = std::fs::canonicalize(&project_root)?;
        Ok(Self {
            project_root,
            canonical_root,
        })
    }

    /// 规范化并校验路径在项目根内（§7.5）。拒绝：绝对越界、`..` 逃逸、symlink 逃逸。
    /// 返回 canonical 后的路径。
    pub fn resolve(&self, rel: impl AsRef<Path>) -> Result<PathBuf> {
        let rel = rel.as_ref();
        // 词法层：拒绝绝对路径与 .. 逃逸（不依赖文件存在）
        if rel.is_absolute() {
            return Err(format!("绝对路径被拒: {}", rel.display()).into());
        }
        for comp in rel.components() {
            if let Component::ParentDir = comp {
                return Err(format!("父目录逃逸被拒: {}", rel.display()).into());
            }
            if let Component::RootDir | Component::Prefix(_) = comp {
                return Err(format!("根/前缀逃逸被拒: {}", rel.display()).into());
            }
        }
        let full = self.project_root.join(rel);
        // 物理层：canonicalize 后必须落在 canonical_root 前缀内（防 symlink 逃逸）
        let canonical = std::fs::canonicalize(&full)
            .map_err(|e| format!("路径解析失败（不存在或不可达）: {} ({e})", full.display()))?;
        if !canonical.starts_with(&self.canonical_root) {
            return Err(format!(
                "symlink 逃逸被拒: {} → {}（根外）",
                rel.display(),
                canonical.display()
            )
            .into());
        }
        Ok(canonical)
    }

    /// 读文件（受限）。
    pub fn read(&self, rel: impl AsRef<Path>) -> Result<Vec<u8>> {
        let p = self.resolve(rel)?;
        Ok(std::fs::read(p)?)
    }

    /// 写文件（受限；父目录必须已存在——resolve 要求 canonicalize 成功）。
    pub fn write(&self, rel: impl AsRef<Path>, content: &[u8]) -> Result<()> {
        let p = self.resolve(rel)?;
        Ok(std::fs::write(p, content)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub/file.txt"), b"x").unwrap();
        dir
    }

    /// §7.5 正面：项目内合法路径可读。
    #[test]
    fn test_resolve_inside_ok() {
        let dir = setup();
        let fs = RestrictedFs::new(dir.path()).unwrap();
        let p = fs.resolve("sub/file.txt").unwrap();
        assert!(p.ends_with("sub/file.txt"));
    }

    /// §7.5 负面（L-6）：`../../etc/passwd` 词法逃逸被拒。
    #[test]
    fn test_resolve_rejects_dotdot() {
        let dir = setup();
        let fs = RestrictedFs::new(dir.path()).unwrap();
        let err = fs.resolve("../../etc/passwd").unwrap_err();
        assert!(err.to_string().contains("逃逸"), ".. 逃逸必须拒: {err}");
    }

    /// §7.5 负面：绝对路径被拒。
    #[test]
    fn test_resolve_rejects_absolute() {
        let dir = setup();
        let fs = RestrictedFs::new(dir.path()).unwrap();
        let err = fs.resolve("/etc/passwd").unwrap_err();
        assert!(err.to_string().contains("绝对"), "绝对路径必须拒");
    }

    /// §7.5 负面（L-6）：symlink 指向项目根外 → canonicalize 后前缀校验拒绝。
    #[cfg(unix)]
    #[test]
    fn test_resolve_rejects_symlink_escape() {
        let dir = setup();
        let outside = std::env::temp_dir().join(format!("pwc_outside_{}", uuid::Uuid::new_v4()));
        std::fs::write(&outside, b"secret").unwrap();
        std::os::unix::fs::symlink(&outside, dir.path().join("evil_link")).unwrap();
        let fs = RestrictedFs::new(dir.path()).unwrap();
        let err = fs.resolve("evil_link").unwrap_err();
        assert!(
            err.to_string().contains("symlink 逃逸"),
            "symlink 逃逸必须拒: {err}"
        );
        let _ = std::fs::remove_file(&outside);
    }

    /// §7.5 正面：不存在的文件 resolve 报错（不静默返回越界路径）。
    #[test]
    fn test_resolve_missing_errors() {
        let dir = setup();
        let fs = RestrictedFs::new(dir.path()).unwrap();
        assert!(fs.resolve("nope.txt").is_err(), "不存在应报错");
    }
}
