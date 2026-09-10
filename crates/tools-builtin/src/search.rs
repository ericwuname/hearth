//! S13（手术包二）：内置联网搜索工具 web_search——查资料的一等公民。
//!
//! 病灶锚点：查资料只能靠 bash curl 间接 + 模型记忆（S2 出网已放开，缺一等公民
//! 工具）。本工具接**零 key 免费搜索源**：DuckDuckGo HTML（主）→ 必应 HTML（回落），
//! 返回 标题 + 链接 + 摘要 top10。
//!
//! 出网治理（S2 口径，与 bash 一致）：**默认放开**；仅当显式配置
//! `HEARTH_EGRESS_ALLOWLIST`（逗号分隔域名/后缀）时按白名单收紧（fail-closed：
//! 认不出主机或不在名单即拒）。注意与 web_fetch 的 deny-by-default 语义**相反**
//! ——两处差异是既有事实（web_fetch 是"唯一受控出网口"时代的产物），本卡按
//! 任务书"出网走 S2 默认放开"落刀，差异已在战报申报。
//!
//! 解析器是**纯函数**（`parse_ddg_html` / `parse_bing_html`）——单测喂 mock HTML，
//! 不打网络；网络可达性属真机验收项。

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use tool_runtime::{Tool, ToolContext, ToolDescription};

/// 返回条数上限（任务书：top10）。
const MAX_RESULTS: usize = 10;
/// 单条摘要字符上限（防个别条目撑爆上下文）。
const MAX_SNIPPET_CHARS: usize = 300;

const DDG_ENDPOINT: &str = "https://html.duckduckgo.com/html/";
const BING_ENDPOINT: &str = "https://www.bing.com/search";

/// 一条搜索结果（解析器的输出单元）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

pub struct WebSearchTool {
    client: reqwest::Client,
}

impl WebSearchTool {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            // 搜索源的 HTML 端点对无 UA 的请求常直接拒绝——带常规 UA。
            .user_agent(
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) \
                 hearth-agent/0.2 (web_search)",
            )
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { client }
    }
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

async fn fetch_html(client: &reqwest::Client, url: &str) -> Result<String> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| anyhow!("请求失败（网络/超时）: {e}"))?;
    if !resp.status().is_success() {
        return Err(anyhow!("HTTP {}", resp.status()));
    }
    resp.text().await.map_err(|e| anyhow!("读 body 失败: {e}"))
}

/// S2 口径的出网判定：allowlist 为空 = 默认放开；非空 = 只放行命中项
///（相等或 `.条目` 后缀；认不出主机一律拒）。
fn egress_allowed_open(host: Option<&str>, allowlist: &[String]) -> bool {
    if allowlist.is_empty() {
        return true; // S2 默认放开（用户拍板 2026-09-09）
    }
    let Some(h) = host else { return false };
    let h = h.trim_end_matches('.').to_lowercase();
    allowlist.iter().any(|a| {
        let a = a.trim().trim_start_matches('.').to_lowercase();
        let a = a.trim_end_matches('.');
        !a.is_empty() && (h == a || h.ends_with(&format!(".{a}")))
    })
}

fn allowlist_from_env(ctx: &ToolContext) -> Vec<String> {
    ctx.env
        .get("HEARTH_EGRESS_ALLOWLIST")
        .map(|v| v.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default()
}

/// 极简 HTML 去标签（保留内部文本；空白归一）。
fn strip_tags(s: &str) -> String {
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
    normalize_ws(&unescape_html(&out))
}

/// 实体解码（搜索结果里的标题/摘要常见 `&amp;` `&#x27;` `&quot;` `&nbsp;`）。
fn unescape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(idx) = rest.find('&') {
        out.push_str(&rest[..idx]);
        let tail = &rest[idx..];
        // 实体最长 &nbsp; = 6 字符 + ';'；取一小段窗口做前缀匹配。
        let window: String = tail.chars().take(10).collect();
        let mut consumed = 0usize;
        for (ent, rep) in [
            ("&amp;", "&"),
            ("&quot;", "\""),
            ("&#x27;", "'"),
            ("&#39;", "'"),
            ("&apos;", "'"),
            ("&lt;", "<"),
            ("&gt;", ">"),
            ("&nbsp;", " "),
            ("&#x2F;", "/"),
            ("&hellip;", "…"),
        ] {
            if window.starts_with(ent) {
                out.push_str(rep);
                consumed = ent.len();
                break;
            }
        }
        if consumed == 0 {
            out.push('&');
            consumed = 1;
        }
        rest = &tail[consumed..];
    }
    out.push_str(rest);
    out
}

