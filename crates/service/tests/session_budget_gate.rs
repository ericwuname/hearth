//! D-163（P1-129）：`POST /api/v1/sessions` 的 `budget.max_steps` 边界门禁。
//!
//! 病灶（护栏被绕过 / 无界族，承 D-160/D-161/D-162 同族）：
//! `agent-runtime::SessionManager::create_session` 用 `req.budget.unwrap_or_default()`——
//! 客户端**自带**的 `budget.max_steps` 被**原样**采纳。而 `Budget::default()`（env 路径）本有
//! 护栏（`HEARTH_MAX_STEPS`，`0`/非法回落 50，注释明写"**护栏不得被配没**"）——
//! API 路径**绕过了它**：
//!   ① 超大值（如 `1e8`）⇒ 服务端会话由后台任务跑、**没有墙钟上限**，`max_steps` 是其**唯一**
//!      时间/成本界 ⇒ 单会话近无界运行（成本/资源 DoS）；
//!   ② `0` ⇒ `is_budget_exhausted()`（`steps_used >= 0`）**开局即耗尽** ⇒ 建出来就是废会话
//!      （与 D-160 的"0 语义"陷阱同型）。
//!
//! 修复口径：`budget` 若显式给出，`max_steps` 必须落在 `1..=MAX_SESSION_STEPS`，否则
//! **400 INVALID_PARAM**；未给 `budget` 时仍走 `Budget::default()`（env 已有护栏）。
//!
//! 盲区（本套件**不**覆盖，勿误当全覆盖）：
//!   - 不覆盖"服务端会话缺**默认墙钟上限**"这一级（CLI 有 `HEARTH_TASK_TIMEOUT_SECS`=900，
//!     服务端未套用）——属独立设计项，见驱动文档债队列 D-163 的"后续选项"。
//!   - 只锁两个代表性边界（超上界 / 0）；`MAX_SESSION_STEPS` 数值若调整须同步本套件。
//!   - 不覆盖 `max_time_secs` / `max_tokens` 的边界（前者=服务端墙钟设计项，后者当前未被 agent 读）。

use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

/// 与 `routes::MAX_SESSION_STEPS` 保持一致（见文件头盲区）。
const MAX_SESSION_STEPS: u64 = 1000;

struct ServiceGuard(Child);

impl Drop for ServiceGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn wait_healthy(port: u16) -> ServiceGuard {
    let svc = env!("CARGO_BIN_EXE_service");
    let tmp = std::env::temp_dir().join(format!("wf-budget-gate-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("create tmp memory dir");
    let child = Command::new(svc)
        .env("PORT", port.to_string())
        .env("MEMORY_DIR", &tmp)
        .env("ALLOW_NO_AUTH", "1")
        .env("OPENAI_API_KEY", "sk-test-placeholder-for-contract-test")
        .env("RUST_LOG", "warn")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn service");
    let guard = ServiceGuard(child);
    for _ in 0..100 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            std::thread::sleep(Duration::from_millis(300));
            return guard;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    panic!("service did not become healthy on port {port}");
}

#[test]
fn session_budget_max_steps_is_bounded() {
    let _svc = wait_healthy(3936);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime");
    let base = "http://127.0.0.1:3936";
    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(0)
        .build()
        .expect("build http client");

    // ① 超过上界 → 400（修复前：原样采纳，201 建出近无界会话）
    let r1 = rt.block_on(async {
        client
            .post(format!("{base}/api/v1/sessions"))
            .json(&serde_json::json!({
                "provider": "openai",
                "goal": "d163 huge",
                "budget": {"max_steps": 100_000_000u64}
            }))
            .send()
            .await
            .expect("send ①")
    });
    let s1 = r1.status().as_u16();
    let b1 = rt.block_on(r1.text()).unwrap_or_default();
    assert_eq!(
        s1, 400,
        "① 超上界的 max_steps 必须 400（修复前被原样采纳）；实得 {s1}；body={b1}"
    );

    // ② `0` → 400（修复前：开局即预算耗尽，建出废会话）
    let r2 = rt.block_on(async {
        client
            .post(format!("{base}/api/v1/sessions"))
            .json(&serde_json::json!({
                "provider": "openai",
                "goal": "d163 zero",
                "budget": {"max_steps": 0}
            }))
            .send()
            .await
            .expect("send ②")
    });
    let s2 = r2.status().as_u16();
    let b2 = rt.block_on(r2.text()).unwrap_or_default();
    assert_eq!(
        s2, 400,
        "② max_steps=0 必须 400（护栏不得被配没 / 否则开局即耗尽）；实得 {s2}；body={b2}"
    );

    // ③ 合法边界值仍必须可用（防修复误伤：上界本身应放行）
    let r3 = rt.block_on(async {
        client
            .post(format!("{base}/api/v1/sessions"))
            .json(&serde_json::json!({
                "provider": "openai",
                "goal": "d163 ok",
                "budget": {"max_steps": MAX_SESSION_STEPS}
            }))
            .send()
            .await
            .expect("send ③")
    });
    assert_eq!(
        r3.status().as_u16(),
        201,
        "③ 上界值（{MAX_SESSION_STEPS}）必须放行（201）"
    );

    eprintln!("D-163 PASS: session budget.max_steps 已被边界校验（1..={MAX_SESSION_STEPS}）");
}
