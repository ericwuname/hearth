//! 门禁：CLI 对用户可见命令的**失败响应必须带上服务端错误体**（不得只报裸状态码）
//! （D-151，2026-10-04, traecode）。
//!
//! 背景（真实缺陷）：D-124 引入 `post_ok` 时明确写了它「统一带上状态码 + **有界错误体**」，
//! 但**同一批**的 `cancel_session` 没跟上——它仍是 `bail!("cancel returned {status}")`，
//! **丢掉**服务端返回的 `{"error":{"code","message"}}`。活体可见：
//! `hearth cancel <不存在的 id>` → `Error: cancel returned 404 Not Found`，
//! 而兄弟命令（history/replay/status/approve…）显示 `server error 404 Not Found:
//! {"error":{"code":"SESSION_NOT_FOUND","message":"session not found: …"}}`。
//! 与用户指南 §7「排错（错误可行动——每条报错都含"下一步"）」直接冲突。
//!
//! 判据（文本级，宁可漏报不误报）：`crates/codex-cli/src/client.rs` 中
//! ① **不得**再出现裸状态码报错形态 `"cancel returned "`；② 必须出现把 `error_body_capped`
//! 的结果拼进报错的写法（有界读错误体，防响应体无界入内存）。
//!
//! 文件头自报盲区：① 只钉"已见的那一种"裸报错（`cancel`）；其它未加的写端点若日后
//! 新写裸状态码报错，本尺抓不到；② 不钉错误体的人类可读性/是否真含"下一步"，只钉
//! "服务端的 code/message 有没有被透出"。

use std::path::PathBuf;

#[test]
fn cli_failures_surface_server_error_body() {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("client.rs");
    let raw = std::fs::read_to_string(&p).expect("crates/codex-cli/src/client.rs 必须存在");
    // 逐行剥掉 `//` 注释后再匹配——否则**修复说明注释**里点名旧写法会自命中
    // （D-147 同款教训：注释必须能照常引用"过时写法"，而真正的**代码**一旦回退即报红）。
    let src: String = raw
        .lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        !src.contains("\"cancel returned "),
        "`cancel_session` 又变回**裸状态码**报错（丢掉服务端 `code`/`message`）。\
         应像 `post_ok` 一样：`let body = error_body_capped(resp).await;` 后拼进 bail!。"
    );
    assert!(
        src.contains("error_body_capped"),
        "`client.rs` 必须用 `error_body_capped` 有界读取服务端错误体（防响应体无界入内存）。"
    );
}
