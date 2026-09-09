//! WS8 (v0.2): introspect 体感工具——LLM 主动查询"身体状态"（steps/预算/
//! 上下文填充/相位/写盘数/provider）。数据来自 AgentLoop 每步写入的
//! ToolContext.scratch["body"]（零新通道，复用既有 ctx 传递）。
//! 目的：大脑（LLM）能"感觉"身体（Hearth）——撞预算前感知疲劳、主动摘要/询问
//! （用户隐喻"高功能瘫痪"的反面：身体如实上报，大脑主动内观）。

use anyhow::Result;
use async_trait::async_trait;
use tool_runtime::{Tool, ToolContext, ToolDescription};

pub struct IntrospectTool;

impl IntrospectTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for IntrospectTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for IntrospectTool {
    fn name(&self) -> &str {
        "introspect"
    }

    fn description(&self) -> ToolDescription {
        ToolDescription {
            name: "introspect".into(),
            description: "查询 Hearth 自身运行状态（体感内观）：当前相位、已用步数/预算上限、上下文填充百分比、已写文件数、provider。在感觉任务要超预算或上下文膨胀时调用，主动向用户汇报或请求重新评估。".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }

    async fn execute(&self, _args: serde_json::Value, ctx: &ToolContext) -> Result<String> {
        // body 状态由 AgentLoop 每步写入 scratch["body"]（update_body_state）
        let body = ctx
            .scratch
            .get("body")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({ "note": "body state not available yet" }));
        let mut out = body.as_object().cloned().unwrap_or_default();
        out.insert(
            "workspace".to_string(),
            serde_json::Value::String(ctx.cwd.display().to_string()),
        );
        // 环境中的 provider/模型信息（若 CLI 注入）
        for k in ["HEARTH_PROVIDER", "HEARTH_MODEL"] {
            if let Some(v) = ctx.env.get(k) {
                out.insert(k.to_string(), serde_json::Value::String(v.clone()));
            }
        }
        Ok(serde_json::to_string_pretty(&serde_json::Value::Object(
            out,
        ))?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// WS8: introspect 返回 scratch["body"]（body 状态可解析 JSON）；无 body 时
    /// 返回可解析 JSON 不 panic。body 缺失时此测试红（体感通道没接上）。
    #[tokio::test]
    async fn test_introspect_returns_body() {
        let tool = IntrospectTool::new();
        let ctx = ToolContext::default();
        // 无 body（agent 未注入）——仍返回可解析 JSON
        let r = tool.execute(serde_json::json!({}), &ctx).await.unwrap();
        let v: serde_json::Value = serde_json::from_str(&r).unwrap();
        assert!(v.get("workspace").is_some(), "introspect 应含 workspace");

        // 注入 body 后返回完整状态
        let mut ctx2 = ToolContext::default();
        if let Some(obj) = ctx2.scratch.as_object_mut() {
            obj.insert(
                "body".to_string(),
                serde_json::json!({
                    "phase": "Act",
                    "steps_used": 7,
                    "budget_max_steps": 40,
                    "compact_pressure_pct": 62,
                    "written_files": 2,
                    "provider": "deepseek",
                }),
            );
        }
        let r2 = tool.execute(serde_json::json!({}), &ctx2).await.unwrap();
        let v2: serde_json::Value = serde_json::from_str(&r2).unwrap();
        assert_eq!(v2["steps_used"], 7, "body 状态应透传");
        assert_eq!(v2["provider"], "deepseek");
        assert_eq!(
            v2["compact_pressure_pct"], 62,
            "距压缩阈值压力应可见（疲劳信号，RC40 改名后语义如实）"
        );
    }
}
