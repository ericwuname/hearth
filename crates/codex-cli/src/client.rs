//! HTTP + SSE client for the codex service.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::time::Duration;

/// D-58（2026-10-01）：SSE 残行缓冲上限——单个事件不该超过它；超限即判为异常流并中断。
const MAX_SSE_BUF_BYTES: usize = 1024 * 1024;

/// D-58：**有界**读取远端响应体并解析 JSON。
///
/// 此前各调用点直接用 `resp.json()` / `resp.text()`——reqwest 无内建上限，对端（或
/// 中间人）返回超大响应即可把 CLI 打爆成 OOM。现统一走 `tools_builtin::read_body_capped`
/// 共享原语（Content-Length 提前拒绝 + 流式硬上限 1 MiB + 截断留痕），**单一实现**。
async fn json_capped(resp: reqwest::Response, what: &str) -> Result<serde_json::Value> {
    let (text, truncated) =
        tools_builtin::read_body_capped(resp, tools_builtin::MAX_BODY_BYTES).await?;
    if truncated {
        tracing::warn!(what, "响应体超上限已截断（JSON 可能不完整）");
    }
    serde_json::from_str(&text).with_context(|| format!("parse {what}"))
}

/// D-58：**有界**读取错误响应体（仅用于拼错误信息，绝不能因它 OOM）。
async fn error_body_capped(resp: reqwest::Response) -> String {
    tools_builtin::read_body_capped(resp, tools_builtin::MAX_BODY_BYTES)
        .await
        .map(|(t, _)| t)
        .unwrap_or_default()
}

/// Client for the codex HTTP service.
pub struct CodexClient {
    base_url: String,
    api_key: Option<String>,
    client: reqwest::Client,
}

/// A decoded SSE event from the service.
#[derive(Debug, Clone)]
pub struct SseEvent {
    pub event_type: String,
    pub data: serde_json::Value,
}

/// Convenience session info.
#[derive(Debug, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub status: String,
    pub steps: Option<u64>,
    pub budget_remaining: Option<u64>,
}

