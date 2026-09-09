//! S2（P5-FOUNDATION-01 N14）：改前快照 + 回滚。
//!
//! 语义：
//! - 写类工具（write_file / edit / apply_patch）执行**前**，把目标文件原始
//!   内容快照到 `~/.config/hearth/snapshots/<sid>/<seq>_<basename>`；目标
//!   **不存在**时落墓碑（manifest created=true；回滚 = 删除新建文件）。
//! - **每条快照带 manifest**（`<seq>.meta.json`：目标绝对路径 + created 标记）
//!   ——回滚必须还原到**快照时刻的绝对路径**，与执行回滚命令时的进程 cwd
//!   无关（v0.2.23 开发期实测：相对路径 + 当前 cwd 解析会还原到错误位置，
//!   test_rollback_* 三连红即此因）。
//! - bash 的任意写不在快照面（无法静态定位目标）——S2 已知边界。
//! - 回滚：LIFO。`rollback <sid>` = 只撤销最后一次写入；`rollback <sid>
//!   --seq N` = 撤销 seq ≥ N 的全部写入（倒序恢复）。
//! - 保留策略：单会话上限 200 个文件或 50MB（超限删最旧）；7 天清理留后续。
//!
//! **依赖注入**：核心逻辑走 `*_in(sessions_dir, ...)` 显式参数——测试零 env
//! 变更（v0.2.23 开发期实测：测试内改 HEARTH_SESSIONS_DIR 全局变量在并行
//! suite 下互踩，DI 后整类竞争消灭）。公开包装函数保持生产签名。

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

/// 单会话快照文件数上限。
const MAX_FILES: usize = 200;
/// 单会话快照总字节上限。
const MAX_BYTES: u64 = 50 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotEntry {
    pub seq: u64,
    /// 内容快照文件（原始字节；墓碑条目无内容文件 + created=true）。
    pub snapshot_path: PathBuf,
    /// 快照时刻的目标**绝对路径**（回滚以此为准，与执行回滚时的 cwd 无关）。
    pub target_abs: PathBuf,
    /// true = 目标当时不存在（墓碑；回滚 = 删除新建文件）。
    pub created: bool,
}

#[derive(Serialize, Deserialize)]
struct Meta {
    target: String,
    created: bool,
}

/// 依赖注入核心：以显式 sessions 目录枚举（**以 manifest 为主**——墓碑条目
/// created=true 没有内容文件，按内容文件枚举会漏，开发期实测）。
fn list_entries_in(sessions_dir: &Path, sid: &str) -> Result<Vec<SnapshotEntry>> {
    let dir = sessions_dir
        .parent()
        .map(|p| p.join("snapshots"))
        .unwrap_or_else(|| PathBuf::from(".hearth_snapshots"))
        .join(sid);
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut out: Vec<SnapshotEntry> = Vec::new();
    for e in std::fs::read_dir(&dir).with_context(|| format!("read {}", dir.display()))? {
        let p = e?.path();
        let Some(name) = p.file_name().map(|s| s.to_string_lossy().to_string()) else {
            continue;
        };
        let Some(stem) = name.strip_suffix(".meta.json") else {
            continue;
        };
        let Ok(seq) = stem.parse::<u64>() else {
            continue;
        };
        let meta: Meta = serde_json::from_str(
            &std::fs::read_to_string(&p).with_context(|| format!("read {}", p.display()))?,
        )?;
        // 内容文件 = manifest 对应的 `<seq>_<fname>`（墓碑不存在，正常）
        let content_path = match meta.target.rsplit(['/', '\\']).next() {
            Some(fname) => dir.join(format!("{seq:04}_{fname}")),
            None => continue,
        };
        out.push(SnapshotEntry {
            seq,
            snapshot_path: content_path,
            target_abs: PathBuf::from(&meta.target),
            created: meta.created,
        });
    }
    out.sort_by_key(|e| e.seq);
    Ok(out)
}

fn snap_dir_in(sessions_dir: &Path, sid: &str) -> PathBuf {
    sessions_dir
        .parent()
        .map(|p| p.join("snapshots"))
        .unwrap_or_else(|| PathBuf::from(".hearth_snapshots"))
        .join(sid)
}

fn snapshot_before_write_in(
    sessions_dir: &Path,
    sid: &str,
    cwd: &Path,
    rel_path: &str,
) -> Result<()> {
    let dir = snap_dir_in(sessions_dir, sid);
    std::fs::create_dir_all(&dir).context("create snapshots dir")?;
    let existing = list_entries_in(sessions_dir, sid)?;
    let seq = existing.last().map(|e| e.seq + 1).unwrap_or(1);
    let abs = cwd.join(rel_path);
    let fname = rel_path
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(rel_path)
        .to_string();
    let meta = Meta {
        target: abs.to_string_lossy().to_string(),
        created: !abs.is_file(),
    };
    std::fs::write(
        dir.join(format!("{seq:04}.meta.json")),
        serde_json::to_string(&meta)?,
    )
    .with_context(|| format!("write meta seq={seq}"))?;
    if abs.is_file() {
        std::fs::copy(&abs, dir.join(format!("{seq:04}_{fname}")))
            .with_context(|| format!("snapshot {}", abs.display()))?;
    }
    evict_in(sessions_dir, sid)?;
    Ok(())
}

