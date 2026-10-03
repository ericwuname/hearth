//! 门禁：**无界文件读入**不得回归（D-131，2026-10-03, traecode）。
//!
//! 背景：本仓有一条**反复复发**的缺陷族——"读入无上限"。D-33 把"读入无上限 /
//! 进程不收尸"收敛成 `bounded-io` 一套原语；D-125 又把 CLI 的四处会话读口有界化。
//! 但收口总有漏网：2026-10-03 体检发现 `config` / `.env` / `coverage` / `/cost` /
//! 快照 meta，以及 service 启动期的**模板清单**与**工具 manifest** 仍是裸
//! `std::fs::read_to_string`——任一被指向一个超大文件，就把 CLI（或服务启动）拖进 OOM。
//! 卡在"每次只修被点名的那几处"就会一直漏；本门禁改为**把不变式钉死**：
//! 只要扫描范围内出现裸读，即判红，除非该行带显式豁免标记。
//!
//! 判据：
//! - 命中形态：非测试代码里的 `read_to_string(` 或 `fs::read(`（`fs::read_dir(` 与
//!   `read_to_end(` 不命中——前者非读文件，后者被 `bounded-io` 自身用于实现有界读）。
//! - 豁免：该行带标记 `bounded-io-exempt:`（必须写明理由，如"字节级还原不可截断"）。
//! - `#[cfg(test)]` 模块内的整份读入**一律豁免**（测试用临时小文件，读满无害）；
//!   用大括号深度跟踪判定，能识别 `#[cfg(test)]` 与 `mod` 之间夹 `use`/其它属性的写法。
//!
//! 文件头自报盲区：
//! ① 只扫 `crates/{codex-cli,service,tool-runtime}/src/**/*.rs`（**产品运行时的
//!    用户可见面**）。其余 crate（`project-xray` / `llm-gateway` / `llm-replay` /
//!    `tools-builtin::patch` / `agent-core::loop`）的同类读口尚未纳管——属**已知待办**
//!    （其中若干处已由前置 `metadata().len()` 校验兜底）。扩展范围时请同批处理。
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

#[test]
fn no_unbounded_file_reads_in_product_crates() {
    let root = workspace_root();
    let mut files = Vec::new();
    for c in ["codex-cli", "service", "tool-runtime"] {
        let src = root.join("crates").join(c).join("src");
        assert!(src.is_dir(), "缺少源码目录：{}", src.display());
        collect_rs(&src, &mut files);
    }
    assert!(
        files.len() >= 20,
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
        for (i, raw) in text.lines().enumerate() {
            let code = code_part(raw);
            let trimmed = code.trim();
            let opens = code.matches('{').count() as i32;
            let closes = code.matches('}').count() as i32;
            if trimmed == "#[cfg(test)]" {
                pending_cfg_test = true;
            }
            let inside_test = test_base.is_some_and(|b| depth > b);
            if !inside_test && is_unbounded_read(code) && !raw.contains("bounded-io-exempt:") {
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
        }
    }

    assert!(
        offenders.is_empty(),
        "发现**无界文件读入**（裸 `read_to_string` / `fs::read` 会把整份文件读进内存，\
         一条超大文件即可 OOM）。请改用 `bounded_io::read_file_text_capped_std`（超限要\
         留痕，见 D-125/D-131 口径）；若确需整份读入（如字节级还原），在该行加\
         `// bounded-io-exempt: <理由>`。命中：{offenders:#?}"
    );
}
