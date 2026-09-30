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
                    let cmd = tokio::process::Command::new("curl")
                        .args([
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
                        ])
                        .output();
                    // P0-04（2026-10-01, traecode）：原为 `let _ =`——把
                    // 「curl 不存在（NotFound）」「超时」「非零退出」**全部静默吞掉**，
                    // 注册端拿不到任何失败信号：告警/回调永不送达却**零痕迹**。
                    // fire-and-forget 语义不变，但失败必须留痕（"不静默"）。
                    match tokio::time::timeout(std::time::Duration::from_secs(5), cmd).await {
                        Ok(Ok(out)) if out.status.success() => {
                            tracing::debug!(url = %url, "webhook delivered");
                        }
                        Ok(Ok(out)) => {
                            tracing::warn!(
                                url = %url,
                                code = ?out.status.code(),
                                stderr = %String::from_utf8_lossy(&out.stderr),
                                "webhook delivery failed (non-zero exit)"
                            );
                        }
                        Ok(Err(e)) => {
                            tracing::warn!(
                                url = %url,
                                error = %e,
                                "webhook spawn failed (curl 未安装？)"
                            );
                        }
                        Err(_) => {
                            tracing::warn!(url = %url, "webhook delivery timed out (5s)");
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
}
