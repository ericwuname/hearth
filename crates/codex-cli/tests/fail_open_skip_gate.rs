//! 门禁：集成测试**不得**「打印 SKIP 后 `return;`」伪装 PASS（D-140，2026-10-04, traecode）。
//!
//! 背景：本仓反复复发「门禁自己失守」缺陷族——防线在环境异常/前置缺失时**静默 PASS**，
//! 让"没测"看起来像"测过了"（D-132/D-136/D-137 逐个收口）。D-137 已把
//! `crates/codex-cli/tests/secret_scan_gate.rs` 的那处 `SKIP→return` 改成 fail-closed；
//! 本轮体检又抓到**漏网**的两处：`crates/service/tests/cli_contract_test.rs` 的两个契约
//! 测试体，找不到 codex 二进制时 `eprintln!("...SKIP..."); return;`——Rust 里测试
//! **正常返回即 PASS**，于是 CLI↔service 契约门禁在缺前置时**谎报「契约无断裂」**。
//! （实测：Windows 上 `cli_bin()` 只探无后缀的 `codex`，永远找不到 `codex.exe` ⇒ 该门禁
//! 在本机**长期为死**、2 个测试永远 SKIP-PASS，直到 D-140 才暴露并收口。）
//!
//! 判据（源码级，宁可漏报不误报）：只扫 `crates/*/tests/**/*.rs`（集成测试文件），
//! 逐行剥掉 `//` 注释后，若某行是 `return` 早退语句，且**其前 3 行内**出现含 `SKIP`
//! 的代码行，即判定为 fail-open 伪装。需要条件性跳过的测试请改成 **fail-closed**：
//! 缺前置就 `panic!`/`assert!`（打印原因并让测试**失败**），而不是打印后 `return`。
//!
//! 文件头自报盲区：
//! ① 只扫 `tests/` 目录，**不扫** `src/**` 内嵌 `#[cfg(test)]` 模块（那里存在**能力门**，
//!    如 sandbox 的 landlock、agent-core 的 ping.exe 探测，判为合法能力缺失），故此形态
//!    在 src 内复发本门禁**不拦**——宁可漏报不误报。
//! ② 只做文本级逐行扫描（剥 `//` 注释后匹配），不做语义分析；`SKIP` 打印与 `return`
//!    相隔超过 3 行、或改用 `continue`/`?`/`std::process::exit` 等其它早退形态，不在覆盖范围。

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

/// 是否为「早退 `return`」语句行。
fn is_return(code: &str) -> bool {
    let t = code.trim();
    t.starts_with("return") && t.ends_with(';')
}

#[test]
fn tests_do_not_fake_pass_with_skip_then_return() {
    let root = workspace_root();
    let crates = root.join("crates");
    assert!(crates.is_dir(), "缺少 crates 目录：{}", crates.display());

    // 收集所有 `crates/<pkg>/tests/**/*.rs`
    let mut files: Vec<PathBuf> = Vec::new();
    for e in std::fs::read_dir(&crates).into_iter().flatten().flatten() {
        let tests = e.path().join("tests");
        if tests.is_dir() {
            collect_rs(&tests, &mut files);
        }
    }
    assert!(
        files.len() >= 10,
        "扫描面过小（{} 个测试文件）——门禁自身可能失效",
        files.len()
    );

    let mut offenders: Vec<String> = Vec::new();
    for f in &files {
        let Ok(text) = std::fs::read_to_string(f) else {
            continue;
        };
        // 最近 3 行（已剥注释）——容下 `SKIP` 打印与 `return;` 之间的空行/多行语句。
        let mut recent: Vec<String> = Vec::new();
        for (i, raw) in text.lines().enumerate() {
            let code = code_part(raw);
            if is_return(code) && recent.iter().any(|l| l.contains("SKIP")) {
                offenders.push(format!(
                    "{}:{} {}",
                    f.strip_prefix(&root).unwrap_or(f).display(),
                    i + 1,
                    raw.trim()
                ));
            }
            recent.push(code.to_string());
            if recent.len() > 3 {
                recent.remove(0);
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "发现**打印 SKIP 后 return** 的 fail-open 假 PASS（测试正常返回＝通过，缺前置却谎报成功）。\
         请改成 fail-closed：缺前置时 `panic!`/`assert!(false, …)`（打印原因并让测试**失败**）。\
         命中：{offenders:#?}"
    );
}
