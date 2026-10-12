//! D-198（2026-10-08, traecode）源码级门禁：**受控出网白名单归一化必须单一真相源**。
//!
//! **盲区（自报）**：仅断言源码文本（逐行剥 `//` 注释后）含指定引用——不执行、不验证匹配语义
//! （语义由 `bash.rs`/`webhook.rs` 的 D-198 行为单测与既有 `web.rs`/`search.rs` 单测覆盖）。
//! 口径「宁可漏报不误报」。
//!
//! 病灶：同一份 `HEARTH_EGRESS_ALLOWLIST` 曾在**四处**各自实现匹配、归一化**分叉**
//! （`web_fetch` 去前导点但后缀未小写主机；`web_search` 两侧小写去点；`bash` 不小写不去点；
//! `webhook` 只认 `*.`）⇒ 同一条目在不同工具上判定不同。修复＝收口到 crate 根
//! `pub fn egress_host_allowed`（唯一实现），四处改用之。本门禁把"三处工具侧必须**委托**"
//! 钉死（service 侧 `webhook` 由 `tools_builtin::egress_host_allowed` 委托，另有行为单测）。

use std::fs;
use std::path::PathBuf;

fn code_only(s: &str) -> String {
    s.lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn egress_normalization_has_single_source() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lib = code_only(&fs::read_to_string(root.join("src/lib.rs")).expect("read src/lib.rs"));
    assert!(
        lib.contains("pub fn egress_host_allowed("),
        "D-198：出网白名单归一化必须在 crate 根有**唯一实现** `pub fn egress_host_allowed`"
    );
    for file in ["src/web.rs", "src/search.rs", "src/bash.rs"] {
        let code = code_only(
            &fs::read_to_string(root.join(file)).unwrap_or_else(|e| panic!("读 {file}: {e}")),
        );
        assert!(
            code.contains("crate::egress_host_allowed("),
            "{file} 必须**委托** crate::egress_host_allowed（D-198：不得各自再写一份归一化 ⇒ \
             同一份白名单在不同工具上判定分叉）"
        );
    }
}
