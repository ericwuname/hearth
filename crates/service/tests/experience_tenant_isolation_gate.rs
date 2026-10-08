//! D-175（P1-139）：**经验库的多租户隔离**门禁。
//!
//! 背景（病灶）：经验库是**进程级单例**（`main.rs` 一个 `ExperienceStore` → `MEMORY_DIR/
//! experience.jsonl`），而 agent-loop 每次 run 收尾都会 append 一条经验，其中
//! `problem = <会话目标文本>`（`agent-core/loop.rs`）。⇒ D-169 打开 `HEARTH_USERS` 多租户后，
//! **所有租户的目标文本落进同一文件**（派生学习制品未按租户分区）；若开 `HEARTH_EXPERIENCE_REUSE=1`
//! 还会把他人失败教训注入本租户 prompt（跨租户注入）。业界口径（联网核实）：派生数据/向量库/
//! 长期记忆**必须**按租户命名空间隔离。
//!
//! 修复口径（镜像 D-109 的 civ 写入器工厂）：新增 `PerUserStore::experience_for(uid)`
//! （`MEMORY_DIR/<uid>/experience.jsonl`）+ `SessionManager::set_experience_factory`，会话按
//! owner 注入；`GET /api/v1/experience/metrics` 亦按 uid 读。单租户（uid 恒 `default`）不变。
//!
//! 断言：①/② 预置**不同条数**的分区文件后，alice 与 bob 各自只看到**自己**的指标
//! （修复前两者都读全局单例 = 跨租户）；③ 源码级钉住组合根**注册了工厂**（会话写入面——
//! 该面需真跑一次 agent 才能端到端验证，本套件不含 LLM，故用源码级断言兜底，见盲区）。
//!
//! 盲区（本套件**不**覆盖，勿误当全覆盖）：
//!   - **不**端到端跑 agent 验证"会话收尾写入落到本租户档"（需真实 LLM）——由 ③ 的源码级
//!     断言 + `per_user.rs` 单测 `test_d175_experience_is_per_user` 覆盖。
//!   - 不覆盖"未设 `HEARTH_USERS` 时单租户行为不变"（由既有测试与 ③ 兜底）。

use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

const KEY_ALICE: &str = "k-alice";
const KEY_BOB: &str = "k-bob";

struct ServiceGuard(Child);

impl Drop for ServiceGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// 一条合法 `Experience` 的 JSONL 行（字段与 `experience::Experience` 一致）。
fn exp_line(id: &str) -> String {
    format!(
        "{{\"id\":\"{id}\",\"category\":\"failure\",\"problem\":\"{id}-goal\",\"solution\":\"s\",\
         \"success\":false,\"effectiveness\":0.6,\"created_at\":\"2026-01-01T00:00:00Z\"}}"
    )
}

fn seed_file(path: &std::path::Path, n: usize, tag: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create seeded dir");
    }
    let mut body = String::new();
    for i in 0..n {
        body.push_str(&exp_line(&format!("{tag}-{i}")));
        body.push('\n');
    }
    std::fs::write(path, body).expect("seed experience file");
}

fn wait_healthy(port: u16, tmp: &std::path::Path) -> ServiceGuard {
    let svc = env!("CARGO_BIN_EXE_service");
    let child = Command::new(svc)
        .env("PORT", port.to_string())
        .env("MEMORY_DIR", tmp)
        .env("ALLOW_NO_AUTH", "1")
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

#[test]
fn experience_metrics_is_tenant_scoped() {
    let tmp = std::env::temp_dir().join(format!("wf-exp-iso-gate-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).expect("create tmp memory dir");

    // ① 预置：全局档 3 条（修复前所有租户都读它）；alice 档 1 条；bob 档 2 条。
    seed_file(&tmp.join("experience.jsonl"), 3, "global");
    seed_file(&tmp.join("alice").join("experience.jsonl"), 1, "alice");
    seed_file(&tmp.join("bob").join("experience.jsonl"), 2, "bob");

    let _svc = wait_healthy(3944, &tmp);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime");
    let base = "http://127.0.0.1:3944";
    let c = reqwest::Client::builder()
        .pool_max_idle_per_host(0)
        .build()
        .expect("build http client");

    let total_for = |key: &str| {
        let key = key.to_string();
        let c = c.clone();
        rt.block_on(async move {
            let r = c
                .get(format!("{base}/api/v1/experience/metrics"))
                .header("Authorization", format!("Bearer {key}"))
                .send()
                .await
                .expect("get metrics");
            assert_eq!(r.status().as_u16(), 200, "metrics 应 200");
            let v: serde_json::Value = r.json().await.expect("metrics json");
            v["total_experiences"].as_u64().unwrap_or(u64::MAX)
        })
    };

    let alice_total = total_for(KEY_ALICE);
    let bob_total = total_for(KEY_BOB);

    assert_eq!(
        alice_total, 1,
        "① **跨租户**：alice 只应看到自己档里的 1 条（修复前读到全局单例 = 3 条）"
    );
    assert_eq!(
        bob_total, 2,
        "② **跨租户**：bob 只应看到自己档里的 2 条（修复前读到全局单例 = 3 条）"
    );
    assert_ne!(
        alice_total, bob_total,
        "② 两租户指标必须可区分（修复前都等于全局单例的同一数值）"
    );

    // ③ 源码级：组合根必须**注册经验库工厂**（会话写入面按 owner 分档）。
    let main_rs = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/main.rs"))
        .expect("read main.rs");
    assert!(
        main_rs.contains("set_experience_factory"),
        "③ 组合根必须注册 `set_experience_factory`（否则会话收尾写入仍落全局单例 = 跨租户）"
    );

    eprintln!("D-175 PASS: 经验库按租户分区（metrics 与写入面均按 uid）");
    let _ = std::fs::remove_dir_all(&tmp);
}
