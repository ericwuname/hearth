// v9.0: Autonomous task orchestrator — executes multi-step plans with retry and branching.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// A single step in a task plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStep {
    pub name: String,
    pub tool: String,
    pub args: serde_json::Value,
    #[serde(default = "default_retries")]
    pub max_retries: u32,
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub condition: StepCondition,
    pub capture: Option<String>,
}

fn default_retries() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StepCondition {
    #[default]
    Always,
    OnSuccess,
    OnFailure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step_name: String,
    pub status: StepStatus,
    pub output: serde_json::Value,
    pub attempts: u32,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Success,
    Failure,
    Skipped,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskReport {
    pub ok: bool,
    pub total_steps: usize,
    pub completed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub results: Vec<StepResult>,
    pub captured: HashMap<String, serde_json::Value>,
}

pub struct TaskOrchestrator;

impl TaskOrchestrator {
    pub async fn execute<F, Fut>(steps: &[TaskStep], mut execute_tool: F) -> TaskReport
    where
        F: FnMut(String, serde_json::Value) -> Fut + Send,
        Fut: std::future::Future<Output = Result<serde_json::Value, String>> + Send,
    {
        let mut results = Vec::new();
        let captured: HashMap<String, serde_json::Value> = HashMap::new();
        let mut last_status: Option<StepStatus> = None;

        for step in steps {
            let should_run = match step.condition {
                StepCondition::Always => true,
                StepCondition::OnSuccess => matches!(last_status, None | Some(StepStatus::Success)),
                StepCondition::OnFailure => matches!(last_status, Some(StepStatus::Failure)),
            };

            if !should_run {
                results.push(StepResult {
                    step_name: step.name.clone(),
                    status: StepStatus::Skipped,
                    output: serde_json::Value::Null,
                    attempts: 0,
                    duration_ms: 0,
                });
                continue;
            }

            let mut attempts = 0u32;
            let t0 = Instant::now();
            let (status, output) = loop {
                attempts += 1;
                let result = execute_tool(step.tool.clone(), step.args.clone()).await;
                match result {
                    Ok(val) => break (StepStatus::Success, val),
                    Err(_) if attempts <= step.max_retries => {
                        if step.max_retries > 1 {
                            tokio::time::sleep(Duration::from_millis(500)).await;
                        }
                        continue;
                    }
                    Err(e) => break (StepStatus::Failure, serde_json::Value::String(e)),
                }
            };

            last_status = Some(status.clone());
            results.push(StepResult {
                step_name: step.name.clone(),
                status,
                output,
                attempts,
                duration_ms: t0.elapsed().as_millis() as u64,
            });
        }

        let completed = results
            .iter()
            .filter(|r| r.status == StepStatus::Success)
            .count();
        let failed = results
            .iter()
            .filter(|r| r.status == StepStatus::Failure)
            .count();
        let skipped = results
            .iter()
            .filter(|r| r.status == StepStatus::Skipped)
            .count();

        TaskReport {
            ok: failed == 0,
            total_steps: steps.len(),
            completed,
            failed,
            skipped,
            results,
            captured,
        }
    }
}

// ── v9.0.2: Pipeline — chain tools with variable substitution ──

/// Substitutes {{key}} placeholders in a JSON value from captured step outputs.
pub fn resolve_vars(
    value: &serde_json::Value,
    captured: &HashMap<String, serde_json::Value>,
) -> serde_json::Value {
    let s = value.to_string();
    let resolved = resolve_string(&s, captured);
    serde_json::from_str(&resolved).unwrap_or_else(|_| value.clone())
}

fn resolve_string(s: &str, captured: &HashMap<String, serde_json::Value>) -> String {
    let mut result = s.to_string();
    for (key, val) in captured {
        let placeholder = format!("{{{{{}}}}}", key);
        let replacement = match val {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        result = result.replace(&placeholder, &replacement);
    }
    result
}

/// Executes a pipeline of steps, resolving {{key}} variables from previous captured outputs.
pub struct PipelineRunner;

impl PipelineRunner {
    pub async fn run<F, Fut>(steps: &[TaskStep], mut execute_tool: F) -> TaskReport
    where
        F: FnMut(String, serde_json::Value) -> Fut + Send,
        Fut: std::future::Future<Output = Result<serde_json::Value, String>> + Send,
    {
        let mut captured: HashMap<String, serde_json::Value> = HashMap::new();
        let mut results = Vec::new();

        for step in steps {
            let resolved_args = resolve_vars(&step.args, &captured);
            let mut attempts = 0u32;
            let t0 = std::time::Instant::now();
            let (status, output) = loop {
                attempts += 1;
                match execute_tool(step.tool.clone(), resolved_args.clone()).await {
                    Ok(val) => break (StepStatus::Success, val),
                    Err(_) if attempts <= step.max_retries => {
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        continue;
                    }
                    Err(e) => break (StepStatus::Failure, serde_json::Value::String(e)),
                }
            };

            if let Some(ref key) = step.capture {
                captured.insert(key.clone(), output.clone());
            }
            results.push(StepResult {
                step_name: step.name.clone(),
                status,
                output,
                attempts,
                duration_ms: t0.elapsed().as_millis() as u64,
            });
        }

        let completed = results
            .iter()
            .filter(|r| r.status == StepStatus::Success)
            .count();
        let failed = results
            .iter()
            .filter(|r| r.status == StepStatus::Failure)
            .count();
        let skipped = results
            .iter()
            .filter(|r| r.status == StepStatus::Skipped)
            .count();

        TaskReport {
            ok: failed == 0,
            total_steps: steps.len(),
            completed,
            failed,
            skipped,
            results,
            captured,
        }
    }
}

// ── v9.0.3: TaskValidator — validate step outputs ──

/// Validates a step result against expected keys and types.
pub struct TaskValidator;

impl TaskValidator {
    /// Check that `output` contains all expected keys with correct types.
    pub fn validate(output: &serde_json::Value, expected: &[(&str, &str)]) -> Result<(), String> {
        for (key, ty) in expected {
            match output.get(key) {
                None => return Err(format!("missing key '{}'", key)),
                Some(v) => {
                    let ok = match *ty {
                        "string" => v.is_string(),
                        "number" => v.is_number(),
                        "bool" => v.is_boolean(),
                        "array" => v.is_array(),
                        "object" => v.is_object(),
                        _ => true,
                    };
                    if !ok {
                        return Err(format!("key '{}' expected {}, got {:?}", key, ty, v));
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_empty_plan() {
        let report =
            TaskOrchestrator::execute(&[], |_, _| async { Ok(serde_json::Value::Null) }).await;
        assert!(report.ok);
    }

    #[tokio::test]
    async fn test_simple_success() {
        let steps = vec![TaskStep {
            name: "hello".into(),
            tool: "echo".into(),
            args: serde_json::json!({"msg": "hi"}),
            max_retries: 1,
            timeout_secs: None,
            condition: StepCondition::Always,
            capture: None,
        }];
        let report =
            TaskOrchestrator::execute(&steps, |_, _| async { Ok(serde_json::json!({"ok": true})) })
                .await;
        assert!(report.ok);
        assert_eq!(report.completed, 1);
    }

    #[tokio::test]
    async fn test_condition_on_failure_skips() {
        let mk = |name: &str, tool: &str, cond: StepCondition| TaskStep {
            name: name.into(),
            tool: tool.into(),
            args: serde_json::json!({}),
            max_retries: 1,
            timeout_secs: None,
            condition: cond,
            capture: None,
        };
        let steps = vec![
            mk("failer", "boom", StepCondition::Always),
            mk("cleanup", "ok", StepCondition::OnFailure),
            mk("unreachable", "noop", StepCondition::OnSuccess),
        ];
        let report = TaskOrchestrator::execute(&steps, |tool, _| async move {
            if tool == "boom" {
                Err("kaboom".into())
            } else {
                Ok(serde_json::json!({"ok": true}))
            }
        })
        .await;
        // failer fails → cleanup runs (OnFailure) and succeeds → unreachable runs (OnSuccess, prev=Success)
        assert!(report
            .results
            .iter()
            .any(|r| r.step_name == "failer" && r.status == StepStatus::Failure));
        assert!(report
            .results
            .iter()
            .any(|r| r.step_name == "cleanup" && r.status == StepStatus::Success));
        assert!(report
            .results
            .iter()
            .any(|r| r.step_name == "unreachable" && r.status == StepStatus::Success));
    }

    #[tokio::test]
    async fn test_pipeline_variable_substitution() {
        let steps = vec![
            TaskStep {
                name: "first".into(),
                tool: "echo".into(),
                args: serde_json::json!({"msg": "hello"}),
                max_retries: 1,
                timeout_secs: None,
                condition: StepCondition::Always,
                capture: Some("greeting".into()),
            },
            TaskStep {
                name: "second".into(),
                tool: "wrap".into(),
                args: serde_json::json!({"text": "{{greeting}}"}),
                max_retries: 1,
                timeout_secs: None,
                condition: StepCondition::Always,
                capture: None,
            },
        ];
        let report = PipelineRunner::run(&steps, |tool, args| async move {
            match tool.as_str() {
                "echo" => Ok(serde_json::Value::String(args["msg"].as_str().unwrap_or("?").to_string())),
                "wrap" => Ok(serde_json::json!({"wrapped": format!("[{}]", args["text"].as_str().unwrap_or("?"))})),
                _ => Err("unknown".into()),
            }
        }).await;
        assert!(report.ok);
        // Second step received resolved "hello" instead of "{{greeting}}"
        let wrap_result = report
            .results
            .iter()
            .find(|r| r.step_name == "second")
            .unwrap();
        assert_eq!(wrap_result.output["wrapped"], "[hello]");
    }

    #[test]
    fn test_validator_checks_keys() {
        assert!(TaskValidator::validate(
            &serde_json::json!({"ok": true, "count": 3}),
            &[("ok", "bool"), ("count", "number")]
        )
        .is_ok());
    }

    #[test]
    fn test_validator_rejects_missing_key() {
        let err = TaskValidator::validate(
            &serde_json::json!({"ok": true}),
            &[("ok", "bool"), ("missing", "string")],
        )
        .unwrap_err();
        assert!(err.contains("missing key"));
    }

    #[test]
    fn test_validator_rejects_wrong_type() {
        let err = TaskValidator::validate(&serde_json::json!({"ok": "yes"}), &[("ok", "bool")])
            .unwrap_err();
        assert!(err.contains("ok"));
    }
}

// ── v10.1: execute_plan — bridge between TaskOrchestrator and AgentLoop ──

/// Execute a pre-planned sequence of TaskSteps using a caller-supplied tool runner.
///
/// The closure `runner` receives (tool_name, args) and must return the tool output.
/// This adapts to any executor (ToolDispatcher, mock, etc).
pub async fn execute_plan<F, Fut>(runner: &mut F, steps: Vec<TaskStep>) -> TaskReport
where
    F: FnMut(String, serde_json::Value) -> Fut + Send,
    Fut: std::future::Future<Output = Result<serde_json::Value, String>> + Send,
{
    TaskOrchestrator::execute(&steps, move |tool, args| {
        let tool = tool.to_string();
        let args = args.clone();
        let fut = runner(tool.clone(), args);
        Box::pin(async move { fut.await.map_err(|e| format!("{tool}: {e}")) })
    })
    .await
}

#[cfg(test)]
mod execute_plan_tests {
    use super::*;

    #[tokio::test]
    async fn test_execute_plan_3_step_chain() {
        let mut call_log: Vec<String> = vec![];
        let steps = vec![
            TaskStep {
                name: "step1".into(),
                tool: "bash".into(),
                args: serde_json::json!({"cmd": "echo step1"}),
                capture: None,
                max_retries: 1,
                timeout_secs: None,
                condition: StepCondition::Always,
            },
            TaskStep {
                name: "step2".into(),
                tool: "edit".into(),
                args: serde_json::json!({"path": "a.txt", "old": "x", "new": "y"}),
                capture: Some("edited".into()),
                max_retries: 1,
                timeout_secs: None,
                condition: StepCondition::Always,
            },
            TaskStep {
                name: "step3".into(),
                tool: "bash".into(),
                args: serde_json::json!({"cmd": "echo done"}),
                capture: None,
                max_retries: 1,
                timeout_secs: None,
                condition: StepCondition::Always,
            },
        ];

        let report = execute_plan(
            &mut |tool, args| {
                let s = format!(
                    "{tool}:{}",
                    args.get("cmd")
                        .or(args.get("path"))
                        .map(|v| v.as_str().unwrap_or("?"))
                        .unwrap_or("?")
                );
                call_log.push(s.clone());
                async move { Ok(serde_json::Value::String(s)) }
            },
            steps,
        )
        .await;

        assert!(report.ok);
        assert_eq!(report.completed, 3);
        assert_eq!(report.total_steps, 3);
        assert_eq!(report.failed, 0);
        assert_eq!(call_log.len(), 3);
        assert!(call_log[0].contains("step1"));
        assert!(call_log[2].contains("done"));
    }
}
