//! D-169（P1-134）：**多租户隔离**配置面门禁（`HEARTH_USERS`）。
//!
//! 背景（病灶）：`main.rs` 的 `UserStore::new(&[])` **恒为空** + 仅 `register(api_key,"default")`
//! ⇒ 全仓只会产生 `"default"` 一个 uid，per-user 档（`MEMORY_DIR/<uid>/civ.jsonl`）恒指向同一份
//! —— D-108/D-109 以"跨租户泄露"为由拒绝合并全局档的那条隔离，**运行期并不存在**。
//!
//! 修复口径（已在驱动文档 D-169 登记设计后接线）：
//!   ① 新增 `HEARTH_USERS="key1=alice,key2=bob"`（**未设 ⇒ 行为与今完全相同**）；
//!   ② 鉴权改为"命中**任一**已注册 key 即通过"（逐 key **常数时间**比对，不早退）；
//!   ③ per-user 档按 uid 隔离 —— 本套件即验证 ③。
//!
//! 盲区（本套件**不**覆盖）：
//!   - 不覆盖"未设 `HEARTH_USERS` 时行为不变"的回归（由既有鉴权测试 + 门禁四件套覆盖）。
//!   - 不覆盖时序侧信道（常数时间比对只在源码层复核，不做统计检验）。

use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

/// 与测试用 env 一致的两个租户 key。
const KEY_ALICE: &str = "k-alice";
const KEY_BOB: &str = "k-bob";

struct ServiceGuard(Child);

impl Drop for ServiceGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn wait_healthy(port: u16) -> ServiceGuard {
    let svc = env!("CARGO_BIN_EXE_service");
    let tmp = std::env::temp_dir().join(format!("wf-mt-gate-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("create tmp memory dir");
    let child = Command::new(svc)
        .env("PORT", port.to_string())
        .env("MEMORY_DIR", &tmp)
        // 只用 ALLOW_NO_AUTH 让进程**绑回环**（便于测试）；鉴权仍因"已配置 key"而被要求。
        .env("ALLOW_NO_AUTH", "1")
        // D-169：多租户配置面——两个 key → 两个独立 uid。
        .env("HEARTH_USERS", format!("{KEY_ALICE}=alice,{KEY_BOB}=bob"))
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

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .pool_max_idle_per_host(0)
        .build()
        .expect("build http client")
}

#[test]
fn hearth_users_isolates_per_tenant_stores() {
    let _svc = wait_healthy(3939);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime");
    let base = "http://127.0.0.1:3939";
    let c = client();

    // ① alice 用她的 key 写一条文明线公告 —— 期望 201（修复前：整站 401，见下 ④）
    let marker = "alice-only-marker-d169";
    let (s1, b1) = rt.block_on(async {
        let r = c
            .post(format!("{base}/api/v1/civilization"))
            .header("Authorization", format!("Bearer {KEY_ALICE}"))
            .json(&serde_json::json!({ "content": marker }))
            .send()
            .await
            .expect("alice post");
        let st = r.status().as_u16();
        (st, r.text().await.unwrap_or_default())
    });
    assert_eq!(
        s1, 201,
        "① alice 的 key 应被接受（修复前 HEARTH_USERS 未被识别 ⇒ 整站 401）；body={b1}"
    );

    // ② alice 自己看得到
    let alice_view: String = rt.block_on(async {
        c.get(format!("{base}/api/v1/civilization"))
            .header("Authorization", format!("Bearer {KEY_ALICE}"))
            .send()
            .await
            .expect("alice get")
            .text()
            .await
            .unwrap_or_default()
    });
    assert!(
        alice_view.contains(marker),
        "② alice 应看到自己的公告；实得 {alice_view}"
    );

    // ③ bob 看**不到** alice 的（跨租户隔离）
    let bob_view: String = rt.block_on(async {
        c.get(format!("{base}/api/v1/civilization"))
            .header("Authorization", format!("Bearer {KEY_BOB}"))
            .send()
            .await
            .expect("bob get")
            .text()
            .await
            .unwrap_or_default()
    });
    assert!(
        !bob_view.contains(marker),
        "③ **跨租户泄露**：bob 不应看到 alice 的公告（per-user 档必须按 uid 隔离）；实得 {bob_view}"
    );

    // ④ 无凭据 → 401；错误凭据 → 403（既有语义不得被本次改动破坏）
    let s_nokey = rt.block_on(async {
        c.get(format!("{base}/api/v1/civilization"))
            .send()
            .await
            .expect("no key")
            .status()
            .as_u16()
    });
    assert_eq!(s_nokey, 401, "④ 无凭据应 401");

    let s_badkey = rt.block_on(async {
        c.get(format!("{base}/api/v1/civilization"))
            .header("Authorization", "Bearer definitely-wrong-key")
            .send()
            .await
            .expect("bad key")
            .status()
            .as_u16()
    });
    assert_eq!(s_badkey, 403, "④ 错误凭据应 403");

    eprintln!("D-169 PASS: HEARTH_USERS 生效且 per-user 档按 uid 隔离");
}