fn evict_in(sessions_dir: &Path, sid: &str) -> Result<()> {
    let dir = snap_dir_in(sessions_dir, sid);
    loop {
        let entries = list_entries_in(sessions_dir, sid)?;
        let total: u64 = entries
            .iter()
            .filter_map(|e| std::fs::metadata(&e.snapshot_path).ok())
            .map(|m| m.len())
            .sum();
        if entries.len() <= MAX_FILES && total <= MAX_BYTES {
            return Ok(());
        }
        match entries.first() {
            Some(oldest) => {
                std::fs::remove_file(&oldest.snapshot_path).ok();
                std::fs::remove_file(dir.join(format!("{:04}.meta.json", oldest.seq))).ok();
            }
            None => return Ok(()),
        }
    }
}

fn rollback_in(
    sessions_dir: &Path,
    sid: &str,
    from_seq: Option<u64>,
    apply: bool,
) -> Result<String> {
    let entries = list_entries_in(sessions_dir, sid)?;
    if entries.is_empty() {
        return Ok("无快照可回滚。".into());
    }
    let threshold = from_seq.unwrap_or_else(|| entries.last().unwrap().seq);
    let mut selected: Vec<SnapshotEntry> =
        entries.into_iter().filter(|e| e.seq >= threshold).collect();
    selected.sort_by_key(|e| std::cmp::Reverse(e.seq));
    if selected.is_empty() {
        return Ok(format!("seq ≥ {threshold} 无快照可回滚。"));
    }
    let mut plan = Vec::new();
    for e in &selected {
        let act = if e.created {
            "删除(新建)"
        } else {
            "还原"
        };
        plan.push(format!("  {:04} {} {}", e.seq, act, e.target_abs.display()));
    }
    let plan_text = format!(
        "将回滚 {} 个快照（seq ≥ {threshold}，倒序）：\n{}",
        selected.len(),
        plan.join("\n")
    );
    if !apply {
        return Ok(format!("{plan_text}\n（dry-run，未执行——加 --yes 执行）"));
    }
    let dir = snap_dir_in(sessions_dir, sid);
    let mut done = Vec::new();
    for e in selected {
        let target = &e.target_abs;
        if e.created {
            if target.is_file() {
                std::fs::remove_file(target)
                    .with_context(|| format!("删除新建文件 {}", target.display()))?;
            }
        } else {
            let content = std::fs::read(&e.snapshot_path)
                .with_context(|| format!("read {}", e.snapshot_path.display()))?;
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            tools_builtin::atomic::write_atomic(target, &content)
                .with_context(|| format!("restore {}", target.display()))?;
        }
        done.push(e.seq);
        // 已应用的条目从快照库删除（回滚过的历史不再重放；manifest 成对删除）
        std::fs::remove_file(&e.snapshot_path).ok();
        std::fs::remove_file(dir.join(format!("{:04}.meta.json", e.seq))).ok();
    }
    Ok(format!(
        "已回滚 {} 个快照（seq {:?}，倒序还原完成）。\n注意：bash 产生的写不在快照面。",
        done.len(),
        done
    ))
}

// ── 生产签名（env 解析 sessions_dir，薄包装）──────────────────────────

/// 写类工具执行**前**调用（生产入口）。
pub fn snapshot_before_write(sid: &str, cwd: &Path, rel_path: &str) -> Result<()> {
    snapshot_before_write_in(&crate::session_store::sessions_dir(), sid, cwd, rel_path)
}

/// 回滚（生产入口）。
pub fn rollback(sid: &str, from_seq: Option<u64>, apply: bool) -> Result<String> {
    rollback_in(&crate::session_store::sessions_dir(), sid, from_seq, apply)
}