/// percent 解码（DuckDuckGo 的 `uddg=` 参数是被转义的真实 URL）。
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
            if let Ok(b) = u8::from_str_radix(hex, 16) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn normalize_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_chars(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        return s.to_string();
    }
    format!("{}…", chars[..max].iter().collect::<String>())
}

/// DuckDuckGo 的跳转链接还原：`//duckduckgo.com/l/?uddg=<encoded>&rut=…` → 真实 URL。
/// 已是绝对 URL 的原样返回；协议相对（`//host/…`）补 https。
fn normalize_ddg_url(raw: &str) -> String {
    let raw = unescape_html(raw);
    if let Some(pos) = raw.find("uddg=") {
        let tail = &raw[pos + "uddg=".len()..];
        let enc = tail.split('&').next().unwrap_or("");
        let decoded = percent_decode(enc);
        if decoded.starts_with("http://") || decoded.starts_with("https://") {
            return decoded;
        }
    }
    if let Some(rest) = raw.strip_prefix("//") {
        return format!("https://{rest}");
    }
    raw
}

/// 取 `from` 之后第一段 `<a …>TEXT</a>` 的 TEXT（跳过起始标签）。
fn anchor_text_after(html: &str, from: usize) -> Option<String> {
    let tail = html.get(from..)?;
    let gt = tail.find('>')?;
    let body = &tail[gt + 1..];
    let close = body.find("</a>")?;
    Some(strip_tags(&body[..close]))
}

/// 在 `from` 之后、`until`（下一个结果块）之前找 marker 所在标签的文本。
fn tagged_text_after(
    html: &str,
    from: usize,
    marker: &str,
    until: Option<usize>,
) -> Option<String> {
    let tail = html.get(from..)?;
    let rel = tail.find(marker)?;
    let at = from + rel;
    if let Some(end) = until {
        if at >= end {
            return None;
        }
    }
    let tail2 = html.get(at..)?;
    let gt = tail2.find('>')?;
    let body = &tail2[gt + 1..];
    let close = body.find("</a>").or_else(|| body.find("</p>"))?;
    Some(strip_tags(&body[..close]))
}

/// DuckDuckGo HTML 结果解析（纯函数——mock HTML 单测）。
pub fn parse_ddg_html(html: &str) -> Vec<SearchHit> {
    let mut hits: Vec<SearchHit> = Vec::new();
    let mut cursor = 0usize;
    while let Some(rel) = html[cursor..].find("result__a") {
        let at = cursor + rel;
        let Some(href_rel) = html[at..].find("href=\"") else {
            break;
        };
        let href_at = at + href_rel + "href=\"".len();
        let Some(q_rel) = html[href_at..].find('"') else {
            break;
        };
        let raw_href = &html[href_at..href_at + q_rel];
        let url = normalize_ddg_url(raw_href);
        let title = anchor_text_after(html, href_at + q_rel).unwrap_or_default();
        // 摘要：限定在下一个 result__a 之前（缺摘要的条目不偷下一条的摘要）。
        let next_anchor = html[href_at + q_rel..]
            .find("result__a")
            .map(|r| href_at + q_rel + r);
        let snippet = tagged_text_after(html, href_at + q_rel, "result__snippet", next_anchor)
            .unwrap_or_default();
        if !url.is_empty() && !title.is_empty() {
            hits.push(SearchHit {
                title,
                url,
                snippet: truncate_chars(&snippet, MAX_SNIPPET_CHARS),
            });
        }
        cursor = at + "result__a".len();
    }
    dedupe_truncate(hits)
}

