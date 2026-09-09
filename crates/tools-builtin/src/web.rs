//! WS10 (v0.2): 受控联网工具 web_fetch——LLM 从指定来源拉真实信息（认知外延）。
//! **G0 出网治理（唯一 sanctioned 出网口）**：
//! - deny-by-default：URL 域名必须命中 `HEARTH_EGRESS_ALLOWLIST`（逗号分隔域名/后缀，
//!   如 "example.com,rust-lang.org"；子域/后缀匹配：allowlist 含 example.com 则
//!   docs.example.com 放行）；白名单未配置 → 全部拒绝（fail-closed）。
//! - 审计：每次出网记 tracing 日志（url/host/ts）。
//! - 返回内容带 `source`(URL) + `verifiable` 标签，进 WS7 验证流（默认待验证断言）。
//!
//! 注：bash `curl` 后门治理依赖全局代理 env（HTTP(S)_PROXY），本版内核级 netns 隔离
//! 不可行（VM 无 CAP_SYS_ADMIN）——列为 🟡 遗留（任务书 §8.5 已注）。

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use tool_runtime::{Tool, ToolContext, ToolDescription};

/// 正文截断上限（防单次工具输出撑爆上下文）。
const MAX_BODY_CHARS: usize = 8000;

pub struct WebTool;

impl WebTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WebTool {
    fn default() -> Self {
        Self::new()
    }
}

/// 出网白名单匹配：allowlist 项是域名或后缀（example.com 匹配 example.com/docs.example.com）。
/// 空 allowlist = fail-closed（全部拒绝，须显式配置才放行）。
fn egress_allowed(host: &str, allowlist: &[String]) -> bool {
    if allowlist.is_empty() {
        return false;
    }
    let h = host.trim_end_matches('.');
    allowlist.iter().any(|a| {
        let a = a.trim().trim_start_matches('.').to_lowercase();
        let a = a.trim_end_matches('.');
        h.eq_ignore_ascii_case(a) || h.ends_with(&format!(".{a}"))
    })
}

/// 极简 HTML→纯文本（去标签/实体；不引重依赖）。返回去标签后的文本。
fn strip_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[async_trait]
impl Tool for WebTool {
    fn name(&self) -> &str {
        "web_fetch"
    }

