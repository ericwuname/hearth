//! D-214（2026-10-08, traecode）**行为级**门禁：`PhaseOverride::Simplify` 必须**真的被消费**。
//!
//! 病灶（负债 D-214）：`CostGuard` 在 cost>80%/95% 产 `PhaseOverride::Simplify`，但调用点
//! **只 `tracing::warn!`** ⇒ "资源压力 → 简化"是**空转**（已设计未接线）。
//!
//! 本门禁：小预算（`HEARTH_COST_BUDGET_USD`）+ 高价表（`HEARTH_PRICE_TABLE`）+ 首轮上报巨额
//! `usage` ⇒ 第二步 plan 时 `cost_ratio` 触顶 ⇒ CostGuard 产 `Simplify` ⇒ 断言**第二步**
//! 的 LLM 请求**关闭了工具**（`tools` 为空）且**降低了 `max_tokens`**；第**一**步作反向对照
//! （工具齐全、常规上限）。修复前第二步与第一步同形 ⇒ **RED**。
//!
//! 盲区（自报）：只覆盖 `Simplify` 的请求整形语义（关闭工具 + 降上限）；`Abandon` 的终止路径
//! 由既有单测覆盖。本文件为**独立测试二进制**⇒ env 隔离（不污染同进程其他测试）。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use agent_core::{Agent, AgentLoop, Goal};
use agent_types::Budget;
use anyhow::Result;
use async_trait::async_trait;
use futures::stream::{self, BoxStream};
use llm_gateway::{
    Capabilities, ChatRequest, ChatResponse, Embedding, LlmProvider, StreamEvent, Usage,
};
use tool_runtime::{ToolContext, ToolDispatcher};
use tools_builtin::BashTool;

/// 单次请求的**可验证形状**（本门禁只关心工具数与输出上限）。
#[derive(Debug, Clone)]
struct ReqShape {
    tools: usize,
    max_tokens: Option<u32>,
}

struct RecordingLlm {
    calls: Mutex<Vec<ReqShape>>,
    n: AtomicUsize,
}

fn usage() -> Option<Usage> {
    // 巨额用量（配 1e6 USD/1M token 的价表 ⇒ 单轮即数万美元）
    Some(Usage {
        prompt_tokens: 10_000,
        completion_tokens: 10_000,
        total_tokens: 20_000,
        prompt_cache_hit_tokens: None,
        prompt_cache_miss_tokens: None,
    })
}

#[async_trait]
impl LlmProvider for RecordingLlm {
    fn name(&self) -> &str {
        "r214-mock"
    }
    fn model(&self) -> &str {
        "mock"
    }
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            chat: true,
            stream: false,
            function_calling: true,
            embeddings: false,
            max_context_tokens: Some(8192),
        }
    }
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse> {
        self.calls.lock().unwrap().push(ReqShape {
            tools: req.tools.len(),
            max_tokens: req.max_tokens,
        });
        let n = self.n.fetch_add(1, Ordering::SeqCst);
        Ok(if n == 0 {
            // 首轮：发起一个工具调用（迫使进入 Act → Reflect → 第二次 plan）
            ChatResponse {
                finish_reason: Some("tool_calls".into()),
                content: Some(String::new()),
                tool_calls: vec![agent_types::ToolCall {
                    call_id: "c1".into(),
                    name: "bash".into(),
                    args: serde_json::json!({"cmd": "echo hi"}),
                }],
                reasoning_content: None,
                usage: usage(),
            }
        } else {
            ChatResponse {
                finish_reason: Some("stop".into()),
                content: Some("任务已完成：已跑 echo，输出 hi。".into()),
                tool_calls: vec![],
                reasoning_content: None,
                usage: usage(),
            }
        })
    }
    fn stream(&self, _req: ChatRequest) -> BoxStream<'static, Result<StreamEvent>> {
        Box::pin(stream::empty())
    }
    async fn embed(&self, _inputs: &[String]) -> Result<Vec<Embedding>> {
        Ok(vec![])
    }
}

#[tokio::test]
async fn simplify_override_really_disables_tools_and_lowers_budget() {
    // 极小额预算 + 极高价表 ⇒ 首轮 usage 记入后 cost_ratio 触顶（CostGuard ⇒ Simplify）。
    std::env::set_var("HEARTH_COST_BUDGET_USD", "0.000001");
    std::env::set_var(
        "HEARTH_PRICE_TABLE",
        r#"{"prices":{"r214-mock/mock":{"input_per_1m_usd":1000000.0,"output_per_1m_usd":1000000.0}}}"#,
    );

    let dir = tempfile::tempdir().unwrap();
    let llm = Arc::new(RecordingLlm {
        calls: Mutex::new(Vec::new()),
        n: AtomicUsize::new(0),
    });
    let mut dispatcher = ToolDispatcher::new();
    dispatcher.register(Arc::new(BashTool::new()));

    let mut agent = AgentLoop::new(
        llm.clone(),
        Arc::new(dispatcher),
        ToolContext {
            cwd: dir.path().to_path_buf(),
            ..Default::default()
        },
        Goal::new("d214 probe"),
    );
    let _ = agent
        .run(Goal::with_budget(
            "跑 echo 并汇报",
            Budget {
                max_steps: 4,
                max_time_secs: None,
                ..Budget::default()
            },
        ))
        .await;

    let calls = llm.calls.lock().unwrap().clone();
    assert!(
        calls.len() >= 2,
        "D-214：至少须有两次 plan（第二步才是简化步）；实得 {}",
        calls.len()
    );
    // 反向对照：第一步**不得**被简化（此时 cost_ratio 仍为 0）。
    assert!(
        calls[0].tools > 0,
        "D-214 对照：第一步（cost_ratio=0）工具必须齐全，实得 tools={}",
        calls[0].tools
    );
    // 正题：第二步（cost 触顶 ⇒ Simplify）必须关闭工具 + 降上限。
    assert_eq!(
        calls[1].tools, 0,
        "D-214：Simplify 命中后本步必须**关闭工具**（修复前与第一步同形 ⇒ 空转）；实得 tools={}",
        calls[1].tools
    );
    let expect = (agent_types::max_output_tokens() / 4).max(256);
    assert_eq!(
        calls[1].max_tokens,
        Some(expect),
        "D-214：Simplify 命中后本步 `max_tokens` 必须降到 {}（修复前为常规上限）",
        expect
    );
    assert!(
        expect < agent_types::max_output_tokens(),
        "D-214 自检：简化上限须严格小于常规上限（否则'降上限'名不副实）"
    );

    std::env::remove_var("HEARTH_COST_BUDGET_USD");
    std::env::remove_var("HEARTH_PRICE_TABLE");
}
