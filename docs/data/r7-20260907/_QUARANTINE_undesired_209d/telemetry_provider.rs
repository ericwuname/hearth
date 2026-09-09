//! R2-C 采集器 v2（补充 1）：TelemetryProvider 装饰器——钩子下沉到 gateway 层。
//!
//! R2-1 采集器 v1 只接 loop 的 plan-chat 出口，planner 直连 provider chat
//! 绕过采集（decompose/reflect 全部漏采）——"全部 LLM 请求必经"不成立。
//! v2 在 provider 构造点（composition root）包一层装饰器：**任何走该
//! provider 的 chat（loop/planner/sub-agent）都被记录**，一处覆盖全部出口。
//!
//! 开关：env `HEARTH_CACHE_TELEMETRY`（与 v1 同名，默认关零开销）。
//! 记录字段与 v1 兼容（jsonl 同 schema），另加 `exit` 字段区分调用来源。

use crate::cache_telemetry;
use crate::ChatRequest;
use crate::{ChatResponse, Embedding, LlmProvider, StreamEvent};
use anyhow::Result;
use futures::stream::BoxStream;

/// 装饰器：透传全部 trait 方法，chat 成功后追加一条遥测。
pub struct TelemetryProvider {
    inner: std::sync::Arc<dyn LlmProvider>,
    /// 调用来源标签（"cli"/"service"/"replay"——区分 composition root）。
    exit: String,
}

impl TelemetryProvider {
    pub fn wrap(inner: std::sync::Arc<dyn LlmProvider>, exit: &str) -> Self {
        Self {
            inner,
            exit: exit.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for TelemetryProvider {
    fn name(&self) -> &str {
        self.inner.name()
    }
    fn model(&self) -> &str {
        self.inner.model()
    }
    fn capabilities(&self) -> crate::Capabilities {
        self.inner.capabilities()
    }

    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse> {
        let resp = self.inner.chat(req.clone()).await;
        if let Ok(ref r) = resp {
            // v2：全部出口单点记录（loop 的 plan chat / planner 的 decompose
            // 与 reflect chat / 任何 sub-agent——共用同一 provider 实例）。
            // Closure-1/2（守门员补充 1）: chain/tools_hash 在采集器内计算——
            // schema 三项（msg_chain/tools_hash/phase）一次落。
            cache_telemetry::record_chat_full(
                // session_id 经 env 侧通道（CLI spawn 前 set HEARTH_TELEMETRY_SID）
                &std::env::var("HEARTH_TELEMETRY_SID").unwrap_or_default(),
                self.inner.name(),
                self.inner.model(),
                &req.messages,
                r.usage.as_ref(),
                &self.exit,
                &req.tools,
            );
        }
        resp
    }

    fn stream(&self, req: ChatRequest) -> BoxStream<'static, Result<StreamEvent>> {
        self.inner.stream(req)
    }

    async fn embed(&self, inputs: &[String]) -> Result<Vec<Embedding>> {
        self.inner.embed(inputs).await
    }
}
