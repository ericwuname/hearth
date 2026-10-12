//! D-200（2026-10-08, traecode）源码级门禁：**HTML 实体解码必须单一真相源**。
//!
//! **盲区（自报）**：仅断言源码文本（逐行剥 `//` 注释后）含指定引用/不含指定定义——不执行、
//! 不验证解码语义（语义由单测 `test_d200_search_html_decoding_matches_web_fetch` 与
//! `web.rs::test_strip_html_decodes_entities` 覆盖）。口径「宁可漏报不误报」。
//!
//! 病灶：`web_fetch`（`src/web.rs`）与 `web_search`（`src/search.rs`）曾**各写一份** HTML 实体
//! 解码表、覆盖不同（web 认任意数值引用 + 15 具名；search 只认 10）⇒ 同一段 HTML 在两个出网
//! 工具里清洗结果不同（`&#8217;`/`&copy;` 在 web_fetch 解码、在 web_search 原样泄漏）。
//! 修复＝`search.rs` **委托** `crate::web::decode_entities`（唯一实现），删除本地 `unescape_html`。

use std::fs;
use std::path::PathBuf;

fn code_only(s: &str) -> String {
    s.lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn html_entity_decoding_has_single_source() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let web = code_only(&fs::read_to_string(root.join("src/web.rs")).expect("read src/web.rs"));
    let search =
        code_only(&fs::read_to_string(root.join("src/search.rs")).expect("read src/search.rs"));

    assert!(
        web.contains("pub(crate) fn decode_entities("),
        "D-200：`web.rs` 必须提供 `pub(crate) fn decode_entities`（唯一实体解码实现）"
    );
    assert!(
        search.contains("crate::web::decode_entities("),
        "D-200：`search.rs` 的 HTML 清洗必须**委托** crate::web::decode_entities"
    );
    assert!(
        !search.contains("fn unescape_html("),
        "D-200：`search.rs` 不得再自带实体解码表（`unescape_html`）——与 web_fetch 分叉"
    );
}
