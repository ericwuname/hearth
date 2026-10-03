//! 门禁：`codex-cli` 的**无界文件读入**不得回归（D-131，2026-10-03, traecode）。
//!
//! 背景与**范围界定**（重要）：本仓有一条反复复发的缺陷族——"读入无上限"（D-33 收敛出
//! `bounded-io` 一套原语；D-51/D-70/D-75/D-86/D-125 逐个落点收口）。本轮体检又抓到三处
//! **确有边界**的裸读并已收口：`coverage` 报告（tarpaulin JSON，含逐行明细，随仓库规模长）、
//! `/cost` 报告（会话累积产物）、以及 `setup` 的 `.env`（**兼修静默清空**：旧
//! `read_to_string(..).unwrap_or_default()` 在读失败时退化成空串 ⇒ 覆盖写回把用户
//! provider key 清掉，= D-128 修掉的数据丢失换触发面）。
//!
//! **刻意圈定边界**：D-77（顶层裁决）已判定"**操作者本机配置/清单/replay 夹具**"这类
//! 无界增长特性的读**不改**（加 cap 无安全收益，反有"合法大文件被截断 → 解析失败"的
//! 静默降级风险）。故本门禁**只覆盖 `crates/codex-cli/src`**，且允许经 `bounded-io-exempt:`
//! 标记的显式豁免——D-77 类读（`config.rs` 的 config.toml、快照 meta）与"字节级还原"
//! （`snapshot_store.rs` 的快照内容）即以此方式如实标注。**不要**顺手把 service /
//! tool-runtime / llm-* / project-xray 的同类读也纳入：它们在 D-77 的裁决范围内。
//!
//! 判据：
//! - 命中形态：非测试代码里的 `read_to_string(` 或 `fs::read(`（`fs::read_dir(` 与
//!   `read_to_end(` 不命中——前者非读文件，后者被 `bounded-io` 自身用于实现有界读）。
//! - 豁免：该行（或**其前 3 行内**）带标记 `bounded-io-exempt:`（必须写明理由——D-77 裁决
//!   或"字节级还原"）。回看 3 行是为了容下多行语句/注释换行，不必把理由硬塞成一行。
//! - `#[cfg(test)]` 模块内的整份读入**一律豁免**（测试用临时小文件，读满无害）；
//!   用大括号深度跟踪判定，能识别 `#[cfg(test)]` 与 `mod` 之间夹 `use`/其它属性的写法。
//!
//! 文件头自报盲区：
//! ① 只扫 `crates/codex-cli/src/**`（见上：范围由 D-77 圈定）。
//! ② 只做**文本级**逐行扫描（剥掉 `//` 注释后匹配），不做语义分析；因此
//!    `#[cfg(all(test, …))]` 等等价写法、以及 `read_to_end` / `tokio::io::read` 等
//!    其它无界读形态不在覆盖范围。

use std::path::{Path, PathBuf};

/// 仓库根：`crates/codex-cli/tests/xxx.rs` → 上溯三级。
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
fn no_unbounded_file_reads_in_codex_cli() {
    let root = workspace_root();
    let src = root.join("crates").join("codex-cli").join("src");
    assert!(src.is_dir(), "缺少源码目录：{}", src.display());
    let mut files = Vec::new();
    collect_rs(&src, &mut files);
    assert!(
        files.len() >= 8,
        "扫描面过小（{} 个源文件）——门禁自身可能失效",
        files.len()
    );

    let mut offenders: Vec<String> = Vec::new();
    for f in &files {
        let Ok(text) = std::fs::read_to_string(f) else {
            continue;
        };
        // 大括号深度 + `#[cfg(test)]` 模块的基点深度——用于跳过测试代码。
        let mut depth: i32 = 0;
        let mut test_base: Option<i32> = None;
        let mut pending_cfg_test = false;
        // 最近 3 行原始文本——豁免标记可以写在调用行的上一行（多行语句/注释换行都容得下）。
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
            // `#[cfg(test)]` 后第一个带 `{` 的 `mod` 即测试模块入口。
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
        "发现**无界文件读入**（裸 `read_to_string` / `fs::read` 会把整份文件读进内存）。\
         请改用 `bounded_io::read_file_text_capped_std`（超限要留痕，见 D-125/D-131 口径）；\
         若属 D-77 裁决的「操作者本机配置/清单」或需字节级还原（截断即损坏），在该行或上一行加\
         `// bounded-io-exempt: <理由>`。命中：{offenders:#?}"
    );
}
