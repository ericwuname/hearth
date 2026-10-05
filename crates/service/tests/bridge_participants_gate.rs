//! D-161（P1-127）：`POST /api/v1/bridge` 的**参与者入参边界**门禁。
//!
//! 病灶（无界扇出 + 错误分类错误，承"无界"/B2 族）：
//!   ① `routes::create_bridge` 只校验 `participants.len() >= 2`，**无上界**；而
//!      `bridge::BridgeSession` 对每个参与者每轮发一次 LLM 调用（`max_rounds=3` 固定）
//!      ⇒ 一份体（2 MiB 上限）里塞 70 万条同名已注册 provider，即可触发约 **210 万**次
//!      顺序 LLM 调用（成本/资源 DoS，认证客户端亦可为）。
//!   ② 参与者名字**不校验是否已注册**——`registry.get()` 失败被 `map_err` 一律映射为
//!      **500 INTERNAL**：客户端拼错 provider 名（客户端错误）被冒充成**服务端故障**。
//!
//! 修复口径：把 `participants` 当**不可信入参**在边界校验——`2 <= len <= MAX_BRIDGE_PARTICIPANTS`
//! 且每个名字都能在 registry 解析（用与 bridge 相同的 `registry.get` 语义，避免误拒别名），
//! 任一不满足 → **400 INVALID_PARAM**（统一 JSON 错误形状，见 D-156）。
//!
//! 盲区（本套件**不**覆盖）：
//!   - 不覆盖 `max_rounds`（`SessionManager::create_bridge` 内硬编码 3，非入参）。
//!   - 不覆盖"合法参与者但 LLM 调用真的失败"的路径（那是 5xx，属服务端故障）。
//!   - 只锁两个代表性拒绝；`MAX_BRIDGE_PARTICIPANTS` 的具体数值若调整，本套件须同步。

use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

/// 与 `routes::MAX_BRIDGE_PARTICIPANTS` 保持一致（见文件头盲区）。
const MAX_BRIDGE_PARTICIPANTS: usize = 8;

struct ServiceGuard(Child);

impl Drop for ServiceGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn wait_healthy(port: u16) -> ServiceGuard {
    let svc = env!("CARGO_BIN_EXE_service");
    let tmp = std::env::temp_dir().join(format!("wf-bridge-gate-{}", std::process::id()));
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

/// 断言响应是 400 + 统一 JSON 错误形状。
async fn assert_bad_request(what: &str, resp: reqwest::Response) {
    let status = resp.status();
    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let body = resp.text().await.expect("read body");
    assert_eq!(
        status.as_u16(),
        400,
        "[{what}] 应 400（客户端错误），实得 {status}；body={body}"
    );
    assert!(
        ct.contains("application/json"),
        "[{what}] 400 应为统一 JSON 错误，实得 Content-Type {ct:?}；body={body}"
    );
    let v: serde_json::Value =
        serde_json::from_str(&body).unwrap_or_else(|e| panic!("[{what}] 体应为 JSON: {e}; {body}"));
    assert!(
        v["error"]["code"].is_string(),
        "[{what}] 缺 error.code：{body}"
    );
}

#[test]
fn bridge_participants_are_bounded_and_validated() {
    let _svc = wait_healthy(3934);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime");
    let base = "http://127.0.0.1:3934";
    // 关空闲连接复用：避免服务端在任何一段关连接时污染后续请求。
    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(0)
        .build()
        .expect("build http client");

    // ① 参与者数量超上界 → 400（修复前：无人校验长度，请求一路进 bridge，最终 500）
    let many: Vec<String> = vec!["nope-not-a-provider".to_string(); MAX_BRIDGE_PARTICIPANTS + 1];
    let body_many = serde_json::json!({"topic": "t", "participants": many});
    let r1 = rt.block_on(async {
        client
            .post(format!("{base}/api/v1/bridge"))
            .json(&body_many)
            .send()
            .await
            .expect("send ①")
    });
    rt.block_on(assert_bad_request("①too many participants", r1));

    // ② 参与者名未注册 → 400（修复前：被 map_err 一律映射成 500 INTERNAL）
    let body_unknown =
        serde_json::json!({"topic": "t", "participants": ["openai", "definitely-not-a-provider"]});
    let r2 = rt.block_on(async {
        client
            .post(format!("{base}/api/v1/bridge"))
            .json(&body_unknown)
            .send()
            .await
            .expect("send ②")
    });
    rt.block_on(assert_bad_request("②unknown participant", r2));

    eprintln!("D-161 PASS: bridge 参与者入参已被边界校验（上界 + 已注册）");
}