/// 必应 HTML 结果解析（纯函数——mock HTML 单测）。结构：
/// `<li class="b_algo"><h2><a href="URL">Title</a></h2>…<p>Snippet</p>…</li>`
pub fn parse_bing_html(html: &str) -> Vec<SearchHit> {
    let mut hits: Vec<SearchHit> = Vec::new();
    let mut cursor = 0usize;
    while let Some(rel) = html[cursor..].find("b_algo") {
        let at = cursor + rel;
        // 块边界：下一个 b_algo（摘要只在块内找）
        let block_end = html[at..]
            .find("b_algo")
            .and_then(|r2| {
                html[at + r2 + 1..]
                    .find("b_algo")
                    .map(|r3| at + r2 + 1 + r3)
            })
            .unwrap_or(html.len());
        let block = html.get(at..block_end).unwrap_or("");
        let url = block
            .find("href=\"http")
            .map(|h| {
                let start = h + "href=\"".len();
                let rest = &block[start..];
                let end = rest.find('"').unwrap_or(0);
                unescape_html(&rest[..end])
            })
            .unwrap_or_default();
        let title = block
            .find("<h2")
            .and_then(|h| {
                let rest = &block[h..];
                let a = rest.find("<a ")?;
                anchor_text_after(rest, a)
            })
            .unwrap_or_default();
        let snippet = block
            .find("<p")
            .and_then(|p| {
                let rest = &block[p..];
                let gt = rest.find('>')?;
                let body = &rest[gt + 1..];
                let close = body.find("</p>")?;
                Some(strip_tags(&body[..close]))
            })
            .unwrap_or_default();
        if !url.is_empty() && !title.is_empty() {
            hits.push(SearchHit {
                title,
                url,
                snippet: truncate_chars(&snippet, MAX_SNIPPET_CHARS),
            });
        }
        cursor = at + "b_algo".len();
    }
    dedupe_truncate(hits)
}

fn dedupe_truncate(hits: Vec<SearchHit>) -> Vec<SearchHit> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for h in hits {
        if seen.insert(h.url.clone()) {
            out.push(h);
            if out.len() >= MAX_RESULTS {
                break;
            }
        }
    }
    out
}

