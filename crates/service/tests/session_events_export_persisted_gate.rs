//! 门禁：**`GET /api/v1/sessions/{id}/events` 对"仅持久化"（重启后）会话必须可导出**（D-174，2026-10-08, traecode）。
//!
//! 背景：`session_events_jsonl` 原**只读内存缓冲**，而 handler 在"缓冲空且不在内存"时
//! 误报 **404 `SESSION_NOT_FOUND`**（会话其实**存在**于持久化 ⇒ 违反 D-166）。后果：**重启后**
//! 对**存在的**会话导出恒 404，磁盘上的录制无从导出。
//!
//! 修复口径：归属校验（`ensure_session_owner`）**已保证会话存在** ⇒ 不得再因"内存缓冲空"
//! 报 404；非存活会话由 `session_events_jsonl` **回落持久化录制**（两态同形状）。
//!
//! 本套件起真实 service，预置一份**仅持久化**会话（meta + 若干事件行），断言：
//! ① `GET /events` → **200**（修复前 404）；
//! ② 导出内容**非空**且每行是合法 JSON（含信封 `type` 字段）；
//! ③ 导出的**事件种类**须覆盖预置的 `phase` / `token`（回落真的读到了持久化录制，
//!    而非返回空串——"200 但空"是对本卡病灶的**假绿**）。
//!
//! 文件头自报盲区：① 不覆盖"完成路径把全量事件流 append 落盘"的写侧（需跑真实会话到
//! 完成，见 `agent-runtime` 侧单测 / 既有 P5 A2 集成路径）；② 预置行用的就是存储格式，
//! 不覆盖真实运行期写入的字段变体。

use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

/// 仅持久化、未被加载的会话 id（预置 JSONL 文件名须等于该 id）。
const PERSISTED_ID: &str = "d174-persisted";

struct ServiceGuard(Child);

impl Drop for ServiceGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn wait_healthy(port: u16, tmp: &std::path::Path) -> ServiceGuard {
    let svc = env!("CARGO_BIN_EXE_service");
    let child = Command::new(svc)
        .env("PORT", port.to_string())
        .env("MEMORY_DIR", tmp)
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
fn persisted_session_events_are_exportable() {
    let tmp = std::env::temp_dir().join(format!("wf-d174-export-gate-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).expect("create tmp memory dir");

    // 预置仅持久化会话：meta + 两条**真实 AgentEvent 载荷**的事件（phase / token）。
    let meta = format!(
        "{{\"type\":\"session_meta\",\"session_id\":\"{PERSISTED_ID}\",\"provider_name\":\"openai\",\
         \"model\":\"gpt-4o\",\"goal\":\"g\",\"owner\":\"default\",\
         \"created_at\":\"2026-01-01T00:00:00Z\"}}\n"
    );
    let ev0 = "{\"type\":\"event\",\"seq\":0,\"event_type\":\"phase\",\
               \"payload\":{\"type\":\"phase\",\"phase\":\"Run\"},\
               \"timestamp\":\"2026-01-01T00:00:00Z\"}\n";
    let ev1 = "{\"type\":\"event\",\"seq\":1,\"event_type\":\"token\",\
               \"payload\":{\"type\":\"token\",\"delta\":\"hi\"},\
               \"timestamp\":\"2026-01-01T00:00:01Z\"}\n";
    std::fs::write(
        tmp.join(format!("{PERSISTED_ID}.jsonl")),
        format!("{meta}{ev0}{ev1}"),
    )
    .expect("seed persisted session");

    let _svc = wait_healthy(3952, &tmp);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime");
    let base = "http://127.0.0.1:3952";
    let c = reqwest::Client::builder()
        .pool_max_idle_per_host(0)
        .build()
        .expect("build http client");

    let (status, body) = rt.block_on(async {
        let resp = c
            .get(format!("{base}/api/v1/sessions/{PERSISTED_ID}/events"))
            .send()
            .await
            .expect("send");
        let st = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        (st, text)
    });

    // ① 存在即应可导出（修复前 404）。
    assert_eq!(
        status, 200,
        "仅持久化会话（重启后）的事件导出应 200（修复前 404 SESSION_NOT_FOUND）；body={body}"
    );

    // ② 每行须是合法 JSON 信封。
    let lines: Vec<&str> = body.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(
        !lines.is_empty(),
        "导出不得为空——否则是对本卡病灶的假绿（200 但无内容）"
    );
    for l in &lines {
        let v: serde_json::Value =
            serde_json::from_str(l).unwrap_or_else(|e| panic!("导出行须为合法 JSON：{e}：{l}"));
        assert!(v.get("type").is_some(), "每行须带信封 `type` 字段：{l}");
    }

    // ③ 回落**真的读到了持久化录制**：事件种类须覆盖预置的 phase / token。
    assert!(
        body.contains("phase") && body.contains("token"),
        "导出须包含预置的 phase / token 事件（回落未读到持久化录制）；body={body}"
    );

    eprintln!("D-174 PASS: 仅持久化会话的事件导出已可回落持久化录制（200，含 phase/token）");
    let _ = std::fs::remove_dir_all(&tmp);
}
