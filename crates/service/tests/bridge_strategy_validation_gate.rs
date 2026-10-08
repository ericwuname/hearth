//! D-177（P1-141）：`POST /api/v1/bridge` 的 **strategy 入参**门禁。
//!
//! 背景（病灶）：`routes::create_bridge` 用
//! ```ignore
//! match body["strategy"].as_str().unwrap_or("round_robin") {
//!     "debate" => Debate, "majority" => MajorityVote, _ => RoundRobin,
//! }
//! ```
//! ⇒ **未知/拼错的策略名被静默当成 `round_robin`**：客户端传 `"voting"` 或 `"debate "`
//! 时，讨论按**错误策略**跑完 3 轮 LLM 调用，客户端**无从知晓**（与 D-164「未知 workline
//! status 静默回落 Pending」同族——本仓已判定"静默回落"为病灶）。
//!
//! 修复口径：键**缺失**才取默认（`round_robin`，文档语义）；键**在**则必须是字符串且属
//! `{round_robin,debate,majority}`，否则 **400 INVALID_PARAM**。
//!
//! 构造手法：把 `OPENAI_BASE_URL` 指向**本机已关闭端口**（`127.0.0.1:1`）⇒ 凡通过入参校验、
//! 真的去跑 bridge 的请求都会因 LLM 连接被拒而**快速**返回 500。于是本套件可**无需真实 LLM**
//! 地断言"该拒的 400 / 该放行的（非 400）"。
//!
//! 盲区（本套件**不**覆盖）：
//!   - 不覆盖"合法策略真的跑通"（那需要真实 LLM）；只锁**入参分类**。
//!   - 不覆盖 `max_rounds`（`SessionManager::create_bridge` 内硬编码 3，非入参）。

use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

struct ServiceGuard(Child);

impl Drop for ServiceGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn wait_healthy(port: u16) -> ServiceGuard {
    let svc = env!("CARGO_BIN_EXE_service");
    let tmp = std::env::temp_dir().join(format!("wf-bridge-strategy-gate-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("create tmp memory dir");
    let child = Command::new(svc)
        .env("PORT", port.to_string())
        .env("MEMORY_DIR", &tmp)
        .env("ALLOW_NO_AUTH", "1")
        .env("OPENAI_API_KEY", "sk-test-placeholder-for-contract-test")
        // 关键：把 provider 端点指向**已关闭端口** ⇒ 真去跑 bridge 的请求**快速** 500。
        .env("OPENAI_BASE_URL", "http://127.0.0.1:1")
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
fn bridge_strategy_is_strictly_validated() {
    let _svc = wait_healthy(3946);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime");
    let base = "http://127.0.0.1:3946";
    let c = reqwest::Client::builder()
        .pool_max_idle_per_host(0)
        .build()
        .expect("build http client");

    let post = |body: serde_json::Value| {
        let c = c.clone();
        rt.block_on(async move {
            let r = c
                .post(format!("{base}/api/v1/bridge"))
                .json(&body)
                .send()
                .await
                .expect("post bridge");
            let st = r.status().as_u16();
            let text = r.text().await.unwrap_or_default();
            (st, text)
        })
    };

    // ① 未知策略名 → 400（修复前：静默回落 round_robin ⇒ 一路跑 bridge ⇒ 500）
    let (s1, b1) = post(serde_json::json!({
        "topic": "t", "participants": ["openai", "openai"], "strategy": "voting"
    }));
    assert_eq!(
        s1, 400,
        "① 未知 strategy 必须 400（修复前被静默当成 round_robin）；实得 {s1}；body={b1}"
    );
    assert!(
        b1.contains("unknown strategy"),
        "① 错误文案应指明 strategy 非法；body={b1}"
    );

    // ② 非字符串 strategy → 400（同 D-164「status must be a string」口径）
    let (s2, b2) = post(serde_json::json!({
        "topic": "t", "participants": ["openai", "openai"], "strategy": 123
    }));
    assert_eq!(
        s2, 400,
        "② 非字符串 strategy 必须 400；实得 {s2}；body={b2}"
    );

    // ③ 键缺失 → 仍取默认 round_robin（**不得**被本次收紧误伤）：请求会去跑 bridge，
    //    因 LLM 端点已关闭而快速 500 —— 关键是**不是 400**。
    let (s3, b3) = post(serde_json::json!({
        "topic": "t", "participants": ["openai", "openai"]
    }));
    assert_ne!(
        s3, 400,
        "③ 缺省 strategy 必须仍被接受（默认 round_robin）——不得误伤；实得 {s3}；body={b3}"
    );

    // ④ 显式合法值 → 仍被接受（同上，非 400）。
    let (s4, _b4) = post(serde_json::json!({
        "topic": "t", "participants": ["openai", "openai"], "strategy": "debate"
    }));
    assert_ne!(s4, 400, "④ 合法 strategy=debate 必须被接受；实得 {s4}");

    eprintln!("D-177 PASS: bridge strategy 已严格校验（未知/非字符串 400，缺省/合法放行）");
}