impl CodexClient {
    pub fn new(base_url: String, api_key: Option<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("reqwest client build");
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            client,
        }
    }

    /// POST /api/v1/sessions → create session, return session id.
    /// B4-1 (backend taskbook #01): 契约对齐——provider 必填 + budget 嵌套结构。
    pub async fn create_session(&self, goal: &str, budget: u64, provider: &str) -> Result<String> {
        let resp = self
            .request(reqwest::Method::POST, "/api/v1/sessions")
            .json(&serde_json::json!({
                "goal": goal,
                "provider": provider,
                "budget": { "max_steps": budget }
            }))
            .send()
            .await
            .context("create session failed")?;
        let body = json_capped(resp, "session response").await?;
        // B4-1: service 返回 {session_id, status}（契约对齐——旧读 body["id"] 永远拿不到）
        body["session_id"]
            .as_str()
            .or_else(|| body["id"].as_str())
            .map(String::from)
            .context("missing session id in response")
    }

    /// POST /api/v1/sessions/:id/messages → stream SSE events.
    pub async fn stream_chat(
        &self,
        session_id: &str,
        message: &str,
    ) -> Result<impl futures::Stream<Item = Result<SseEvent>>> {
        let resp = self
            .request(
                reqwest::Method::POST,
                &format!(
                    "/api/v1/sessions/{}/messages",
                    crate::pct_encode(session_id)
                ),
            )
            .json(&serde_json::json!({ "content": message }))
            .send()
            .await
            .context("send message failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = error_body_capped(resp).await;
            anyhow::bail!("server error {status}: {body}");
        }

        Ok(parse_sse_stream(resp))
    }

    /// GET /api/v1/sessions → list sessions.
    pub async fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        let resp = self
            .request(reqwest::Method::GET, "/api/v1/sessions")
            .send()
            .await
            .context("list sessions failed")?;
        let body = json_capped(resp, "sessions").await?;
        let sessions: Vec<SessionInfo> =
            serde_json::from_value(body["sessions"].clone()).unwrap_or_default();
        Ok(sessions)
    }

    /// GET /api/v1/sessions/:id/messages → history replay.
    pub async fn get_history(&self, session_id: &str) -> Result<serde_json::Value> {
        let resp = self
            .request(
                reqwest::Method::GET,
                &format!(
                    "/api/v1/sessions/{}/messages",
                    crate::pct_encode(session_id)
                ),
            )
            .send()
            .await
            .context("get history failed")?;
        json_capped(resp, "history").await
    }

    /// GET /api/v1/sessions/:id → session status.
    pub async fn get_status(&self, session_id: &str) -> Result<serde_json::Value> {
        let resp = self
            .request(
                reqwest::Method::GET,
                &format!("/api/v1/sessions/{}", crate::pct_encode(session_id)),
            )
            .send()
            .await
            .context("get status failed")?;
        // P1-6 (audit-fix): 检查状态码——404 应报"会话不存在"而非静默输出错误体。
        if !resp.status().is_success() {
            let status = resp.status();
            let body = error_body_capped(resp).await;
            anyhow::bail!("server error {status}: {body}");
        }
        json_capped(resp, "status").await
    }

    /// POST /api/v1/sessions/:id/interaction/:iid → 通用交互响应（WP-0）。
    /// approve/deny = kind="approval" 的交互，resolved=true/false。
    /// 旧 /approvals 路由保留为服务端薄适配层（deprecated）。
    pub async fn submit_approval(
        &self,
        session_id: &str,
        approval_id: &str,
        approve: bool,
        answer: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let mut payload = serde_json::json!({
            "decision": if approve { "approve" } else { "deny" },
        });
        // R10-C5 (v0.1.6): 澄清的文本/选项答案随审批一并带回（远程不丢文本）
        if let Some(a) = answer {
            payload["answer"] = a;
        }
        let resp = self
            .request(
                reqwest::Method::POST,
                &format!(
                    "/api/v1/sessions/{}/interaction/{}",
                    crate::pct_encode(session_id),
                    crate::pct_encode(approval_id)
                ),
            )
            .json(&serde_json::json!({
                "id": approval_id,
                "by": "human",
                "resolved": approve,
                "payload": payload,
                "latency_ms": null,
            }))
            .send()
            .await
            .context("submit interaction failed")?;
        json_capped(resp, "interaction response").await
    }

    /// POST /api/v1/sessions/:id/cancel → cancel session.
    pub async fn cancel_session(&self, session_id: &str) -> Result<()> {
        let resp = self
            .request(
                reqwest::Method::POST,
                &format!("/api/v1/sessions/{}/cancel", crate::pct_encode(session_id)),
            )
            .send()
            .await
            .context("cancel session failed")?;
        if resp.status().is_success() {
            Ok(())
        } else {
            anyhow::bail!("cancel returned {}", resp.status())
        }
    }

    fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.client.request(method, &url);
        if let Some(ref key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {key}"));
        }
        req
    }

    /// GET arbitrary JSON endpoint (for civ/workline etc).
    pub async fn get_json(&self, path: &str) -> Result<serde_json::Value> {
        let resp = self
            .request(reqwest::Method::GET, path)
            .send()
            .await
            .context("get_json failed")?;
        json_capped(resp, "json").await
    }

    /// POST arbitrary JSON to endpoint.
    pub async fn post_json(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let resp = self
            .request(reqwest::Method::POST, path)
            .json(body)
            .send()
            .await
            .context("post_json failed")?;
        json_capped(resp, "json").await
    }

    /// Y2: GET /healthz → probe service availability (text body, not JSON).
    pub async fn healthz(&self) -> Result<()> {
        let resp = self
            .request(reqwest::Method::GET, "/healthz")
            .send()
            .await
            .context("healthz failed")?;
        if resp.status().is_success() {
            Ok(())
        } else {
            anyhow::bail!("healthz returned {}", resp.status())
        }
    }
}