    fn description(&self) -> ToolDescription {
        ToolDescription {
            name: "web_fetch".into(),
            description: "从指定 URL 获取网页正文（受控联网：仅限 HEARTH_EGRESS_ALLOWLIST 白名单域名）。返回 source(URL) + 正文——内容默认未验证断言，影响结果正确性时须再核验。用于查官方文档/primary source。".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "要获取的完整 URL（http/https）" }
                },
                "required": ["url"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<String> {
        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("missing 'url' argument"))?;
        let parsed = reqwest::Url::parse(url).map_err(|e| anyhow!("URL 解析失败: {e}"))?;
        let host = parsed
            .host_str()
            .ok_or_else(|| anyhow!("URL 无 host: {url}"))?;

        // G0 出网治理：deny-by-default 白名单
        let allowlist: Vec<String> = ctx
            .env
            .get("HEARTH_EGRESS_ALLOWLIST")
            .map(|v| v.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();
        if !egress_allowed(host, &allowlist) {
            tracing::warn!(url, host, "web_fetch denied by egress allowlist");
            return Err(anyhow!(
                "出网被拒：域名 {host} 不在 HEARTH_EGRESS_ALLOWLIST 白名单——\
                 联网工具是唯一受控出网口；如需放行请配置白名单（如 \
                 HEARTH_EGRESS_ALLOWLIST=example.com,rust-lang.org）"
            ));
        }
        // 审计：URL/域/时间（fail-open 侧仍留痕——白名单放行也记）
        tracing::info!(url, host, "web_fetch egress audit");

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent("hearth-agent/0.2 (web_fetch)")
            .build()?;
        let resp = client
            .get(url)
            .send()
            .await
            .map_err(|e| anyhow!("fetch 失败（网络/超时）: {e}"))?;
        if !resp.status().is_success() {
            return Err(anyhow!("HTTP {} for {url}", resp.status()));
        }
        let raw = resp
            .text()
            .await
            .map_err(|e| anyhow!("读 body 失败: {e}"))?;
        let text = strip_html(&raw);
        let raw_chars = text.chars().count();
        let truncated: String = text.chars().take(MAX_BODY_CHARS).collect();
        let truncated = if raw_chars > MAX_BODY_CHARS {
            format!(
                "{truncated}\n... [{} chars truncated (web body cap)] ...",
                raw_chars - MAX_BODY_CHARS
            )
        } else {
            truncated
        };
        let out = serde_json::json!({
            "source": url,
            "verifiable": true,
            "host": host,
            "content": truncated,
            "note": "内容来自联网检索，默认未验证断言——影响结果正确性时须再核验（WS7）",
        });
        Ok(serde_json::to_string_pretty(&out)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// WS10: 出网白名单 deny-by-default——未配置白名单全部拒绝（fail-closed）；
    /// 白名单命中放行（含子域后缀匹配）；未命中拒绝。无白名单时此测试红。
    #[test]
    fn test_egress_allowlist() {
        // 空白名单 = 全拒（fail-closed）
        assert!(!egress_allowed("example.com", &[]));
        // 精确命中
        let wl = vec!["example.com".to_string()];
        assert!(egress_allowed("example.com", &wl));
        assert!(egress_allowed("docs.example.com", &wl), "子域应放行");
        assert!(!egress_allowed("evil.com", &wl), "非白名单域拒绝");
        assert!(!egress_allowed("notexample.com", &wl), "后缀混淆拒绝");
        // 多域名 + 前缀点
        let wl2 = vec!["rust-lang.org".to_string(), ".google.com".to_string()];
        assert!(egress_allowed("rust-lang.org", &wl2));
        assert!(egress_allowed("developers.google.com", &wl2));
        assert!(!egress_allowed("example.org", &wl2));
    }

    /// WS10: 极简 HTML 去标签
    #[test]
    fn test_strip_html() {
        let r = strip_html("<html><body><h1>标题</h1><p>正文内容</p></body></html>");
        assert!(r.contains("标题"), "应保留文本: {r}");
        assert!(r.contains("正文内容"));
        assert!(!r.contains('<'), "不应残留标签");
    }

    /// WS10: web_fetch 白名单拒绝路径（deny-by-default，不真发请求）
    #[tokio::test]
    async fn test_web_fetch_denied_without_allowlist() {
        let tool = WebTool::new();
        let ctx = ToolContext {
            env: HashMap::new(),
            ..Default::default()
        };
        let r = tool
            .execute(serde_json::json!({"url": "https://example.com"}), &ctx)
            .await;
        assert!(r.is_err(), "无白名单必须拒绝出网");
        assert!(
            r.unwrap_err().to_string().contains("出网被拒"),
            "错误须含出网被拒提示"
        );
    }

    /// T10 (v0.2.3): 配了白名单必须放行——修复"假绿"（旧测试只证明空 env→拒，
    /// 不证明配了能放行；生产路径 ctx.env 曾结构性为空导致白名单永不生效）。
    #[tokio::test]
    async fn test_web_fetch_allowed_with_allowlist() {
        let tool = WebTool::new();
        let mut env = HashMap::new();
        env.insert(
            "HEARTH_EGRESS_ALLOWLIST".to_string(),
            "example.com".to_string(),
        );
        let ctx = ToolContext {
            env,
            ..Default::default()
        };
        // 命中白名单（后缀匹配）→ 进入 fetch（此处网络不可达会报 fetch 错误，
        // 但绝不报"出网被拒"——证明白名单放行逻辑生效）
        let r = tool
            .execute(serde_json::json!({"url": "https://docs.example.com"}), &ctx)
            .await;
        match r {
            Ok(_) => {} // 网络可达时成功
            Err(e) => {
                let msg = e.to_string();
                assert!(!msg.contains("出网被拒"), "白名单内域名不得被拒: {msg}");
                assert!(
                    !msg.contains("HEARTH_EGRESS_ALLOWLIST"),
                    "不得提示配白名单: {msg}"
                );
            }
        }
        // 白名单外仍拒（不破坏 deny-by-default）
        let r2 = tool
            .execute(serde_json::json!({"url": "https://evil.example.net"}), &ctx)
            .await;
        assert!(r2.is_err(), "白名单外必须仍拒");
        assert!(r2.unwrap_err().to_string().contains("出网被拒"));
    }
}
