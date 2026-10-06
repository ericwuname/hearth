//! D-164（P1-130）：`PATCH /api/v1/workline/nodes/:id` 的 **PATCH 语义**门禁。
//!
//! 病灶（三处，同属"PATCH 语义不严谨"）：
//!   ① **假成功**：`WorkLineStore::update` 用 `if let Some(n) = nodes.iter_mut().find(..)`，
//!      找不到节点时**直接跳过**并 `flush()` 返回 `Ok(())` ⇒ 对**不存在**的 id PATCH 得 **200**
//!      （CLI `hearth tasks done <坏 id>` 会打印 `marked done`，可什么都没改）。
//!   ② **自相矛盾**：`update(&id, progress: f32, status)` 的 `progress` 是**必填** ⇒ 只改 status
//!      的请求（`{"status":"completed"}`，不传 progress）会把进度**静默重置为 0.0** ⇒
//!      "Completed 但 0%"的矛盾状态（PATCH 的语义本应是**部分更新**）。
//!   ③ **未知枚举静默回落**：`status` 匹配 `_ => WorkStatus::Pending` ⇒ 客户端拼错
//!      （如 `"done"`）被**静默**当成 `Pending`（无报错、无提示）。
//!
//! 修复口径：
//!   ① 节点不存在 → `WorkLineStore::update` 返回"未命中" ⇒ handler 回 **404 NOT_FOUND**；
//!   ② `progress` 改为可选（`Option<f32>`）：**缺失即不改**；
//!   ③ `status` 显式给出但取值非法 → **400 INVALID_PARAM**（合法取值：`pending`/`in_progress`/
//!      `blocked`/`review`/`completed`）；键缺失 = 不改。`progress` 键在但非数字 → 400。
//!
//! 盲区（本套件**不**覆盖）：
//!   - 不覆盖 `create_bridge` 的 `strategy` 同名静默回落（该路径需已注册 provider 才能到达校验）。
//!   - 不覆盖 `notes`/`deps` 等其它字段的部分更新语义。

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
    let tmp = std::env::temp_dir().join(format!("wf-patch-gate-{}", std::process::id()));
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
fn workline_patch_semantics_are_strict() {
    let _svc = wait_healthy(3937);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime");
    let base = "http://127.0.0.1:3937";
    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(0)
        .build()
        .expect("build http client");

    // 建节点 → 设到 50% / in_progress
    let created: serde_json::Value = rt.block_on(async {
        client
            .post(format!("{base}/api/v1/workline/nodes"))
            .json(&serde_json::json!({"description": "d164"}))
            .send()
            .await
            .expect("create")
            .json()
            .await
            .expect("parse create")
    });
    let id = created["id"].as_str().expect("id").to_string();
    let ok: u16 = rt.block_on(async {
        client
            .patch(format!("{base}/api/v1/workline/nodes/{id}"))
            .json(&serde_json::json!({"progress": 0.5, "status": "in_progress"}))
            .send()
            .await
            .expect("seed")
            .status()
            .as_u16()
    });
    assert_eq!(ok, 200, "置位请求应 200");

    // ① 不存在的节点 → 必须 404（修复前：静默跳过 + 200 假成功）
    let r1 = rt.block_on(async {
        client
            .patch(format!("{base}/api/v1/workline/nodes/no-such-node-d164"))
            .json(&serde_json::json!({"progress": 0.9}))
            .send()
            .await
            .expect("patch missing")
    });
    let s1 = r1.status().as_u16();
    let b1 = rt.block_on(r1.text()).unwrap_or_default();
    assert_eq!(
        s1, 404,
        "① 对不存在的节点 PATCH 必须 404（修复前 200 假成功）；body={b1}"
    );

    // ② 未知 status → 必须 400（修复前：静默回落 Pending）
    let r2 = rt.block_on(async {
        client
            .patch(format!("{base}/api/v1/workline/nodes/{id}"))
            .json(&serde_json::json!({"status": "bogus"}))
            .send()
            .await
            .expect("patch bogus status")
    });
    let s2 = r2.status().as_u16();
    let b2 = rt.block_on(r2.text()).unwrap_or_default();
    assert_eq!(
        s2, 400,
        "② 未知 status 必须 400（修复前被静默当成 Pending）；body={b2}"
    );

    // ③ 只改 status（不传 progress）→ 进度必须**保持 0.5**（修复前被静默重置为 0.0）
    let s3: u16 = rt.block_on(async {
        client
            .patch(format!("{base}/api/v1/workline/nodes/{id}"))
            .json(&serde_json::json!({"status": "completed"}))
            .send()
            .await
            .expect("patch status only")
            .status()
            .as_u16()
    });
    assert_eq!(s3, 200, "只改 status 的合法请求应 200");
    let listed: serde_json::Value = rt.block_on(async {
        client
            .get(format!("{base}/api/v1/workline"))
            .send()
            .await
            .expect("get")
            .json()
            .await
            .expect("parse")
    });
    let node = listed["nodes"]
        .as_array()
        .and_then(|a| a.iter().find(|n| n["id"] == serde_json::json!(id)))
        .unwrap_or_else(|| panic!("节点必须还在：{listed}"));
    assert_eq!(
        node["status"], "Completed",
        "status 应已置 Completed；实得 {}",
        node["status"]
    );
    assert_eq!(
        node["progress"].as_f64(),
        Some(0.5),
        "③ 只改 status 不得重置进度（修复前 0.0 ⇒ 'Completed 但 0%' 自相矛盾）；实得 {}",
        node["progress"]
    );

    eprintln!("D-164 PASS: workline PATCH 语义已收紧（404 / 严格枚举 / 部分更新）");
}
