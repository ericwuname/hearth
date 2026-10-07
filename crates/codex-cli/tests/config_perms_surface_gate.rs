//! D-168（P1-133）：config 落盘的**权限收紧不得静默失败**门禁。
//!
//! 病灶（静默失败 / 安全属性静默失效，承 D-124/D-134/D-135 同族）：
//! `codex-cli/src/config.rs::Config::save()` 在 unix 上把 `config.toml` 权限收紧到 `0600`
//!（该文件**可含** `api_key` / `provider_keys` 等密钥），但用的是 `let _ = std::fs::set_permissions(..)`
//! ——**失败被静默丢弃**：chmod 不生效时文件可能仍为 umask 默认（如 `0644`），
//! 密钥对**同机其他用户可读**，而用户**看不到任何提示**。
//!
//! 修复口径：**不得丢弃**该 `Result`——失败须**上抛**（带可行动文案：配置文件已写入、
//! 但权限未能收紧、请手动 `chmod 600`）。
//!
//! 盲区（本套件**不**覆盖，勿误当全覆盖）：
//!   - 这是**源码级**锁：不覆盖"chmod 真的失败时 CLI 行为"的运行时验证——本机为 Windows
//!     （`#[cfg(unix)]` 分支不参与编译/运行），无法确定性构造 chmod 失败的红侧。
//!   - 不覆盖 Windows 侧（依赖用户目录 ACL，无 chmod 语义）。

use std::path::PathBuf;

/// 读源码并**逐行剥 `//` 注释**——避免"注释里点名旧写法 ⇒ 自命中"（D-147/151/153 反复踩到）。
fn read_stripped(rel: &str) -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    let raw = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("读不到 {}：{e}", p.display()));
    raw.lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn config_permission_hardening_is_not_silently_discarded() {
    let src = read_stripped("src/config.rs");

    assert!(
        src.contains("set_permissions"),
        "config 落盘必须**保留** 0600 权限收紧（不许以'删掉收紧'的方式绕过本门禁）"
    );
    assert!(
        !src.contains("let _ = std::fs::set_permissions"),
        "`set_permissions` 的 Result **不得**用 `let _ =` 丢弃——失败会让含密钥的 config.toml \
         保持 umask 默认权限（可能对他用户可读）而**无任何提示**（D-168）。请改为上抛（带可行动文案）。"
    );
    assert!(
        !src.contains("let _ = std::fs::set_permissions("),
        "同上门禁（带括号写法）。"
    );
}
