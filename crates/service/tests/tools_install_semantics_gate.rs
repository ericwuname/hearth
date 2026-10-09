//! 门禁：**`POST /api/v1/tools/install` 的语义必须如实（"登记清单" 而非 "安装可执行工具"）**（D-167，2026-10-08, traecode）。
//!
//! 背景：`ToolRegistry` 是**清单目录（catalog）**——`install()` 只把 `ToolManifest` 插进内存
//! map，**不落地产物、不执行任何命令**（工具执行走 `ToolDispatcher` + `tools-builtin`，与本
//! 注册表无关）；`ToolManifest.command`/`sha256` **全仓零读取**（D-73 已订正模块文案）。
//! 但 HTTP 端点仍回 `{"status":"installed"}`（201）——对客户端**暗示"装上了可执行的工具"**，
//! 而实际什么也不会执行 ⇒ **声称≠实现**。
//!
//! 为什么不"接线执行"：本端点的 manifest 由**客户端**提供，`command` 是**任意字符串**——
//! 真去执行即把该端点变成**远程命令执行（RCE）**入口，安全上不可接受。故采纳**如实化**：
//! 端点/响应/文档一律表明这是**清单登记**（供 `search`/`tool-registry` 发现），**不安装、不执行**。
//!
//! 断言：① HTTP 响应 `status` 为 `registered`（**不得**是 `installed`）且带**非执行**说明；
//! ② 源码级：`openapi.rs` 的路由摘要须表述为"登记/不执行"，**不得**写"安装工具"。
//!
//! 文件头自报盲区：不断言 `ToolRegistry` 内部实现，也不覆盖 catalog 的持久化语义（本注册表本就仅内存）。

use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

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
fn tools_install_reports_registered_not_executable() {
    let tmp = std::env::temp_dir().join(format!("wf-d167-install-gate-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).expect("create tmp memory dir");

    let _svc = wait_healthy(3953, &tmp);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime");
    let base = "http://127.0.0.1:3953";
    let c = reqwest::Client::builder()
        .pool_max_idle_per_host(0)
        .build()
        .expect("build http client");

    // 客户端提供的 manifest（`command` 为任意字符串——绝不能被执行）。
    let body = serde_json::json!({
        "manifest": {
            "name": "d167-probe",
            "description": "probe tool",
            "version": "0.0.1",
            "sha256": "deadbeef",
            "command": "echo should-never-run",
            "tags": ["probe"]
        }
    });
    let (status, text) = rt.block_on(async {
        let resp = c
            .post(format!("{base}/api/v1/tools/install"))
            .json(&body)
            .send()
            .await
            .expect("send");
        let st = resp.status().as_u16();
        let t = resp.text().await.unwrap_or_default();
        (st, t)
    });
    assert_eq!(status, 201, "首次登记应 201；body={text}");
    let v: serde_json::Value = serde_json::from_str(&text).expect("json");

    // ① 语义如实：不得回 "installed"（暗示可执行）。
    assert_eq!(
        v["status"].as_str(),
        Some("registered"),
        "D-167：install 端点须如实回 status=registered（修复前 \"installed\" 暗示可执行）；body={text}"
    );
    // ①b 带**非执行**说明（把"这不会执行任何命令"写进响应，客户端无从误解）。
    let note = v["note"].as_str().unwrap_or_default();
    assert!(
        note.contains("不执行") || note.contains("不安装"),
        "D-167：响应须带非执行说明（note 含『不执行/不安装』）；body={text}"
    );

    eprintln!("D-167 PASS: /tools/install 如实回 registered + 非执行说明");
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn tools_install_route_summary_is_honest() {
    // ② 源码级：openapi.rs 的路由摘要须表述为"登记/不执行"，不得写"安装工具"。
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("openapi.rs");
    let src = std::fs::read_to_string(&p).expect("openapi.rs 必须存在");
    // 取该路由所在行及其后 2 行（**按行**取窗口——避免按字节切片落进多字节字符边界，
    // 这是本门禁自身的陷阱，勿改回 `&src[idx..idx+n]`）。
    let window = src
        .lines()
        .skip_while(|l| !l.contains("/api/v1/tools/install"))
        .take(3)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !window.is_empty(),
        "openapi.rs 须含 /api/v1/tools/install 路由"
    );
    assert!(
        !window.contains("安装工具"),
        "D-167：路由摘要**不得**写『安装工具』（暗示可执行）；附近文本={window}"
    );
    assert!(
        window.contains("登记"),
        "D-167：路由摘要须如实表述为『登记』（供发现/检索、不执行）；附近文本={window}"
    );
}
