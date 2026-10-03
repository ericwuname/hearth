//! 门禁：`project-xray` 的**无界文件读入**不得回归（D-142，2026-10-04, traecode）。
//!
//! 背景与**范围界定**（重要）：本仓有一条反复复发的缺陷族——"读入无上限"
//! （D-33 收敛出 `bounded-io` 一套原语；D-38/D-51/D-53/D-70/D-75/D-86/D-125/D-131
//! 逐个落点收口）。`codex-xray` 是**只读扫描工具**：`scan --root` 解析被扫描工作区的
//! 根 `Cargo.toml`，再逐 `.rs` 统计行数/测试数；`wiring` 逐条读**被扫描工作区的源文件**
//! 断言其接线锚点仍在。这些读的是**被扫描工作区的文件**（随仓库规模增长），
//! 与 D-38（`code-index` 遍历会话 workspace 的 `.rs`）、D-75（`agent-core` 读正在处理
//! 的 cwd 仓库文件）**同类**，属于**必须加界**的一族——
//! **不是** D-77 裁决「不改」的那一类（D-77 只圈定"操作者自己的本机**配置/清单/价表/
//! replay 夹具**"，且其债条目**逐一枚举了 6 个落点、不含 project-xray**）。
//!
//! 说明（如实登记）：`codex-cli/tests/unbounded_read_gate.rs` 的头注曾把 `project-xray`
//! 与 service/tool-runtime 并列为"D-77 裁决范围内"。那是**把 D-77 的圈定范围写宽了**：
//! D-77 的本体裁定只覆盖那 6 个**操作者配置**落点，而 xray 读的是**被扫描的工作区源码**。
//! 本门禁即为纠正该表述、并为 project-xray 自带一把尺子（D-142）。
//!
//! 判据（与 `codex-cli` 同款）：
//! - 命中形态：非测试代码里的 `read_to_string(` 或 `fs::read(`（`fs::read_dir(` 与
//!   `read_to_end(` 不命中——前者非读文件，后者是 `bounded-io` 自身实现有界读的手段）。
//! - 豁免：该行**前 3 行内**带标记 `bounded-io-exempt:`（须写明理由——D-77 裁决）。
//!   注意标记要写在**前一行**注释里：门禁只回看"已处理过的"前 3 行，写在同行的行尾注释
//!   不在回看窗口内。
//! - `#[cfg(test)]` 模块内的整份读入一律豁免（测试用临时小文件，读满无害）。
//!
//! 文件头自报盲区：
//! ① 只扫 `crates/project-xray/src/**`。
//! ② 只做**文本级**逐行扫描（剥掉 `//` 注释后匹配），不做语义分析；`read_to_end` /
//!    `tokio::io::read` 等其它无界读形态不在覆盖范围。
//! ③ 标记是"信任式"豁免：不校验被豁免行是否真的只读操作者配置（人工判定，靠 D-77 口径把关）。

use std::path::{Path, PathBuf};

/// 仓库根：`crates/project-xray/tests/xxx.rs` → 上溯三级。
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

/// 逐行剥掉 `//` 之后的部分（注释里提到旧写法不算回归）。
fn code_part(line: &str) -> &str {
    match line.find("//") {
        Some(i) => &line[..i],
        None => line,
    }
}

/// 该行是否是"无界整份文件读入"的调用点。
fn is_unbounded_read(code: &str) -> bool {
    code.contains("read_to_string(") || code.contains("fs::read(")
}

const EXEMPT_MARK: &str = "bounded-io-exempt:";

#[test]
fn no_unbounded_file_reads_in_project_xray() {
    let root = workspace_root();
    let src = root.join("crates").join("project-xray").join("src");
    assert!(src.is_dir(), "缺少源码目录：{}", src.display());
    let mut files = Vec::new();
    collect_rs(&src, &mut files);
    assert!(
        files.len() >= 3,
        "扫描面过小（{} 个源文件）——门禁自身可能失效",
        files.len()
    );

    let mut offenders: Vec<String> = Vec::new();
    for f in &files {
        let Ok(text) = std::fs::read_to_string(f) else {
            continue;
        };
        let mut depth: i32 = 0;
        let mut test_base: Option<i32> = None;
        let mut pending_cfg_test = false;
        let mut recent: Vec<&str> = Vec::new();
        for (i, raw) in text.lines().enumerate() {
            let code = code_part(raw);
            let trimmed = code.trim();
            let opens = code.matches('{').count() as i32;
            let closes = code.matches('}').count() as i32;
            if trimmed == "#[cfg(test)]" {
                pending_cfg_test = true;
            }
            let inside_test = test_base.is_some_and(|b| depth > b);
            let exempt = recent.iter().any(|l| l.contains(EXEMPT_MARK));
            if !inside_test && is_unbounded_read(code) && !exempt {
                offenders.push(format!(
                    "{}:{} {}",
                    f.strip_prefix(&root).unwrap_or(f).display(),
                    i + 1,
                    raw.trim()
                ));
            }
            if pending_cfg_test && trimmed.starts_with("mod ") && opens > 0 {
                test_base = Some(depth);
                pending_cfg_test = false;
            }
            depth += opens - closes;
            if test_base.is_some_and(|b| depth <= b) {
                test_base = None;
            }
            recent.push(raw);
            if recent.len() > 3 {
                recent.remove(0);
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "发现**无界文件读入**（裸 `read_to_string` / `fs::read` 会把整份文件读进内存，\
         被扫描工作区的源码随仓库规模增长）。请改用 `bounded_io::read_file_text_capped_std`\
         （超限要留痕/拒绝，见 D-38/D-131 口径）；若属 D-77 裁决的「操作者本机配置/清单」\
         或需字节级还原，在该行**前一行**加 `// bounded-io-exempt: <理由>`。命中：{offenders:#?}"
    );
}
