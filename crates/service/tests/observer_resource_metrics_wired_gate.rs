//! D-219（2026-10-08, traecode）**行为级**门禁：observer ④ 资源效率指标必须由**真实会话**喂出数据。
//!
//! 病灶（D-218 登记）：`Done` 事件 payload 从不含 `usage` ⇒ observer ④ `token_efficiency` /
//! `cost_per_step` **无任何可达数据源**、生产恒 0，规则 `cost_per_step_high` 永不触发。
//!
//! 本门禁：脚本化 provider（每轮都上报 `usage`）+ 接上 Observer 真跑一个会话 ⇒ 读 observer
//! 落盘的 `report.json`，断言 ④ `token_efficiency` 是**非零实数**（修复前恒 0.0 ⇒ RED）。
//!
//! 盲区（自报）：测试环境的 provider/model 无价格条目 ⇒ `cost_per_step` 应为 `null`
//! （**成本未知**，不是 0）——"已定价"一侧由 `observer::metrics` 的单测覆盖。

use std::sync::{Arc, Mutex};

use anyhow::Result;
use api::SessionCreate;
use async_trait::async_trait;
use futures::stream::{self, BoxStream};
use llm_gateway::{
    Capabilities, ChatRequest, ChatResponse, Embedding, LlmProvider, ProviderRegistry, StreamEvent,
    Usage,
};
use tool_runtime::{ToolContext, ToolDispatcher};
use tools_builtin::{EditTool, GlobTool, GrepTool, ReadTool};

struct ScriptedProvider {
    name: String,
    responses: Mutex<Vec<ChatResponse>>,
}

impl ScriptedProvider {
    fn new(name: &str, responses: Vec<ChatResponse>) -> Self {
        Self {
            name: name.to_string(),
            responses: Mutex::new(responses),
        }
    }
}

fn usage() -> Option<Usage> {
    Some(Usage {
        prompt_tokens: 300,
        completion_tokens: 600,
        total_tokens: 900,
        prompt_cache_hit_tokens: None,
        prompt_cache_miss_tokens: None,
    })
}

#[async_trait]
impl LlmProvider for ScriptedProvider {
    fn name(&self) -> &str {
        &self.name
    }
    fn model(&self) -> &str {
        "mock-model"
    }
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            chat: true,
            stream: true,
            function_calling: true,
            embeddings: false,
            max_context_tokens: Some(8192),
        }
    }
    async fn chat(&self, _req: ChatRequest) -> Result<ChatResponse> {
        let mut q = self.responses.lock().unwrap();
        if !q.is_empty() {
            return Ok(q.remove(0));
        }
        Ok(ChatResponse {
            content: Some("DONE".into()),
            tool_calls: vec![],
            finish_reason: Some("stop".into()),
            usage: usage(),
            reasoning_content: None,
        })
    }
    fn stream(&self, _req: ChatRequest) -> BoxStream<'static, Result<StreamEvent>> {
        Box::pin(stream::empty())
    }
    async fn embed(&self, _inputs: &[String]) -> Result<Vec<Embedding>> {
        Ok(vec![])
    }
}

fn stop(content: &str) -> ChatResponse {
    ChatResponse {
        content: Some(content.into()),
        tool_calls: vec![],
        finish_reason: Some("stop".into()),
        usage: usage(),
        reasoning_content: None,
    }
}

#[tokio::test]
async fn observer_resource_metrics_are_fed_by_real_sessions() {
    let obs_dir = std::env::temp_dir().join(format!("hearth-d219-{}", uuid::Uuid::new_v4()));
    std::env::set_var("CODEX_OBSERVER_DIR", &obs_dir);

    let mut registry = ProviderRegistry::new();
    registry.register(Arc::new(ScriptedProvider::new(
        "mock-metrics",
        vec![stop("DONE")],
    )));
    let registry = Arc::new(registry);

    let mut dispatcher = ToolDispatcher::new();
    dispatcher.register(Arc::new(ReadTool::new()));
    dispatcher.register(Arc::new(EditTool::new()));
    dispatcher.register(Arc::new(GlobTool::new()));
    dispatcher.register(Arc::new(GrepTool::new()));
    let dispatcher = Arc::new(dispatcher);

    let mut mgr =
        service::session::SessionManager::new(registry, dispatcher, ToolContext::default());
    mgr.set_observer(Arc::new(observer::Observer::new()));
    let mgr = Arc::new(mgr);

    let created = mgr
        .create_session(SessionCreate {
            provider: "mock-metrics".into(),
            model: None,
            goal: "explain the resource metrics".into(),
            budget: None,
        })
        .await
        .expect("建会话");
    let sid = created.session_id.clone();
    let mut rx = mgr
        .send_message(
            &sid,
            api::MessageReq {
                content: "go".into(),
            },
        )
        .await
        .expect("发消息");

    let timeout = tokio::time::sleep(std::time::Duration::from_secs(30));
    tokio::pin!(timeout);
    loop {
        tokio::select! {
            r = rx.recv() => match r {
                Ok(evt) => { if matches!(evt, api::AgentEvent::Done { .. }) { break; } }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                Err(_) => continue,
            },
            _ = &mut timeout => break,
        }
    }

    // observer 报告落盘在 <CODEX_OBSERVER_DIR>/reports/<sid>/report.json
    let report_path = obs_dir.join("reports").join(&sid).join("report.json");
    let raw = std::fs::read_to_string(&report_path)
        .unwrap_or_else(|e| panic!("须有 observer 报告 {:?}: {e}", report_path));
    let v: serde_json::Value = serde_json::from_str(&raw).expect("report.json 可解析");
    let metrics = v.get("metrics").expect("报告须含 metrics");

    let eff = metrics
        .get("token_efficiency")
        .expect("metrics 须含 token_efficiency");
    assert!(
        eff.as_f64().is_some_and(|x| x > 0.0),
        "D-219：真实会话（每轮上报 usage）跑完，token_efficiency 必须是非零实数\
         （修复前恒 0.0——Done payload 无 usage）；实得 {eff}"
    );
    // 成本侧：本测试 provider/model 无价格条目 ⇒ `cost_usd` = null ⇒ 成本**未知**（null，不是 0）。
    let cps = metrics
        .get("cost_per_step")
        .expect("metrics 须含 cost_per_step");
    assert!(
        cps.is_null() || cps.as_f64().is_some_and(|x| x >= 0.0),
        "D-219：cost_per_step 须为 null（未定价 ⇒ 未知）或数值；实得 {cps}"
    );

    let _ = std::fs::remove_dir_all(&obs_dir);
    std::env::remove_var("CODEX_OBSERVER_DIR");
}
