//! D-201（2026-10-08, traecode）源码级门禁：`hearth config get` / `config set` 的**字段面**
//! 必须与 `ResolvedConfig`/`Config` 对齐、且**别名对称**。
//!
//! **盲区（自报）**：仅断言源码文本（逐行剥 `//` 注释后）含指定字段/别名——不执行、不验证
//! 打印内容（`get` 在命令处理大函数里、`set` 直接落盘，均无纯函数可驱动；口径「宁可漏报不误报」）。
//!
//! 病灶：
//! ① `set_field`（`config.rs`）只认 `read-roots`，而 `config get`（`lib.rs`）认
//!    `read-roots`/`read_roots` 两种写法 ⇒ 同一命名规律"一半能设、一半报未知字段"；
//! ② `providers`（S9 降级链，`ResolvedConfig.providers` 已生效）在 `config get`/`set`
//!    里**完全缺席** ⇒ 用户在 CLI 看不到、也改不了已生效的降级链（用户可见面落后于结构体）。

use std::fs;
use std::path::PathBuf;

fn code_only(s: &str) -> String {
    s.lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn config_get_set_field_surface_is_symmetric_and_covers_providers() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cfg = code_only(&fs::read_to_string(root.join("src/config.rs")).expect("read config.rs"));
    let lib = code_only(&fs::read_to_string(root.join("src/lib.rs")).expect("read lib.rs"));

    // ① read-roots 别名对称：set(config.rs) 与 get(lib.rs) 都必须认连字符与下划线两种写法。
    for (file, code) in [("src/config.rs", &cfg), ("src/lib.rs", &lib)] {
        assert!(
            code.contains("\"read_roots\""),
            "{file} 缺 `read-roots` 的**下划线别名** `\"read_roots\"`——get/set 别名不对称（D-105 声称已对齐）"
        );
    }
    // ② providers 覆盖：get 与 set 都必须覆盖 `providers`（降级链）。
    for (file, code) in [("src/config.rs", &cfg), ("src/lib.rs", &lib)] {
        assert!(
            code.contains("\"providers\""),
            "{file} 缺 `providers` 字段支持——降级链在 CLI 不可见/不可改（用户可见面落后于结构体）"
        );
    }
}