/// Parse a `text/event-stream` response body into a stream of SseEvent.
fn parse_sse_stream(resp: reqwest::Response) -> impl futures::Stream<Item = Result<SseEvent>> {
    let stream = tokio_stream::wrappers::ReceiverStream::new({
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<SseEvent>>(64);
        tokio::spawn(async move {
            use futures::StreamExt;
            let mut bytes = resp.bytes_stream();
            let mut buf = String::new();
            while let Some(chunk) = bytes.next().await {
                match chunk {
                    Ok(b) => {
                        buf.push_str(&String::from_utf8_lossy(&b));
                        // Y2: 纯函数解析（可单测）——处理完整事件，保留跨块残行
                        let (events, leftover) = parse_sse_lines(&buf);
                        for ev in events {
                            if tx.send(Ok(ev)).await.is_err() {
                                return;
                            }
                        }
                        buf = leftover;
                        // D-58：残行缓冲必须有上限——对端若不发换行地狂推字节，
                        // `buf` 会无界增长（无界读入的流式形态）。
                        if buf.len() > MAX_SSE_BUF_BYTES {
                            let _ = tx
                                .send(Err(anyhow::anyhow!(
                                    "SSE 残行缓冲超过 {MAX_SSE_BUF_BYTES} 字节上限——疑似异常流，已中断"
                                )))
                                .await;
                            return;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(anyhow::anyhow!("{e}"))).await;
                        return;
                    }
                }
            }
        });
        rx
    });
    stream
}

/// Y2: Pure SSE parser — split raw SSE text into (complete events, leftover partial).
/// Keeps chunked-stream semantics: an unterminated event (its `event:`/`data:` lines)
/// stays verbatim in `leftover` until the terminating blank line arrives.
fn parse_sse_lines(raw: &str) -> (Vec<SseEvent>, String) {
    let mut events = Vec::new();
    let mut event_type = String::new();
    let mut data = String::new();
    let mut rest = String::new();
    for line in raw.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            if !event_type.is_empty() || !data.is_empty() {
                let parsed: serde_json::Value =
                    serde_json::from_str(&data).unwrap_or(serde_json::Value::Null);
                events.push(SseEvent {
                    event_type: std::mem::take(&mut event_type),
                    data: parsed,
                });
                data.clear();
            }
            // 事件已终止：rest 清空（已完成事件的行不再需要保留）
            rest.clear();
        } else {
            // 未完成事件的行：原文保留到 rest（跨 chunk 续接），同时更新解析状态
            rest.push_str(line);
            if let Some(r) = trimmed.strip_prefix("event:") {
                event_type = r.trim().to_string();
            } else if let Some(r) = trimmed.strip_prefix("data:") {
                if !data.is_empty() {
                    data.push('\n');
                }
                data.push_str(r.trim());
            }
        }
    }
    (events, rest)
}