/// 交互确认：tty 下展示计划并等 y/n；非 tty 必须 `--yes`（结构化拒绝）。
pub fn confirm_or_deny(plan: &str, yes: bool) -> Result<bool> {
    if yes {
        return Ok(true);
    }
    if !std::io::stdin().is_terminal() {
        anyhow::bail!(
            "非交互模式回滚需要 --yes（结构化拒绝；同 approval_denied_noninteractive 语义）"
        );
    }
    println!("{plan}");
    println!("确认回滚？(y/N)");
    let mut ans = String::new();
    std::io::stdin().read_line(&mut ans)?;
    Ok(matches!(ans.trim(), "y" | "Y" | "yes"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_sid(tag: &str) -> String {
        format!(
            "s2-test-{tag}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )
    }

    #[test]
    fn test_snapshot_existing_and_new_file() {
        let dir = tempfile::tempdir().unwrap();
        let sdir = dir.path().join("sessions");
        let sid = unique_sid("a");
        let cwd = dir.path().join("ws");
        std::fs::create_dir_all(&cwd).unwrap();
        std::fs::write(cwd.join("orig.txt"), "OLD").unwrap();

        snapshot_before_write_in(&sdir, &sid, &cwd, "orig.txt").unwrap(); // 已存在 → 内容快照
        snapshot_before_write_in(&sdir, &sid, &cwd, "new.txt").unwrap(); // 不存在 → 墓碑

        let entries = list_entries_in(&sdir, &sid).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(!entries[0].created);
        assert_eq!(entries[0].target_abs.file_name().unwrap(), "orig.txt");
        assert!(entries[1].created);
        assert_eq!(entries[1].target_abs.file_name().unwrap(), "new.txt");
    }

    #[test]
    fn test_rollback_latest_restores_content() {
        let dir = tempfile::tempdir().unwrap();
        let sdir = dir.path().join("sessions");
        let sid = unique_sid("b");
        let cwd = dir.path().join("ws");
        std::fs::create_dir_all(&cwd).unwrap();
        std::fs::write(cwd.join("f.txt"), "V1").unwrap();
        snapshot_before_write_in(&sdir, &sid, &cwd, "f.txt").unwrap();
        std::fs::write(cwd.join("f.txt"), "V2").unwrap();
        snapshot_before_write_in(&sdir, &sid, &cwd, "f.txt").unwrap();
        std::fs::write(cwd.join("f.txt"), "V3").unwrap();

        // 只撤销最后一次 → 回到 V2（**与回滚时进程 cwd 无关**——manifest 存绝对路径）
        rollback_in(&sdir, &sid, None, true).unwrap();
        assert_eq!(std::fs::read(cwd.join("f.txt")).unwrap(), b"V2");
    }

    #[test]
    fn test_rollback_from_seq_restores_lifo() {
        let dir = tempfile::tempdir().unwrap();
        let sdir = dir.path().join("sessions");
        let sid = unique_sid("c");
        let cwd = dir.path().join("ws");
        std::fs::create_dir_all(&cwd).unwrap();
        std::fs::write(cwd.join("g.txt"), "BASE").unwrap();
        snapshot_before_write_in(&sdir, &sid, &cwd, "g.txt").unwrap(); // seq1: BASE
        std::fs::write(cwd.join("g.txt"), "EDIT1").unwrap();
        snapshot_before_write_in(&sdir, &sid, &cwd, "g.txt").unwrap(); // seq2: EDIT1
        std::fs::write(cwd.join("g.txt"), "EDIT2").unwrap();

        // seq ≥ 2 全撤销（倒序）→ 回到 EDIT1（seq2 快照内容）
        rollback_in(&sdir, &sid, Some(2), true).unwrap();
        assert_eq!(std::fs::read(cwd.join("g.txt")).unwrap(), b"EDIT1");
    }

    #[test]
    fn test_rollback_created_tombstone_deletes_file() {
        let dir = tempfile::tempdir().unwrap();
        let sdir = dir.path().join("sessions");
        let sid = unique_sid("d");
        let cwd = dir.path().join("ws");
        std::fs::create_dir_all(&cwd).unwrap();
        snapshot_before_write_in(&sdir, &sid, &cwd, "created.txt").unwrap(); // 墓碑
        std::fs::write(cwd.join("created.txt"), "NEWFILE").unwrap();

        rollback_in(&sdir, &sid, None, true).unwrap();
        assert!(!cwd.join("created.txt").exists(), "新建文件应被删除");
    }

    #[test]
    fn test_eviction_respects_file_cap() {
        let dir = tempfile::tempdir().unwrap();
        let sdir = dir.path().join("sessions");
        let sid = unique_sid("e");
        let cwd = dir.path().join("ws");
        std::fs::create_dir_all(&cwd).unwrap();
        std::fs::write(cwd.join("h.txt"), "x").unwrap();
        for _ in 0..(MAX_FILES as u64 + 10) {
            snapshot_before_write_in(&sdir, &sid, &cwd, "h.txt").unwrap();
        }
        let entries = list_entries_in(&sdir, &sid).unwrap();
        assert!(
            entries.len() <= MAX_FILES,
            "快照数超上限：{} > {MAX_FILES}",
            entries.len()
        );
    }

    #[test]
    fn test_dry_run_does_not_apply() {
        let dir = tempfile::tempdir().unwrap();
        let sdir = dir.path().join("sessions");
        let sid = unique_sid("f");
        let cwd = dir.path().join("ws");
        std::fs::create_dir_all(&cwd).unwrap();
        std::fs::write(cwd.join("k.txt"), "V1").unwrap();
        snapshot_before_write_in(&sdir, &sid, &cwd, "k.txt").unwrap();
        std::fs::write(cwd.join("k.txt"), "V2").unwrap();
        let out = rollback_in(&sdir, &sid, None, false).unwrap();
        assert!(out.contains("dry-run"), "须标明未执行");
        assert_eq!(std::fs::read(cwd.join("k.txt")).unwrap(), b"V2");
    }
}
