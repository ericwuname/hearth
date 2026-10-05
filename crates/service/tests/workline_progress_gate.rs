//! D-162（P1-128）：`PATCH /api/v1/workline/nodes/:id` 的 `progress` **有限值**门禁。
//!
//! 病灶（入参边界 / 数据完整性，承 D-161 同族 + D-138「进度不可谎报」主题）：
//! `routes::update_work_node` 用 `body["progress"].as_f64().unwrap_or(0.0) as f32` **原样**采纳——
//! 客户端发一个超出 `f32` 表示范围但仍为有限 `f64` 的数（如 `1e40`）→ `as f32` 溢出为 `inf`
//! → 写进 `WorkNode.progress`。后果有两级：
//!   ① **活体读**：GET 以 `serde_json` 序列化非有限 `f32` 为 **`null`** ⇒ 该节点字段从"数字"变成
//!      `null`，按数字反序列化的客户端直接失败（契约破坏）；
//!   ② **落盘/重载**：`MemoryStore::read_nodes` 对解析失败的行 `if let Ok(..) { }` **静默跳过**
//!      ⇒ 重启后该节点被**静默丢弃**（数据丢失）。
//!
//! 修复口径：`progress` 若转成 `f32` 后非有限 → **400 INVALID_PARAM**；缺失/非数字保持既有
//! 宽松语义（`unwrap_or(0.0)`，不在本卡范围）。
//!
//! 盲区（本套件**不**覆盖）：
//!   - 不校验 `progress` 的**取值范围**（0..1 还是 0..100 无权威口径，本卡只堵"非有限值"）。
//!   - 不覆盖 `description`/`notes` 等其它字段的长度边界。
//!   - 不覆盖"重载丢弃"这一级（需重启 service，由 `read_nodes` 的既有语义代偿）。

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
    let tmp = std::env::temp_dir().join(format!("wf-workline-gate-{}", std::process::id()));
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
fn workline_progress_must_stay_finite() {
    let _svc = wait_healthy(3935);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime");
    let base = "http://127.0.0.1:3935";
    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(0)
        .build()
        .expect("build http client");

    // 建一个节点
    let created = rt.block_on(async {
        client
            .post(format!("{base}/api/v1/workline/nodes"))
            .json(&serde_json::json!({"description": "d162"}))
            .send()
            .await
            .expect("create node")
    });
    assert_eq!(created.status().as_u16(), 201, "建节点应 201");
    let created_body: serde_json::Value = rt.block_on(created.json()).expect("parse create");
    let id = created_body["id"].as_str().expect("id").to_string();

    // 用超出 f32 范围、但仍为有限 f64 的值 patch（1e40 → f32 = inf）
    let patched = rt.block_on(async {
        client
            .patch(format!("{base}/api/v1/workline/nodes/{id}"))
            .json(&serde_json::json!({"progress": 1e40}))
            .send()
            .await
            .expect("patch progress")
    });
    let patch_status = patched.status().as_u16();

    // ① 读回：该节点的 progress 必须仍是**有限数字**（修复前 serde_json 会把 inf 写成 null）
    let listed = rt.block_on(async {
        client
            .get(format!("{base}/api/v1/workline"))
            .send()
            .await
            .expect("get workline")
    });
    let listed_body: serde_json::Value = rt.block_on(listed.json()).expect("parse workline");
    let node = listed_body["nodes"]
        .as_array()
        .and_then(|a| a.iter().find(|n| n["id"] == serde_json::json!(id)))
        .unwrap_or_else(|| panic!("节点必须还在 workline 里：{listed_body}"));
    let progress = &node["progress"];
    assert!(
        progress.as_f64().map(|f| f.is_finite()).unwrap_or(false),
        "progress 必须是有限数字；修复前非有限 f32 会被序列化成 null ⇒ 实得 {progress}"
    );

    // ② 该 patch 本身必须被判为**客户端错误**（400），而不是被采纳
    assert_eq!(
        patch_status, 400,
        "超出 f32 范围的 progress 必须 400 拒绝（修复前被采纳并污染节点）"
    );

    eprintln!("D-162 PASS: workline progress 非有限值已被 400 拒绝");
}
