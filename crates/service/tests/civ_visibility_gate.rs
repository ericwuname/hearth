//! 门禁：**"会话创建"公告必须写进 API 真正读取的那个 store**（D-108，2026-10-02, traecode）。
//!
//! 背景：读接口 `get_civ_feed` 读 `per_user.civ_for(uid)`（v8.0 多用户隔离后的**唯一
//! 可见** store，落在 `MEMORY_DIR/<uid>/civ.jsonl`）；而 `create_session` 曾写**全局**
//! 文明线 store（`MEMORY_DIR/civilization.jsonl`）——两者是不同文件 ⇒ 公告写进
//! **无人读取的档**，用户 `hearth civ feed` 永远看不到（"写了但不可见"）。
//! 修法是**写侧对齐读侧**（不能反过来让读侧合并全局档：那会把各用户的目标文本
//! 跨租户泄露）。
//!
//! 判据（源码级，宁可漏报不误报）：`routes.rs` 里 `create_session` 函数体内
//! ① 必须出现 `per_user.civ_for`（写进可见档）；② **不得**出现 `civ_store`
//! （写回无人读取的全局档）。
//!
//! 文件头自报盲区：① 只扫 `crates/service/src/routes.rs`，且只取
//! `create_session` 签名起的固定字节窗口（够覆盖本函数、不进入下一个 handler 的实质
//! 代码；故本门禁要求"详细解释写在该签名**上方**的文档注释里"，避免注释里的字样误伤）；
//! ② 只钉 store 选择，不钉条目内容/权限语义。

use std::path::PathBuf;

fn routes_src() -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("routes.rs");
    std::fs::read_to_string(&p).expect("crates/service/src/routes.rs 必须存在")
}

/// 取 `create_session` 函数体的近似片段（签名起 4000 字节窗口）。
fn create_session_window(src: &str) -> String {
    let sig = "pub async fn create_session(";
    let i = src
        .find(sig)
        .expect("create_session 必须存在（改名请同步本门禁）");
    let rest = &src[i..];
    rest[..rest.len().min(4000)].to_string()
}

#[test]
fn civ_announcement_goes_to_visible_per_user_store() {
    let win = create_session_window(&routes_src());

    assert!(
        win.contains("per_user.civ_for"),
        "`create_session` 必须把公告写进 per-user 文明线档（读接口读的就是它）。\
         若改了实现方式，请同步更新本门禁的判据。"
    );
    assert!(
        !win.contains("civ_store"),
        "`create_session` 不得再写**全局** civ store —— 它是另一个文件，API 不读它，\
         条目会永久不可见（D-108 病灶）。若确需改变可见性设计，请连同读侧一起改，\
         并同步更新本门禁。"
    );
}
