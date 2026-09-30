// v8.0: Webhook notification system.
// P0-05（2026-10-01, traecode）：锁访问改走 `recover`——与 per_user/routes 同批清掉
// "锁中毒 → 其后每次访问都 panic" 这一类（此处数据是 `Vec<WebhookConfig>`，无跨字段
// 不变式，中毒无保留价值）。
use crate::lock::recover;
use serde::{Deserialize, Serialize};
use std::sync::RwLock;

/// A registered webhook endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub url: String,
    pub events: Vec<String>,
}

/// In-memory webhook registry (per-user keyed by user_id, for future per-user isolation).
pub struct WebhookManager {
    hooks: RwLock<Vec<WebhookConfig>>,
}

impl Default for WebhookManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WebhookManager {
    pub fn new() -> Self {
        Self {
            hooks: RwLock::new(Vec::new()),
        }
    }

    /// Register a new webhook.
    pub fn register(&self, cfg: WebhookConfig) {
        recover(self.hooks.write()).push(cfg);
    }

    /// Fire all webhooks that match the given event name.
    /// Runs fire-and-forget via curl subprocess (no extra dep needed).
    ///
    /// 2026-10-01 安全修复（traecode）：原实现把注册来的 `url` **原样**交给 curl，
    /// 存在两个问题（详见 `validate_webhook_url` 文档）：**curl 选项注入** 与 **SSRF**。
    /// 现在先校验、不通过则跳过并告警（不静默）。
    pub async fn fire(event: &str, payload: &serde_json::Value, hooks: &[WebhookConfig]) {
        let allowlist = std::env::var("HEARTH_EGRESS_ALLOWLIST").ok();
        for h in hooks {
            if h.events.iter().any(|e| e == event) {
                if let Err(e) = validate_webhook_url(&h.url, allowlist.as_deref()) {
                    tracing::warn!(url = %h.url, error = %e, "webhook skipped (url rejected)");
                    continue;
                }
                let url = h.url.clone();
                let body = payload.to_string();
                tokio::spawn(async move {
                    let mut cmd = tokio::process::Command::new("curl");
                    cmd.args([
                        "-s",
                        "-X",
                        "POST",
                        "-H",
                        "Content-Type: application/json",
                        "-d",
                        &body,
                        // `--` 终止选项解析：url 一律按位置参数处理（纵深防御）
                        "--",
                        &url,
                    ]);
                    // P1-13（D-36）：原实现 = `cmd.output()` + 外层 `timeout(5s, fut)`，
                    // 有两个缺陷（与被修过的 D-18 / D-32 同类）：
                    //   ① `output()` 把 curl 的 stdout/stderr **全量**读进内存，无字节上限；
                    //   ② 超时只是**丢掉那个 future**——`kill_on_drop` 默认 false，curl
                    //      进程继续跑（孤儿 + 管道缓冲滞留），且从不 `wait()` 收割。
                    // 现改走 `run_bounded()`：有界排空到 EOF + 超时**树杀 + 收割**。
                    match run_bounded(cmd, WEBHOOK_TIMEOUT, bounded_io::MAX_CAPTURED_BYTES).await {
                        Ok(out) if out.status.success() => {
                            if out.truncated() {
                                tracing::warn!(
                                    url = %url,
                                    stdout_bytes = out.stdout.len(),
                                    stderr_bytes = out.stderr.len(),
                                    "webhook 响应体超上限，已截断（仅记前 8 MiB，不静默）"
                                );
                            }
                            tracing::debug!(url = %url, "webhook delivered");
                        }
                        Ok(out) => {
                            tracing::warn!(
                                url = %url,
                                code = ?out.status.code(),
                                stderr = %String::from_utf8_lossy(&out.stderr),
                                "webhook delivery failed (non-zero exit)"
                            );
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                            tracing::warn!(
                                url = %url,
                                "webhook delivery timed out (5s)——已树上杀 + 收割"
                            );
                        }
                        Err(e) => {
                            tracing::warn!(
                                url = %url,
                                error = %e,
                                "webhook spawn failed (curl 未安装？)"
                            );
                        }
                    }
                });
            }
        }
    }

    /// v10.4: Fire matching webhooks from the registry.
    pub async fn fire_event(&self, event: &str, payload: &serde_json::Value) {
        let hooks = recover(self.hooks.read()).clone();
        if !hooks.is_empty() {
            Self::fire(event, payload, &hooks).await;
        }
    }

    pub fn list(&self) -> Vec<WebhookConfig> {
        recover(self.hooks.read()).clone()
    }
}

/// webhook 投递超时（原实现同为 5s；提为常量以便生产与回归测试共用）。
const WEBHOOK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// 单次子进程执行结果——**有界**捕获（最多各 `cap` 字节）。
#[derive(Debug)]
struct BoundedOutcome {
    status: std::process::ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    stdout_truncated: bool,
    stderr_truncated: bool,
}

