//! 门禁：seccomp 白名单**计数**在代码与两份文档间必须一致（D-148，2026-10-04, traecode）。
//!
//! 背景（真实漂移）：白名单权威值是 `crates/sandbox/src/lib.rs` 的
//! `SECCOMP_ALLOWLIST: [u32; 136]`，但**三处**仍写旧值 **99**：
//!   · 代码自述注释（`lib.rs`「默认 deny 白名单——99 个实测必需 syscall」）；
//!   · 契约文档 `docs/seccomp-allowlist-v1.md`（自称"前后端/运维**唯一依据**"、§3 更规定
//!     "白名单变更必须同步本文件 + 单测"）；
//!   · 用户指南 `docs/hearth-cli-guide.md`（照抄该计数）。
//! 而 README 写的是 **136**（正确）⇒ 同一事实对外出现两个值，运维/用户照"唯一依据"读到的是错的。
//! 属"声称≠实现 / 文档与实现脱节"族（承 D-89/D-133/D-147）。
//!
//! 判据（文本级，**没有任何需要维护的清单**）：
//! - 从 `SECCOMP_ALLOWLIST: [u32; N]` 解析权威 `N`；
//! - 在被扫文件里，凡出现「`<数字> 个` + `实测必需`」的**计数声明**，其数字必须等于 `N`。
//!
//! 文件头自报盲区：
//! ① 只做**计数**一致性校验，**不**校验名单内容（谁在名单里）——那需要 syscall 号↔名字映射，
//!    代价大且易误报；名单的权威源是代码本身。
//! ② 判据锚在"数字 `个` +（`实测必需`｜`包含全部`）"这两类**已见措辞**上：若有人改写措辞绕过本门禁，
//!    属"有意规避"，超出本尺射程（宁可漏报不误报）。

use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

/// 从 `SECCOMP_ALLOWLIST: [u32; N]` 解析权威计数 `N`。
fn code_allowlist_len(src: &str) -> Option<usize> {
    let key = "SECCOMP_ALLOWLIST: [u32;";
    let i = src.find(key)?;
    let after = &src[i + key.len()..];
    let digits: String = after
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

/// 一行是否是"白名单计数声明"——已见两种措辞（都要求"数字 + 个"）：
///   · `… 99 个实测必需 syscall`（代码注释 / 指南）
///   · `… 白名单（99 个，实测必需）`（契约文档 §1）
///   · `… 白名单常量包含全部 99 个（缺→fail）`（契约文档 §4）
fn is_count_line(line: &str) -> bool {
    line.contains("实测必需") || line.contains("包含全部")
}

/// 抽取某文件里每一条「`<数字> 个`（计数声明行内）」的计数 → (行号, 数字)。
fn declared_counts(text: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if !is_count_line(line) {
            continue;
        }
        let Some(pos) = line.find('个') else {
            continue;
        };
        let head = line[..pos].trim_end();
        let digits: String = head
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        if let Ok(n) = digits.parse() {
            out.push((i + 1, n));
        }
    }
    out
}

#[test]
fn seccomp_allowlist_count_is_consistent_across_code_and_docs() {
    let root = root();
    let code_path = root
        .join("crates")
        .join("sandbox")
        .join("src")
        .join("lib.rs");
    let code = std::fs::read_to_string(&code_path).expect("sandbox/src/lib.rs 必须存在");
    let n = code_allowlist_len(&code)
        .expect("解析不到 `SECCOMP_ALLOWLIST: [u32; N]`——白名单常量形态变了？门禁自身可能失效");
    assert!(
        n >= 50,
        "白名单计数 {n} 过小（白名单被清空？）——门禁自身可能失效"
    );

    let mut scanned = vec![code_path.clone()];
    for rel in [
        ["docs", "seccomp-allowlist-v1.md"],
        ["docs", "hearth-cli-guide.md"],
    ] {
        scanned.push(root.join(rel[0]).join(rel[1]));
    }

    let mut offenders: Vec<String> = Vec::new();
    let mut seen_any = false;
    for path in &scanned {
        let Ok(text) = std::fs::read_to_string(path) else {
            offenders.push(format!("{}: 读不到（文件被删/改名？）", path.display()));
            continue;
        };
        for (line_no, declared) in declared_counts(&text) {
            seen_any = true;
            if declared != n {
                offenders.push(format!(
                    "{}:{} 声明 {} 个，实际 {} 个",
                    path.strip_prefix(&root).unwrap_or(path).display(),
                    line_no,
                    declared,
                    n
                ));
            }
        }
    }
    assert!(
        seen_any,
        "被扫文件里**一条**「<数字> 个…实测必需」计数声明都没找到——门禁自身可能失效（措辞被改？）"
    );
    assert!(
        offenders.is_empty(),
        "seccomp 白名单计数在代码与文档间**不一致**（对外出现两个值，运维/用户照文档读到错的）：\n{offenders:#?}\n\
         权威值 = `SECCOMP_ALLOWLIST: [u32; {n}]`。请把各处的计数与代码对齐；\
         若名单内容也变了，请同步契约文档的说明（本门禁只锁**计数**）。"
    );
}
