//! D-185（P1-148）：**退役 `api::InteractionRequest`**（零构造 / 零引用 / 未入契约）——源码级 pin 门禁。
//!
//! 背景（病灶，D-78/D-111 同族"只有定义、无任何消费者"）：`api::InteractionRequest`
//! （`id`/`kind`/`blocking`/`timeout`/`on_timeout`/`payload`）**全仓零构造、零读取**——
//!   · 生产侧：内核发出的人类介入点是 **SSE 事件** `Event::InteractionRequested { id, kind, … }`
//!     （`agent-core`），服务端 `POST …/interaction/{iid}` 只反序列化 **`InteractionResponse`**；
//!   · 契约侧：`ApiDoc` 的 `components(schemas(…))`（`main.rs`）**未列出**它；
//!   · 其余命中全在注释 / 历史计划文档（`docs/top-level-plan-v23.md`）里**提及名字**，非引用。
//!   ⇒ 它与 `InteractionResponse` 是**不对称**的：响应型在用、请求型从未落地（内核用事件代替了它）。
//!   按 D-111/D-78 纪律**删除**（避免"看似有契约、实则无人用"的漂移陷阱）。
//!
//! 本门禁是**源码级**（不启动任何东西，修复前后都能编译）：读 `src/lib.rs`、逐行剥 `//`
//! 注释后断言 ① `pub struct InteractionRequest` **不存在**；② **反向对照**
//! `pub struct InteractionResponse` **仍存在**（防误删在用的兄弟类型——它是
//! `POST /api/v1/sessions/{id}/interaction/{iid}` 的请求体）。
//!
//! 盲区（本套件**不**覆盖，勿误当全覆盖）：
//!   - 只 pin 该类型名不再出现；**不**做调用图分析（源码级做引用计数不现实）。
//!   - 剥注释用行级 `//`（与既有源码级门禁同口径）；文档注释 `///` 会被剥掉 `//` 之后的部分，
//!     因此"仅在文档里提及"不会造成误报——这正是本门禁想要的：**只认代码**。

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
fn interaction_request_type_stays_retired() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs");
    let src = std::fs::read_to_string(path).expect("read api/src/lib.rs");
    let code = code_only(&src);

    assert!(
        !code.contains("pub struct InteractionRequest"),
        "D-185 退役回潮：`api::InteractionRequest` 又出现了——它**零构造/零引用**且**未入契约**\
         （内核用 SSE 事件 `InteractionRequested` 代替了它）。若确有需求，请先让**消费者接线**\
         并讲清它与 `InteractionResponse` 的不对称关系，再立卡加回。"
    );
    assert!(
        code.contains("pub struct InteractionResponse"),
        "对照失败：`InteractionResponse` 是 `POST …/interaction/{{iid}}` 的**在用请求体**，必须保留"
    );
}