impl BoundedOutcome {
    fn truncated(&self) -> bool {
        self.stdout_truncated || self.stderr_truncated
    }
}

/// 执行命令并**有界**捕获输出；超时则**树杀 + 收割**（不留孤儿、不留僵尸）。
///
/// 与 P0-06 / P0-07 / P0-09 / P0-12 同一条不变式（实现已收敛到 `bounded-io`）：
/// - **必须排空到 EOF**，只保留前 `cap` 字节——若图省事用 `take(cap)` 提前停读，
///   子进程写满管道后会永久阻塞（把 OOM 换成死锁）；
/// - 超时必须杀到**进程树**并 `wait()` 收割，否则 `curl` 会继续跑。
///
/// `kill_on_drop(true)` 是**兜底**（显式树杀失败时仍会终止直接子进程）。
async fn run_bounded(
    mut cmd: tokio::process::Command,
    timeout: std::time::Duration,
    cap: usize,
) -> std::io::Result<BoundedOutcome> {
    use std::process::Stdio;
    cmd.stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = cmd.spawn()?;
    let pid = child.id();
    let so = child.stdout.take().expect("stdout 已 piped");
    let se = child.stderr.take().expect("stderr 已 piped");
    // 两路并发有界排空（各自独立任务，避免与 wait() 互相借用）。
    let h_out = tokio::spawn(bounded_io::drain_capped_async(so, cap));
    let h_err = tokio::spawn(bounded_io::drain_capped_async(se, cap));
    match tokio::time::timeout(timeout, child.wait()).await {
        Ok(status) => {
            let (stdout, stdout_truncated) = h_out.await.unwrap_or_default();
            let (stderr, stderr_truncated) = h_err.await.unwrap_or_default();
            Ok(BoundedOutcome {
                status: status?,
                stdout,
                stderr,
                stdout_truncated,
                stderr_truncated,
            })
        }
        Err(_) => {
            // 超时：先树杀（Windows 用系统自带 taskkill /T），再直接杀 + 收割。
            if let Some(pid) = pid {
                bounded_io::kill_process_tree(pid);
            }
            let _ = child.kill().await;
            let _ = child.wait().await; // 收割，不留僵尸
            h_out.abort();
            h_err.abort();
            Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "webhook delivery timed out",
            ))
        }
    }
}

