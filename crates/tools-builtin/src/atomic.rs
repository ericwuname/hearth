//! S1（P5-FOUNDATION-01 N12）：工具层原子写——"骨架给自己穿了救生衣，
//! 给用户代码没穿"的收口。
//!
//! 背景（三层摸底 + 十轮深挖实证）：项目**自己的**状态文件早已是 tmp+rename
//! 原子写（`session_store.rs:58/82/124`、`config.rs:78`、`memory/src/lib.rs:147`、
//! `project-sync/registry.rs:32`），但用户代码路径（edit/patch/write_file）仍是
//! 裸 `tokio::fs::write`——`edit.rs` 超长写甚至**先 truncate 原文件再分块追加**，
//! 崩溃 = 原文件被毁。本模块把既有保障复用到用户代码路径。
//!
//! 语义：同目录 tmp → write_all → sync_all → rename（POSIX 原子，最后一步）。
//! 任一步失败返回 Err 且**原文件字节不变**；rename 失败必须清理 tmp。
//! 崩溃安全：kill -9 在 rename 前的任意时刻，磁盘上要么是旧文件要么是完整的
//! tmp（下次写入覆盖），**不存在半截内容的原文件**。

use std::io::Write as _;
use std::path::Path;

/// 原子写 `content` 到 `path`（覆盖语义）。
///
/// - tmp 与目标**同目录**（保证 rename 同文件系统，跨盘 rename 会 EXDEV）；
/// - tmp 名含 pid + 纳秒时间戳（并发写同一目标互不踩踏）；
/// - `sync_all` 在 rename 前——数据先落盘再原子改名（防"改名成功但内容
///   还在 page cache"的掉电窗口）；
/// - 失败清理：rename 失败时 unlink tmp，不留垃圾。
pub fn write_atomic(path: &Path, content: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let fname = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".into());
    let tmp = parent.join(format!(
        ".{}.hearth-tmp-{}-{}",
        fname,
        std::process::id(),
        nanos()
    ));
    let res = (|| -> std::io::Result<()> {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(content)?;
        f.sync_all()?;
        drop(f); // 显式关闭后再 rename（Windows 上打开句柄会阻止 rename）
        std::fs::rename(&tmp, path)?;
        Ok(())
    })();
    if res.is_err() {
        // 失败清理：不留 tmp 垃圾（best-effort，清理失败不掩盖原错误）
        let _ = std::fs::remove_file(&tmp);
    }
    res
}

fn nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_residues(dir: &Path) -> Vec<String> {
        std::fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.contains("hearth-tmp"))
            .collect()
    }

    /// S1-G1：正常写入——内容正确、无 tmp 残留。
    #[test]
    fn test_write_atomic_basic() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.txt");
        write_atomic(&p, b"hello").unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"hello");
        assert!(tmp_residues(dir.path()).is_empty(), "不得残留 tmp");
    }

    /// S1-G1b：覆盖已有文件——rename 语义下旧内容被整体替换。
    #[test]
    fn test_write_atomic_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("b.txt");
        std::fs::write(&p, b"OLD-CONTENT").unwrap();
        write_atomic(&p, b"NEW").unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"NEW");
        assert!(tmp_residues(dir.path()).is_empty());
    }

    /// S1-R3：目标是个**目录** → rename 必败（EISDIR）→ 返回 Err 且 tmp 清理。
    #[test]
    fn test_write_atomic_rename_failure_cleans_tmp() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("sub"); // 目录，不是文件
        std::fs::create_dir(&p).unwrap();
        let e = write_atomic(&p, b"x").unwrap_err();
        let _ = e; // 错误类型平台相关（EISDIR/EEXIST），关键在下面两条：
        assert!(tmp_residues(dir.path()).is_empty(), "失败必须清理 tmp");
    }

    /// S1-R1：父目录不可写 → tmp 创建失败 → Err 且**原文件字节不变**。
    /// （root 环境下 chmod 无效 → 软跳过；门禁 VM 以非 root 跑，有效。）
    #[test]
    fn test_write_atomic_failure_preserves_original() {
        let dir = tempfile::tempdir().unwrap();
        let ro = dir.path().join("ro");
        std::fs::create_dir(&ro).unwrap();
        let p = ro.join("orig.txt");
        std::fs::write(&p, b"PRECIOUS").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&ro, std::fs::Permissions::from_mode(0o555)).unwrap();
            let res = write_atomic(&p, b"OVERWRITE");
            // 恢复权限以便 tempdir 清理
            std::fs::set_permissions(&ro, std::fs::Permissions::from_mode(0o755)).unwrap();
            match res {
                Err(_) => {
                    assert_eq!(
                        std::fs::read(&p).unwrap(),
                        b"PRECIOUS",
                        "失败时原文件字节不变（S1 核心保证）"
                    );
                }
                Ok(()) => {
                    // root 环境：chmod 不生效，写入"成功"——软跳过（不假装红）
                    eprintln!("[skip] running as privileged user, permission test vacuous");
                }
            }
        }
        #[cfg(not(unix))]
        {
            let _ = write_atomic(&p, b"OVERWRITE");
        }
    }

    /// 大内容（>1MB）：一次性原子写，内容逐字节一致。
    #[test]
    fn test_write_atomic_large_content() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("big.bin");
        let content: Vec<u8> = (0..1_200_000u32).map(|i| (i % 251) as u8).collect();
        write_atomic(&p, &content).unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), content);
        assert!(tmp_residues(dir.path()).is_empty());
    }
}
