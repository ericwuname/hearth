//! HTTP + SSE client for the codex service.

use anyhow::{Context, Result};
use futures::StreamExt;
use serde::Deserialize;
use std::time::Duration;

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
        let body: serde_json::Value = resp.json().await.context("parse session response")?;
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
                &format!("/api/v1/sessions/{session_id}/messages"),
            )
            .json(&serde_json::json!({ "content": message }))
            .send()
            .await
            .context("send message failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
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
        let body: serde_json::Value = resp.json().await.context("parse sessions")?;
        let sessions: Vec<SessionInfo> =
            serde_json::from_value(body["sessions"].clone()).unwrap_or_default();
        Ok(sessions)
    }

    /// GET /api/v1/sessions/:id/messages → history replay.
    pub async fn get_history(&self, session_id: &str) -> Result<serde_json::Value> {
        let resp = self
            .request(
                reqwest::Method::GET,
                &format!("/api/v1/sessions/{session_id}/messages"),
            )
            .send()
            .await
            .context("get history failed")?;
        resp.json().await.context("parse history")
    }

    /// GET /api/v1/sessions/:id → session status.
    pub async fn get_status(&self, session_id: &str) -> Result<serde_json::Value> {
        let resp = self
            .request(
                reqwest::Method::GET,
                &format!("/api/v1/sessions/{session_id}"),
            )
            .send()
            .await
            .context("get status failed")?;
        // P1-6 (audit-fix): 检查状态码——404 应报"会话不存在"而非静默输出错误体。
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("server error {status}: {body}");
        }
        resp.json().await.context("parse status")
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
                &format!("/api/v1/sessions/{session_id}/interaction/{approval_id}"),
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
        resp.json().await.context("parse interaction response")
    }

    /// POST /api/v1/sessions/:id/cancel → cancel session.
    pub async fn cancel_session(&self, session_id: &str) -> Result<()> {
        let resp = self
            .request(
                reqwest::Method::POST,
                &format!("/api/v1/sessions/{session_id}/cancel"),
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
        resp.json().await.context("parse json")
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
        resp.json().await.context("parse json")
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
}
