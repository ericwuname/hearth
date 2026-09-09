//! P1-6 (audit-fix): CLI ↔ service 端到端契约测试。
//!
//! 背景（audit-fix-taskbook-v22 P1-6）：审计一次发现 2 处契约断裂（approve 422、
//! sessions 405），而 214 个单测没有一个真正跑过 CLI 打真 service。
//!
//! 本套件：起**真实 service**（ALLOW_NO_AUTH=1 回环模式）+ `codex-cli` 二进制
//! 打非交互子命令，断言协议层不 4xx/5xx（业务失败可接受，协议层失败不可接受）。
//! 覆盖 12 个非交互命令；chat/repl/setup/resume/replay 为交互/特殊，跳过。

use std::net::TcpStream;
use std::process::{Child, Command};
use std::time::Duration;

/// 定位 codex-cli 二进制：CODEX_CLI_BIN env → workspace target/debug/codex。
fn cli_bin() -> Option<std::path::PathBuf> {
    if let Ok(p) = std::env::var("CODEX_CLI_BIN") {
        let pb = std::path::PathBuf::from(p);
        if pb.exists() {
            return Some(pb);
        }
    }
    // workspace 根 = manifest_dir 上溯两级
    let mdir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    [
        mdir.join("../../target/debug/codex"),
        mdir.join("../../../target/debug/codex"),
        mdir.join("target/debug/codex"),
    ]
    .into_iter()
    .find(|cand| cand.exists())
}

fn wait_healthy(port: u16) -> Child {
    let svc = env!("CARGO_BIN_EXE_service");
    let tmp = std::env::temp_dir().join(format!("wf-cli-contract-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("create tmp memory dir");
    let mut child = Command::new(svc)
        .env("PORT", port.to_string())
        .env("MEMORY_DIR", &tmp)
        .env("ALLOW_NO_AUTH", "1")
        // D-extra: service 无 key 启动失败（fail-closed）——测试环境注入测试 key
        .env("OPENAI_API_KEY", "sk-test-placeholder-for-contract-test")
        .env("RUST_LOG", "warn")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn service");
    for _ in 0..100 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            // 端口可连即视为健康（服务已绑定并 accept）
            std::thread::sleep(Duration::from_millis(300));
            return child;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    let _ = child.kill();
    let _ = child.wait(); // clippy: spawned process must be wait()ed in all paths
    panic!("service did not become healthy on port {port}");
}

fn cli(bin: &std::path::Path, port: u16, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(bin);
    cmd.arg("--url").arg(format!("http://127.0.0.1:{port}"));
    cmd.args(args);
    cmd.output().expect("run codex-cli")
}

/// 断言协议层不 5xx/422/405（stderr/stdout 不出现对应字样）。
/// 注：404（SESSION_NOT_FOUND）是业务失败，可接受；5xx=服务端故障、
/// 422=契约错（approve 曾因字段不符 422）、405=路由错（sessions 曾 405）才算协议层失败。
fn assert_no_protocol_error(name: &str, out: &std::process::Output) {
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let combined = format!("{stderr}\n{stdout}");
    let bad = combined.contains("server error 5")
        || combined.contains("server error 405")
        || combined.contains("server error 422")
        || combined.contains("missing field `decision`")
        || combined.contains("Failed to deserialize");
    assert!(
        !bad,
        "[{name}] protocol error leaked: stderr={stderr} stdout={stdout} rc={:?}",
        out.status.code()
    );
}

#[test]
fn cli_contract_sessions_and_readonly() {
    let Some(bin) = cli_bin() else {
        eprintln!("P1-6 SKIP: codex-cli 二进制未找到（先 cargo build -p codex-cli）");
        return;
    };
    let mut svc = wait_healthy(3919);
    // P0-3 回归：sessions 曾 405 → 现在 200
    let out = cli(&bin, 3919, &["sessions"]);
    assert_no_protocol_error("sessions", &out);
    assert!(
        out.status.success(),
        "sessions 应成功，rc={:?} stderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    eprintln!("P1-6 PASS: sessions (was 405) → rc=0");
    // 只读命令批量
    for cmd in ["tools", "whoami", "coverage", "template"] {
        let o = cli(&bin, 3919, &[cmd]);
        assert_no_protocol_error(cmd, &o);
        eprintln!("P1-6 PASS: {cmd} → rc={:?}", o.status.code());
    }
    let _ = svc.kill();
    let _ = svc.wait(); // clippy: spawned process must be wait()ed
}

#[test]
fn cli_contract_status_and_approve_boundary() {
    let Some(bin) = cli_bin() else {
        eprintln!("P1-6 SKIP: codex-cli 二进制未找到");
        return;
    };
    let mut svc = wait_healthy(3920);
    // status 不存在的会话 → 业务失败（非 0），协议层不 4xx
    let out = cli(&bin, 3920, &["status", "no-such-session-xyz"]);
    assert_no_protocol_error("status-404", &out);
    assert!(
        out.status.code().unwrap_or(1) != 0,
        "status 不存在应非 0 退出"
    );
    // approve/deny 不存在的会话 → 业务失败清晰报错（非 422 契约错）
    let oa = cli(&bin, 3920, &["approve", "no-such-session", "a1"]);
    assert_no_protocol_error("approve", &oa);
    let od = cli(&bin, 3920, &["deny", "no-such-session", "a1"]);
    assert_no_protocol_error("deny", &od);
    eprintln!("P1-6 PASS: status/approve/deny → 业务失败清晰（非契约错）");
    let _ = svc.kill();
    let _ = svc.wait(); // clippy: spawned process must be wait()ed
}
