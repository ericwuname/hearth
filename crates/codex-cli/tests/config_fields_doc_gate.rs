//! 门禁：README / 用户指南里「可改字段」清单必须涵盖 `config set` 实际支持的**全部键**
//! （D-150，2026-10-04, traecode）。
//!
//! 背景（真实漂移）：`hearth config set` 实际支持 **8** 个键（`provider` / `model` / `url` /
//! `api-key` / `mode` / `feedback-prompt` / `egress-allowlist` / `read-roots`），而
//! README「可改字段」只列 6（漏 `egress-allowlist`/`read-roots`）、用户指南「支持字段」只列 5
//! （再漏 `model`）。操作者照文档读，会以为 `model`/出网白名单/读根不可配——属"文档与实现脱节"
//! 族（承 D-89/D-133/D-147/D-148）。
//!
//! 判据（文本级）：权威源 = `crates/codex-cli/src/config.rs` 的 `set()` 错误消息
//! 「未知字段: {other}——可用: A / B / ...」；解析出该键清单后，断言 README 含「可改字段」的那行
//! 与 `docs/hearth-cli-guide.md` 含「支持字段」的那行**都出现每个键**（`-`/`_` 归一化后比较——
//! 代码同时接受 `api-key`/`api_key`）。
//!
//! 文件头自报盲区：① 只钉"该行是否列出这个键"，**不**钉顺序/格式/描述正确性；
//! ② 依赖那条错误消息的措辞：措辞被改会让本门禁报「解析不到键清单」（**自报失败，不静默放行**）。

use std::path::PathBuf;

fn root() -> PathBuf {
    // crates/codex-cli → ../..
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

/// 归一化键名：去掉 `-`/`_`，转小写（`api-key` == `api_key` == `apiKey`）。
fn norm(s: &str) -> String {
    s.chars()
        .filter(|c| *c != '-' && *c != '_')
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// 从 `config.rs` 的错误消息「…可用: A / B / …」解析权威键清单。
fn code_field_keys(config_src: &str) -> Vec<String> {
    let anchor = "可用:";
    let i = config_src
        .find(anchor)
        .expect("解析不到 `可用:` 键清单——config.rs 的 set() 错误消息措辞变了？门禁自身可能失效");
    let after = &config_src[i + anchor.len()..];
    // 键清单止于该字符串字面量的收尾引号。
    let upto = after.find('"').unwrap_or(after.len());
    after[..upto]
        .split('/')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// 取含 `marker` 的那一行（首个匹配）。
fn line_with<'a>(text: &'a str, marker: &str) -> Option<&'a str> {
    text.lines().find(|l| l.contains(marker))
}

#[test]
fn documented_config_fields_cover_all_code_keys() {
    let root = root();
    let cfg_path = root
        .join("crates")
        .join("codex-cli")
        .join("src")
        .join("config.rs");
    let cfg = std::fs::read_to_string(&cfg_path).expect("config.rs 必须存在");
    let keys = code_field_keys(&cfg);
    assert!(
        keys.len() >= 6,
        "解析出的键清单过少（{keys:?}）——门禁自身可能失效"
    );

    let cases: [(&str, &str, &str); 2] = [
        ("README.md", "可改字段", "README"),
        (
            "docs/hearth-cli-guide.md",
            "支持字段",
            "hearth-cli-guide.md",
        ),
    ];

    let mut offenders: Vec<String> = Vec::new();
    for (rel, marker, label) in cases {
        let path = root.join(rel);
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("读不到 {rel}: {e}（文件被删/改名？）"));
        let Some(line) = line_with(&text, marker) else {
            offenders.push(format!("{label}：找不到含「{marker}」的那一行"));
            continue;
        };
        let needle = norm(line);
        for k in &keys {
            if !needle.contains(&norm(k)) {
                offenders.push(format!("{label}「{marker}」行**漏列** `{k}`"));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "「可改字段」文档落后于 `config set` 实际支持（操作者会以为这些键不可配）：\n{}\n\
         权威清单 = config.rs 的 `可用: …`（共 {} 键：{}）。请把两个文档的清单补齐。",
        offenders.join("\n"),
        keys.len(),
        keys.join(" / ")
    );
}