fn search_url(endpoint: &str, query: &str) -> Result<String> {
    let mut u = reqwest::Url::parse(endpoint).map_err(|e| anyhow!("搜索源 URL 非法: {e}"))?;
    u.query_pairs_mut().append_pair("q", query);
    Ok(u.to_string())
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> ToolDescription {
        ToolDescription {
            name: "web_search".into(),
            description: "Search the web and return the top results (title + URL + snippet).\n\
何时用: 需要项目之外的信息——库/API 用法、版本变更、报错含义、事实核对；\
实现类任务**先查资料再动手**（不靠记忆猜 API，模型知识可能过时）。\n\
何时不用: 查本项目内的代码/文件（grep/glob/read 更快更准）；已知道具体 URL（直接 web_fetch 取正文）。\n\
示例: web_search(query=\"rust tokio select biased cancellation\")；\
web_search(query=\"cargo workspace 依赖继承 workspace = true\")。\n\
边界: 返回 标题+链接+摘要 top10（零 key 免费源：DuckDuckGo HTML，失败自动回落必应）；\
出网走 S2 默认放开（配置 HEARTH_EGRESS_ALLOWLIST 时按白名单收紧）；单次超时 20s。\n\
错误解读: \"无结果\"=关键词太偏或太窄（换英文/更短关键词重试）；\
\"搜索源不可用\"=网络不通或源端限流（稍后重试，或改用 web_fetch 直取已知 URL）；\
\"出网被拒\"=域名不在白名单（提示用户配置 HEARTH_EGRESS_ALLOWLIST）。"
                .into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "搜索关键词（英文关键词命中率更高；避免整句自然语言）"
                    }
                },
                "required": ["query"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<String> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|q| !q.is_empty())
            .ok_or_else(|| anyhow!("missing 'query' argument（搜索关键词不能为空）"))?;

        let allowlist = allowlist_from_env(ctx);
        let ddg_url = search_url(DDG_ENDPOINT, query)?;
        let bing_url = search_url(BING_ENDPOINT, query)?;
        for u in [&ddg_url, &bing_url] {
            let host = reqwest::Url::parse(u)
                .ok()
                .and_then(|p| p.host_str().map(String::from));
            if !egress_allowed_open(host.as_deref(), &allowlist) {
                return Err(anyhow!(
                    "出网被拒：域名 {} 不在 HEARTH_EGRESS_ALLOWLIST 白名单——\
                     如需放行请配置（如 HEARTH_EGRESS_ALLOWLIST=duckduckgo.com,bing.com），\
                     或清空该配置走默认放开（S2）",
                    host.unwrap_or_else(|| "?".into())
                ));
            }
        }

        // 主源 → 回落源（源端限流/结构变更时不至于整个工具失效）。
        let mut failures: Vec<String> = Vec::new();
        for (name, url, parser) in [
            (
                "duckduckgo",
                ddg_url.as_str(),
                parse_ddg_html as fn(&str) -> Vec<SearchHit>,
            ),
            (
                "bing",
                bing_url.as_str(),
                parse_bing_html as fn(&str) -> Vec<SearchHit>,
            ),
        ] {
            // S2 审计：出网留痕（与 bash 的 [net] 审计同规——不阻断，只记）。
            tracing::info!(source = name, host = %url, "web_search egress audit");
            match fetch_html(&self.client, url).await {
                Ok(html) => {
                    let hits = parser(&html);
                    if hits.is_empty() {
                        failures.push(format!("{name}: 解析出 0 条结果（结构变更或限流页）"));
                        continue;
                    }
                    let out = serde_json::json!({
                        "source": name,
                        "query": query,
                        "count": hits.len(),
                        "results": hits
                            .iter()
                            .map(|h| serde_json::json!({
                                "title": h.title,
                                "url": h.url,
                                "snippet": h.snippet,
                            }))
                            .collect::<Vec<_>>(),
                        "note": "结果为搜索摘要（默认未验证断言）——引用事实前用 web_fetch 打开链接核对原文。",
                    });
                    return Ok(serde_json::to_string_pretty(&out)?);
                }
                Err(e) => {
                    failures.push(format!("{name}: {e}"));
                }
            }
        }
        Err(anyhow!(
            "搜索源不可用（{} ）——稍后重试，或改用 web_fetch 直取已知 URL",
            failures.join("；")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// 真实 DuckDuckGo HTML 端点的结果块形态（简化但保留关键 class/跳转链接）。
    const DDG_FIXTURE: &str = r#"
<div class="result results_links">
  <h2 class="result__title">
    <a rel="nofollow" class="result__a" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fdoc.rust-lang.org%2Fstd%2F&amp;rut=abc">Rust std &amp; docs</a>
  </h2>
  <a class="result__snippet" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fdoc.rust-lang.org%2Fstd%2F">Standard library documentation &#x27;for&#x27; Rust.</a>
</div>
<div class="result results_links">
  <h2 class="result__title">
    <a rel="nofollow" class="result__a" href="https://tokio.rs/tokio/tutorial/select">Tokio select!</a>
  </h2>
  <a class="result__snippet" href="https://tokio.rs/tokio/tutorial/select">Wait on multiple futures; biased mode polls in order.</a>
</div>
"#;

    const BING_FIXTURE: &str = r#"
<li class="b_algo">
  <h2><a href="https://doc.rust-lang.org/cargo/reference/workspaces.html">Cargo Workspaces - The Cargo Book</a></h2>
  <div class="b_caption"><p>Workspaces let you manage multiple packages with shared Cargo.lock.</p></div>
</li>
<li class="b_algo">
  <h2><a href="https://example.com/other">Other result</a></h2>
  <p>A snippet without caption div.</p>
</li>
"#;

    /// S13①：DuckDuckGo HTML 解析——跳转链接还原 + 实体解码 + 标题/摘要提取；
    /// 多命中顺序稳定、互不串摘要。
    #[test]
    fn test_s13_parse_ddg_html() {
        let hits = parse_ddg_html(DDG_FIXTURE);
        assert_eq!(hits.len(), 2, "两条结果都要解析出来：{hits:?}");
        assert_eq!(hits[0].url, "https://doc.rust-lang.org/std/");
        assert_eq!(hits[0].title, "Rust std & docs", "实体 &amp; 必须解码");
        assert!(
            hits[0]
                .snippet
                .contains("Standard library documentation 'for' Rust."),
            "摘要须与自身条目配对（不串下一条），实际 {}",
            hits[0].snippet
        );
        // 已是绝对 URL 的原样保留
        assert_eq!(hits[1].url, "https://tokio.rs/tokio/tutorial/select");
        assert!(hits[1].snippet.contains("biased mode"));
    }

    /// S13②：必应 HTML 解析（块状结构 + `<p>` 摘要）。
    #[test]
    fn test_s13_parse_bing_html() {
        let hits = parse_bing_html(BING_FIXTURE);
        assert_eq!(hits.len(), 2, "两条结果都要解析出来：{hits:?}");
        assert_eq!(hits[0].title, "Cargo Workspaces - The Cargo Book");
        assert_eq!(
            hits[0].url,
            "https://doc.rust-lang.org/cargo/reference/workspaces.html"
        );
        assert!(hits[0].snippet.contains("shared Cargo.lock"));
        assert!(hits[1].snippet.contains("without caption"));
    }

    /// S13③：top10 截断 + 同 URL 去重（防重复条目灌满上下文）。
    #[test]
    fn test_s13_parser_caps_and_dedupes() {
        let mut html = String::new();
        for i in 0..15 {
            html.push_str(&format!(
                "<a class=\"result__a\" href=\"https://example.com/{i}\">T{i}</a>\
                 <a class=\"result__snippet\">S{i}</a>"
            ));
        }
        // 重复 URL（与第 0 条相同）必须被去重
        html.push_str(
            "<a class=\"result__a\" href=\"https://example.com/0\">dup</a>\
             <a class=\"result__snippet\">dup</a>",
        );
        let hits = parse_ddg_html(&html);
        assert_eq!(hits.len(), MAX_RESULTS, "上限 top10");
        assert_eq!(hits[0].url, "https://example.com/0");
        let urls: std::collections::HashSet<_> = hits.iter().map(|h| h.url.clone()).collect();
        assert_eq!(urls.len(), hits.len(), "同 URL 必须去重");
    }

    /// S13④：空/畸形 HTML 不 panic、返回空（源端改版时不崩工具）。
    #[test]
    fn test_s13_parser_malformed_html_is_safe() {
        assert!(parse_ddg_html("").is_empty());
        assert!(parse_ddg_html("<html><body>no results</body></html>").is_empty());
        assert!(parse_ddg_html("class=\"result__a\" href=\"").is_empty());
        assert!(parse_bing_html("").is_empty());
        assert!(parse_bing_html("<li class=\"b_algo\"><h2><a href=").is_empty());
    }

    /// S13⑤：S2 出网口径——空白名单=默认放开；显式白名单=收紧（含子域）。
    #[test]
    fn test_s13_egress_s2_and_restricted() {
        // 默认放开（S2：用户拍板全部默认允许）
        assert!(egress_allowed_open(Some("duckduckgo.com"), &[]));
        let wl = vec!["duckduckgo.com".to_string()];
        assert!(egress_allowed_open(Some("html.duckduckgo.com"), &wl));
        assert!(!egress_allowed_open(Some("evil.com"), &wl));
        // 认不出主机 → 收紧模式下拒绝（fail-closed）
        assert!(!egress_allowed_open(None, &wl));
        assert!(!egress_allowed_open(Some("notduckduckgo.com"), &wl));
    }

    /// S13⑥：出网被拒时给出可行动错误（不真发请求）；参数缺失走结构化报错。
    #[tokio::test]
    async fn test_s13_denied_and_bad_args() {
        let tool = WebSearchTool::new();
        let mut env = HashMap::new();
        env.insert(
            "HEARTH_EGRESS_ALLOWLIST".to_string(),
            "example.com".to_string(),
        );
        let ctx = ToolContext {
            env,
            ..Default::default()
        };
        let r = tool
            .execute(serde_json::json!({"query": "rust"}), &ctx)
            .await;
        let msg = r.expect_err("白名单外必须拒绝").to_string();
        assert!(
            msg.contains("出网被拒") && msg.contains("HEARTH_EGRESS_ALLOWLIST"),
            "拒绝错误须可行动，实际 {msg}"
        );

        let ctx2 = ToolContext::default();
        let r2 = tool.execute(serde_json::json!({}), &ctx2).await;
        assert!(
            r2.expect_err("缺 query 必须报错")
                .to_string()
                .contains("missing 'query'"),
            "缺参错误须指出字段名"
        );
    }

    /// S13⑦：五种既有形态的描述必须含五要素（顶层附加审计的机器判据在
    /// tools-builtin/src/lib.rs 的 R5-9 测试里覆盖全部工具；此处校验本工具）。
    #[test]
    fn test_s13_web_search_description_five_elements() {
        let d = WebSearchTool::new().description();
        for marker in ["何时用", "何时不用", "示例", "边界", "错误解读"] {
            assert!(
                d.description.contains(marker),
                "web_search 描述缺要素「{marker}」：\n{}",
                d.description
            );
        }
        assert!(d.description.contains("query"), "参数说明必须可被模型理解");
        assert_eq!(d.parameters["required"][0], "query");
    }
}
