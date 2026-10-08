//! D-184（P1-147）：CLI `list_sessions` 对响应**形状不符**不得静默回落成空列表 —— 源码级门禁。
//!
//! 背景（病灶，"静默回落"族，承 D-164「未知 status 静默回落 Pending」/ D-177「未知 strategy
//! 静默回落 round_robin」）：`client.rs::list_sessions` 原写作
//! ```ignore
//! let sessions: Vec<SessionInfo> =
//!     serde_json::from_value(body["sessions"].clone()).unwrap_or_default();
//! ```
//! ⇒ 一旦 `/api/v1/sessions` 的响应形状变了（或 `sessions` 缺失/类型不符），解析失败被**静默吞掉**、
//! 回落为空列表：用户在 `hearth repl` 的 `/sessions` 看到 `(no active sessions)`，**服务端其实有会话**
//! ——把"契约漂移/解析失败"伪装成"没有数据"。而调用方**早已备好** `Err` 分支
//! （`repl.rs`：`Err(e) => render::error(...)`）——缺的只是客户端**从不产出**该错误。
//!
//! 修复口径：抽出纯函数 `parse_sessions(&Value) -> Result<Vec<SessionInfo>>`，形状不符**显式上抛**
//! （`context`），调用方原样呈现。
//!
//! 本门禁是**源码级**（修复前后都能编译）：读 `src/client.rs`、逐行剥 `//` 注释后断言
//! ① 存在 `fn parse_sessions(`；② 旧的静默回落写法**不再存在**（防回潮）。
//!
//! 盲区（本套件**不**覆盖）：
//!   - 只 pin"存在显式解析 + 旧写法已除"；**不**断言运行时行为（那由 `client.rs` 的单测覆盖）。
//!   - 剥注释用行级 `//`（与既有源码级门禁同口径）。

/// 读源文件并逐行剥掉 `//` 之后的注释（保留代码）。
fn code_only(src: &str) -> String {
    src.lines()
        .map(|l| match l.find("//") {
            Some(i) => &l[..i],
            None => l,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn sessions_parse_does_not_silently_fall_back() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/client.rs");
    let src = std::fs::read_to_string(path).expect("read client.rs");
    let code = code_only(&src);

    assert!(
        code.contains("fn parse_sessions("),
        "D-184：`client.rs` 必须有显式的 `parse_sessions`（形状不符时可上抛）——\
         否则解析失败会被静默回落成空列表，用户看到 `(no active sessions)` 而服务端其实有会话"
    );
    assert!(
        !code.contains(r#"serde_json::from_value(body["sessions"].clone()).unwrap_or_default()"#),
        "D-184 回潮：`list_sessions` 又用 `unwrap_or_default()` 吞掉解析失败了——\
         形状不符必须显式上抛（调用方已有 Err 分支）"
    );
}
