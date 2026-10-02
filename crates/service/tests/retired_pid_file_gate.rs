//! 门禁：**已退役的 PID 文件写入不得回归**（D-130，2026-10-02, traecode）。
//!
//! 背景：`service/src/main.rs` 启动期原有一行
//! `let _ = std::fs::write("/tmp/codex.pid", std::process::id().to_string());`，
//! 注释称 "Write PID file for Observer health checks"。核对全仓（observer crate、
//! CLI、脚本、文档）后确认该消费者**不存在**：没有任何代码读它；运维面的 pid 文件
//! 由 `run-hearth.sh` 自己维护（`.hearth-service.pid`，另一个文件）。而且硬编码
//! `/tmp` 在 Windows 上不存在 ⇒ 这行**永远静默失败**（错误又被 `let _ =` 吞掉），
//! 连"出了问题能看见"都做不到。属"只写不读 + 不实注释"的死物，已删除。
//!
//! 为什么要门禁：删掉的一行看不出代价，而"顺手补个 pid 文件"是很自然的动作——
//! 一旦有人补回去，就会同时把**不实的注释**（声称有消费者）与**吞错误**的写法带回来。
//! 本门禁把"删除"钉住：真要恢复该能力，请**先接线消费者**（谁读、读来做什么），
//! 用**可移植路径**（如 config/运行目录，而非硬编码 `/tmp`），并把失败**留痕**；
//! 那时再来改本门禁并写明理由。
//!
//! 文件头自报盲区：① 只做**文本级**扫描（逐行剥掉 `//` 注释后匹配"写入 /tmp 的 pid
//! 文件"这一形态），不做语义分析；② 只覆盖 `crates/service/src/**`——若将来别的
//! crate 也想写 pid 文件，请同步扩展扫描范围。

use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_rs(&p, out);
        } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
            out.push(p);
        }
    }
}

/// 逐行剥掉 `//` 之后的部分（本门禁只关心**代码**，注释里提到旧写法不算回归）。
fn code_part(line: &str) -> &str {
    match line.find("//") {
        Some(i) => &line[..i],
        None => line,
    }
}

#[test]
fn retired_pid_file_write_does_not_come_back() {
    let src = workspace_root().join("crates").join("service").join("src");
    let mut files = Vec::new();
    collect_rs(&src, &mut files);
    assert!(!files.is_empty(), "未找到 service 源码——门禁自身可能失效");

    let mut offenders = Vec::new();
    for f in &files {
        let Ok(text) = std::fs::read_to_string(f) else {
            continue;
        };
        for (i, line) in text.lines().enumerate() {
            let code = code_part(line);
            // 形态：往一个 `*.pid` 路径写文件（无论 write/create/OpenOptions）。
            let writes =
                code.contains("write") || code.contains("OpenOptions") || code.contains("create(");
            if writes && code.contains(".pid") {
                offenders.push(format!(
                    "{}:{} {}",
                    f.strip_prefix(workspace_root()).unwrap_or(f).display(),
                    i + 1,
                    line.trim()
                ));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "已退役的 PID 文件写入回归了（D-130）。若确要恢复该能力，请**先接线消费者**\
         （谁读它、读来做什么）、用**可移植路径**（不要硬编码 `/tmp`）并让失败**留痕**，\
         然后同步更新本门禁并写明理由。命中：{offenders:#?}"
    );
}
