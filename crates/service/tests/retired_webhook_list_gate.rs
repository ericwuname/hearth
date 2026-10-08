//! D-180（P1-144）：**退役 `WebhookManager::list()`**（零调用方死物）——源码级 pin 门禁。
//!
//! 背景（病灶，D-78/D-111 同族"零调用方死物"）：`WebhookManager::list()` 全仓**零调用方**
//! ——
//!   · 生产侧：`routes::register_webhook` 只用 `try_register`；`create_session` 只用
//!     `fire_event`；`main.rs` 只用 `WebhookManager::new()`。
//!   · 测试侧：`webhook.rs` 单测只覆盖 `validate_webhook_url` / `select_hooks`；
//!     `service/tests/*` 两个 webhook 套件均走 HTTP，不调 `list()`。
//!   ⇒ 它是一个**只有定义、没有任何读取方**的 `pub` 方法（webhook 无 `GET` 读端点）。
//!   按 D-111/D-78 纪律**删除**（避免成为"看似有 API、实则无人用"的漂移陷阱）。
//!
//! 本门禁是**源码级**（不启动 service）：读 `src/webhook.rs`、逐行剥 `//` 注释后断言
//! `pub fn list(` **不再存在**——即"退役不得回潮"。**先红后绿**：删前该断言为红（方法仍在）。
//!
//! 盲区（本套件**不**覆盖，勿误当全覆盖）：
//!   - 只 pin 本方法的"存在性"；**不**pin 其调用方计数（源码级做调用图不现实）。
//!   - 剥注释用行级 `//`（与既有源码级门禁同口径）；若有人把该方法写进**字符串字面量**
//!     以规避，本门禁会漏报（宁可漏报不误报）。

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
fn webhook_manager_list_stays_retired() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/webhook.rs");
    let src = std::fs::read_to_string(path).expect("read webhook.rs");
    let code = code_only(&src);

    assert!(
        !code.contains("pub fn list("),
        "D-180 退役回潮：`WebhookManager::list()` 又出现了——它是**零调用方死物**，\
         不得重新引入（若确有读取需求，请先接线到消费点并立卡说明）。"
    );

    // 反向对照：本次退役**不得**误删真正在用者——`try_register` / `fire_event` 必须仍在。
    assert!(
        code.contains("pub fn try_register("),
        "对照失败：`try_register` 是注册面唯一入口，必须保留"
    );
    assert!(
        code.contains("pub async fn fire_event("),
        "对照失败：`fire_event` 是投递面唯一入口，必须保留"
    );
}