/// Y2: Classify an error into structured feedback (what / why / how).
/// Feedback surface: failure must be visible, localizable, actionable.
pub fn classify_error(err: &anyhow::Error) -> (&'static str, String, &'static str) {
    let msg = format!("{err:#}");
    let msg_lower = msg.to_lowercase();
    if msg_lower.contains("error sending request")
        || msg_lower.contains("connection refused")
        || msg_lower.contains("connect")
    {
        (
            "连不上 codex service",
            msg,
            "先启动 service（cargo run -p service），或检查 --url / CODEX_URL（默认 http://localhost:3000）",
        )
    } else if msg.contains("401") || msg_lower.contains("unauthorized") || msg.contains("403") {
        (
            "鉴权失败",
            msg,
            "运行 `codex setup` 配置 API key，或用 --api-key 传入",
        )
    } else if msg.contains("404") {
        (
            "路径不存在",
            msg,
            "检查 --url 是否指向 codex service（默认 http://localhost:3000）",
        )
    } else if msg_lower.contains("timeout") || msg_lower.contains("timed out") {
        (
            "请求超时",
            msg,
            "稍后重试，或检查 service 日志（RUST_LOG=debug）",
        )
    } else {
        (
            "执行失败",
            // P2 Node 10-13 决议（P4 Node 13 落地）：诊断消息截断须显式标记
            // （错误细节是排障事实，静默砍尾会误导用户）。
            agent_types::truncate_marked(&msg, 160),
            "查看 service 日志：RUST_LOG=debug 运行后复现",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sse_single_event() {
        let raw = "event: Phase\ndata: {\"p\":\"plan\"}\n\n";
        let (events, rest) = parse_sse_lines(raw);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "Phase");
        assert_eq!(events[0].data["p"], "plan");
        assert!(rest.is_empty());
    }

    #[test]
    fn parse_sse_chunked_partial_line() {
        // chunk 1: 无终止空行（事件跨块，event/data 行保留原文）
        let (e1, rest1) = parse_sse_lines("event: Token\ndata: \"hel");
        assert!(e1.is_empty());
        assert!(rest1.contains("hel"), "rest = {rest1:?}");
        // chunk 2: 续上（闭合 JSON 字符串）+ 空行终止
        let (e2, rest2) = parse_sse_lines(&format!("{rest1}lo\"\n\n"));
        assert_eq!(e2.len(), 1);
        assert_eq!(e2[0].event_type, "Token");
        assert_eq!(e2[0].data.as_str().unwrap(), "hello");
        assert!(rest2.is_empty());
    }

    #[test]
    fn parse_sse_tool_call_event() {
        let raw = "event: ToolCall\ndata: {\"name\":\"bash\",\"args\":{\"cmd\":\"ls\"}}\n\n";
        let (events, _) = parse_sse_lines(raw);
        assert_eq!(events[0].event_type, "ToolCall");
        assert_eq!(events[0].data["name"], "bash");
    }

    #[test]
    fn parse_sse_multiple_events() {
        let raw =
            "event: Phase\ndata: \"plan\"\n\nevent: Done\ndata: {\"steps\":3,\"ok\":true}\n\n";
        let (events, _) = parse_sse_lines(raw);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, "Phase");
        assert_eq!(events[1].event_type, "Done");
        assert_eq!(events[1].data["steps"], 3);
    }

    #[test]
    fn classify_connection_refused() {
        let e = anyhow::anyhow!("error sending request for url (http://localhost:3000/healthz)");
        let (w, _y, h) = classify_error(&e);
        assert!(w.contains("连不上"), "what = {w}");
        assert!(h.contains("service"), "how = {h}");
    }

    #[test]
    fn classify_unauthorized() {
        let e = anyhow::anyhow!("server error 401 Unauthorized: bad key");
        let (w, _y, _h) = classify_error(&e);
        assert!(w.contains("鉴权"), "what = {w}");
    }

    #[test]
    fn classify_timeout() {
        let e = anyhow::anyhow!("request timed out");
        let (w, _y, _h) = classify_error(&e);
        assert!(w.contains("超时"), "what = {w}");
    }

    #[test]
    fn classify_generic() {
        let e = anyhow::anyhow!("some unknown failure happened");
        let (w, _y, _h) = classify_error(&e);
        assert_eq!(w, "执行失败");
    }

    // ── D-58：远端响应体有界读取 ──

    /// 起一次性本地 HTTP 服务（回固定 body），返回 `http://host:port`。
    async fn serve_once(content_length: usize, body_len: usize) -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            if let Ok((mut sock, _)) = listener.accept().await {
                let mut req = [0u8; 1024];
                let _ = sock.read(&mut req).await;
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
                     Content-Length: {content_length}\r\nConnection: close\r\n\r\n"
                );
                let _ = sock.write_all(head.as_bytes()).await;
                let chunk = vec![b'a'; 64 * 1024];
                let mut left = body_len;
                while left > 0 {
                    let n = left.min(chunk.len());
                    if sock.write_all(&chunk[..n]).await.is_err() {
                        break;
                    }
                    left -= n;
                }
                let _ = sock.flush().await;
            }
        });
        format!("http://{addr}")
    }

    /// 先红后绿（D-58）：CLI 也必须**有界**读远端响应体。修复前走 `resp.json()` →
    /// 整份入内存（本测只会拿到 "parse json" 的解析错，而非"响应体过大"的拒绝）。
    #[tokio::test]
    async fn test_d58_get_json_rejects_oversized_body() {
        let url = serve_once(5_000_000, 5_000_000).await;
        let client = CodexClient::new(url, None);
        let err = client
            .get_json("/")
            .await
            .expect_err("超大响应必须被拒绝（不整份读入）");
        assert!(
            err.to_string().contains("响应体过大"),
            "须为有界拒绝而非解析失败：{err}"
        );
    }
}