/// 2026-10-01 安全修复（traecode）：webhook 的 `url` 由 API 注册后**原样交给 curl**。
///
/// **问题 ①：curl 选项注入（本地文件读写原语）**
/// `url` 是 curl 的**最后一个位置参数**，但 curl 会**在整个 argv 里**解析选项——
/// 若 `url` 以 `-` 开头即被当作选项。例如 `-K/path/to/conf`（curl 会读该配置文件，
/// 从而注入任意 curl 选项）或 `-o/path/to/file`（把响应体写到任意路径）
/// → 注册一个 webhook 即可获得**宿主机文件读写**能力。
///
/// **问题 ②：SSRF**
/// 服务端会代注册者发起请求，可打到内网/云元数据（`169.254.169.254` 等）。
/// webhook 的语义本就允许指向任意外部服务，故**默认不阻断**（与 README
/// 「不设/空 = 全放」一致）；但**显式设置 `HEARTH_EGRESS_ALLOWLIST` 时**按白名单收紧，
/// 与工具层同口径。
///
/// 纯函数（显式传入白名单）——便于单测，且不依赖进程环境。
fn validate_webhook_url(raw: &str, allowlist_csv: Option<&str>) -> anyhow::Result<()> {
    // ① 必须是 http/https（同时挡掉 `-K…` 这类非 URL 的选项注入）
    if !(raw.starts_with("http://") || raw.starts_with("https://")) {
        anyhow::bail!("webhook url 必须以 http:// 或 https:// 开头（收到: {raw}）");
    }
    let host = raw
        .split("://")
        .nth(1)
        .and_then(|rest| rest.split(['/', '?', '#']).next())
        .unwrap_or("");
    if host.is_empty() {
        anyhow::bail!("webhook url 无 host: {raw}");
    }
    // ② 仅当显式设置白名单时才收紧（默认全放，保持既有语义）
    let allow: Vec<&str> = allowlist_csv
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if !allow.is_empty() {
        let hostname = host.split(':').next().unwrap_or(host);
        let ok = allow.iter().any(|a| {
            let a = a.trim();
            let base = a.strip_prefix("*.").unwrap_or(a);
            hostname == base || hostname.ends_with(&format!(".{base}"))
        });
        if !ok {
            anyhow::bail!("webhook url 域名 {hostname} 不在 HEARTH_EGRESS_ALLOWLIST 白名单");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-10-01 安全修复回归（**先红后绿**）：curl 选项注入与非法 scheme 必须被拒。
    /// 修复前 `fire` 不做任何校验，这些 url 会原样进入 curl argv。
    #[test]
    fn test_webhook_url_validation() {
        // ① curl 选项注入：以 `-` 开头（-K 读配置 / -o 写文件）
        assert!(validate_webhook_url("-K/tmp/evil.conf", None).is_err());
        assert!(validate_webhook_url("--output/etc/passwd", None).is_err());
        // ② 非 http(s) scheme / 无 host
        assert!(validate_webhook_url("file:///etc/passwd", None).is_err());
        assert!(validate_webhook_url("http://", None).is_err());
        assert!(validate_webhook_url("", None).is_err());
        // ③ 合法 URL：默认（未设白名单）放行
        assert!(validate_webhook_url("https://hooks.example.com/x", None).is_ok());
        assert!(validate_webhook_url("http://127.0.0.1:8080/h", None).is_ok());

        // ④ 显式白名单时收紧：未命中即拒，命中（含子域）即放
        let al = Some("example.com,foo.org");
        assert!(validate_webhook_url("https://hooks.example.com/x", al).is_ok());
        assert!(validate_webhook_url("https://evil.test/x", al).is_err());
        assert!(validate_webhook_url("http://169.254.169.254/latest", al).is_err());
        // 通配写法 `*.example.com` 与其裸域等价
        assert!(validate_webhook_url("https://a.example.com/x", Some("*.example.com")).is_ok());
        assert!(validate_webhook_url("https://example.com/x", Some("*.example.com")).is_ok());
    }

    /// P1-13（D-36）回归锁——投递子进程的两条不变式（与已修的 D-18/D-32 同类）：
    /// ① **有界捕获**：修复前 `Command::output()` 把 curl 的 stdout/stderr 全量读进内存；
    /// ② **超时必须真杀**：修复前超时只丢掉 `output()` 的 future，而 `kill_on_drop`
    ///    默认 false → 子进程继续跑（孤儿）。此处用**哨兵法**验真：子进程本应在数秒后
    ///    写一个哨兵文件；300ms 超时后它必须已被杀，哨兵永不出现。
    #[tokio::test]
    async fn test_p1_13_run_bounded_caps_output_and_kills_on_timeout() {
        // ① 有界捕获（cap=4096，命令产出约 120KB）
        let out = run_bounded(noisy_cmd(), std::time::Duration::from_secs(60), 4096)
            .await
            .expect("spawn 必须成功");
        assert_eq!(
            out.stdout.len(),
            4096,
            "保留量必须被 cap 钳住（修复前 = 全量入内存）"
        );
        assert!(out.stdout_truncated, "超限必须标记截断（不静默）");
        assert!(out.status.success());

        // ② 超时树杀 + 收割（哨兵法）
        let dir = std::env::temp_dir().join(format!("hearth_p113_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let sentinel = dir.join("sentinel.txt");
        let err = run_bounded(
            hang_then_touch_cmd(&sentinel),
            std::time::Duration::from_millis(300),
            4096,
        )
        .await
        .expect_err("超时必须返回 Err");
        assert_eq!(err.kind(), std::io::ErrorKind::TimedOut);
        // 等过子进程原本该写哨兵的时刻，确认它确实没能活到那时。
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        assert!(
            !sentinel.exists(),
            "超时后子进程必须已被杀——哨兵文件不得出现（出现 = 孤儿进程仍在跑）"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 产出约 120KB 到 stdout 的跨平台命令（远超测试用 cap=4096）。
    fn noisy_cmd() -> tokio::process::Command {
        #[cfg(windows)]
        {
            let mut c = tokio::process::Command::new("cmd");
            c.args([
                "/C",
                "for /L %i in (1,1,2000) do @echo 0123456789012345678901234567890123456789012345678901234567890",
            ]);
            c
        }
        #[cfg(not(windows))]
        {
            let mut c = tokio::process::Command::new("sh");
            c.args(["-c", "i=0; while [ $i -lt 2000 ]; do echo 0123456789012345678901234567890123456789012345678901234567890; i=$((i+1)); done"]);
            c
        }
    }

    /// 先等约 3 秒再写哨兵文件的跨平台命令。
    fn hang_then_touch_cmd(sentinel: &std::path::Path) -> tokio::process::Command {
        #[cfg(windows)]
        {
            let mut c = tokio::process::Command::new("cmd");
            c.args([
                "/C",
                &format!(
                    "ping -n 4 127.0.0.1 >nul & echo x > \"{}\"",
                    sentinel.display()
                ),
            ]);
            c
        }
        #[cfg(not(windows))]
        {
            let mut c = tokio::process::Command::new("sh");
            c.args(["-c", &format!("sleep 3; echo x > '{}'", sentinel.display())]);
            c
        }
    }
}
