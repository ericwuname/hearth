//! D-192（2026-10-08, traecode）源码级门禁：`web_search` 的出网白名单必须**逐跳复检重定向**。
//!
//! **盲区（自报）**：仅断言**源码文本**（逐行剥 `//` 注释后）含指定构造——不执行网络、不验证
//! 运行时行为。之所以用源码级：`WebSearchTool` 的搜索端点**硬编码**（duckduckgo/bing 常量），
//! 无法把请求指向测试内的 302 服务器 ⇒ **行为侧构造不出 RED**；口径「宁可漏报不误报」。
//!
//! 病灶（同 `web_fetch` 已于 2026-10-01 修复、此处**残留**）：`web_search` 的 `reqwest` 客户端
//! **未设重定向策略** ⇒ 默认**自动跟随最多 10 跳且不复检白名单**。配置 `HEARTH_EGRESS_ALLOWLIST`
//! 时，白名单内源若 302 到 `http://169.254.169.254/…`（云元数据）或 `127.0.0.1:…`（本机服务），
//! 请求**已经发出**（出网治理被一条 302 绕过）。

use std::fs;
use std::path::PathBuf;

/// 逐行剥 `//` 注释（粗略：不解析字符串内的 `//`——本门禁只做「宁可漏报不误报」的粗筛）。
fn code_only(s: &str) -> String {
    s.lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn web_search_rechecks_allowlist_on_redirect() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src = fs::read_to_string(root.join("src/search.rs")).expect("read src/search.rs");
    let code = code_only(&src);
    assert!(
        code.contains(".redirect(reqwest::redirect::Policy::custom"),
        "D-192：web_search 客户端必须设置**自定义重定向策略**（逐跳复检出网白名单）——\
         修复前为 reqwest 默认（自动跟随 10 跳、不复检）"
    );
    assert!(
        code.contains("egress_allowed_open"),
        "D-192：重定向策略内必须复用 `egress_allowed_open` 复检（S2 口径）"
    );
}
