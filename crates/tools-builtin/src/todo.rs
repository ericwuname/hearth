use anyhow::{anyhow, Result};
use async_trait::async_trait;
use tool_runtime::{Tool, ToolContext, ToolDescription};

pub struct TodoWriteTool;

impl TodoWriteTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TodoWriteTool {
    fn default() -> Self {
        Self::new()
    }
}

/// R6-3（判定权归还长程任务书 v1.0）：TodoWrite——模型自持任务清单。
/// 判定权归还范式下的关键工具：框架不再用 planner 分解/T4 停滞计数替模型
/// 管任务，模型用本工具**自己维护**清单（Claude Code TodoWrite 语义）。
///
/// 状态设计：**无框架状态**——每次调用回显格式化清单进 tool result（对话
/// 历史），清单活在上下文里，模型自己读写。框架零记账、零停滞检测输入源
/// （T4 自擒链条从源头断一截），planner 独立分解调用随之减少。
#[async_trait]
impl Tool for TodoWriteTool {
    fn name(&self) -> &str {
        "todo_write"
    }

    fn description(&self) -> ToolDescription {
        ToolDescription {
            name: "todo_write".into(),
            description: "Write your own task checklist (replaces the whole list each call).\n\
                 何时用: 多步任务开工前列出计划；完成一项就更新状态；发现计划变了就重写清单。\n\
                 何时不用: 单步小任务（直接做，不需要清单）。\n\
                 示例: todo_write(todos=[{content:\"定位 bug\",status:\"completed\"},{content:\"写补丁\",status:\"in_progress\"},{content:\"跑测试\",status:\"pending\"}])。\n\
                 边界: 每次调用整体替换清单（不是增量追加）；status 只能是 pending/in_progress/completed；\n\
                 清单回显在工具输出里——历史里可查，无需另行记忆。\n\
                 错误解读: \"invalid status\"=状态词写错（三选一）；\"empty content\"=清单项内容为空。"
                .into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "todos": {
                        "type": "array",
                        "description": "Full replacement list, in order",
                        "items": {
                            "type": "object",
                            "properties": {
                                "content": {
                                    "type": "string",
                                    "description": "What this item is about"
                                },
                                "status": {
                                    "type": "string",
                                    "enum": ["pending", "in_progress", "completed"],
                                    "description": "Item state"
                                }
                            },
                            "required": ["content", "status"]
                        }
                    }
                },
                "required": ["todos"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, _ctx: &ToolContext) -> Result<String> {
        let todos = args
            .get("todos")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow!("missing 'todos' array argument"))?;
        if todos.is_empty() {
            return Err(anyhow!("empty 'todos' array — write at least one item"));
        }
        let mut out = String::from("Checklist (R6-3 模型自持清单):\n");
        let (mut done, mut active) = (0usize, 0usize);
        for (i, item) in todos.iter().enumerate() {
            let content = item
                .get("content")
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| anyhow!("item #{} has empty 'content'", i))?;
            let status = item
                .get("status")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("item #{} missing 'status'", i))?;
            let mark = match status {
                "completed" => {
                    done += 1;
                    "[x]"
                }
                "in_progress" => {
                    active += 1;
                    "[~]"
                }
                "pending" => "[ ]",
                other => {
                    return Err(anyhow!(
                        "item #{} invalid status '{other}' (pending|in_progress|completed only)",
                        i
                    ))
                }
            };
            out.push_str(&format!("{mark} {}. {content}\n", i + 1));
        }
        out.push_str(&format!(
            "— {}/{} completed, {} in progress —",
            done,
            todos.len(),
            active
        ));
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tool_runtime::ToolContext;

    /// R6-3 判据：模型可自主维护清单——合法写入回显格式化清单。
    #[tokio::test]
    async fn test_r63_todo_write_echoes_checklist() {
        let tool = TodoWriteTool::new();
        let ctx = ToolContext::default();
        let result = tool
            .execute(
                serde_json::json!({
                    "todos": [
                        {"content": "定位 bug", "status": "completed"},
                        {"content": "写补丁", "status": "in_progress"},
                        {"content": "跑测试", "status": "pending"}
                    ]
                }),
                &ctx,
            )
            .await
            .unwrap();
        assert!(result.contains("[x] 1. 定位 bug"), "{result:?}");
        assert!(result.contains("[~] 2. 写补丁"));
        assert!(result.contains("[ ] 3. 跑测试"));
        assert!(result.contains("1/3 completed"));
    }

    /// R6-3 反例：非法 status 必须结构化报错（不静默接受）。
    #[tokio::test]
    async fn test_r63_todo_write_rejects_invalid_status() {
        let tool = TodoWriteTool::new();
        let ctx = ToolContext::default();
        let result = tool
            .execute(
                serde_json::json!({
                    "todos": [{"content": "x", "status": "done"}]
                }),
                &ctx,
            )
            .await;
        let err = format!("{}", result.unwrap_err());
        assert!(err.contains("invalid status"), "{err:?}");
    }

    /// R6-3 边界：空清单拒绝（清单要有内容）。
    #[tokio::test]
    async fn test_r63_todo_write_rejects_empty() {
        let tool = TodoWriteTool::new();
        let ctx = ToolContext::default();
        let result = tool.execute(serde_json::json!({"todos": []}), &ctx).await;
        assert!(result.is_err());
    }
}
