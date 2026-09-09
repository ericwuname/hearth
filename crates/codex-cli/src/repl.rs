//! Interactive REPL: multi-turn conversation with approval handling.

use crate::client::CodexClient;
use crate::render;
use anyhow::Result;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;

/// R2 (v0.1.4): REPL 预算常量——lib.rs 调用方引用（此前仅用于会话元数据，未接 agent 循环）。
pub(crate) const REPL_BUDGET: u64 = 40;

pub async fn run(base_url: String, api_key: Option<String>) -> Result<()> {
    use futures::StreamExt as _; // needed for stream.next() in tokio::select!
    let client = std::sync::Arc::new(CodexClient::new(base_url.clone(), api_key.clone()));
    render::info(&format!("Connected to {base_url}"));
    render::info("Type a goal to start, /sessions to list, /quit to exit.");

    let stdin = tokio::io::stdin();
    let reader = BufReader::new(stdin);
    let mut lines = reader.lines();

    loop {
        // Prompt
        print!("\ncodex> ");
        use std::io::Write;
        std::io::stdout().flush().ok();

        let line = match lines.next_line().await {
            Ok(Some(l)) => l,
            _ => break,
        };
        let line = line.trim().to_string();

        if line.is_empty() {
            continue;
        }

        // Dispatch built-in commands
        if line == "/quit" || line == "/q" {
            break;
        }
        if line == "/sessions" {
            match client.list_sessions().await {
                Ok(sessions) => {
                    if sessions.is_empty() {
                        render::info("(no active sessions)");
                    } else {
                        for s in &sessions {
                            println!(
                                "  {} [{}] steps={:?} budget_remaining={:?}",
                                s.id, s.status, s.steps, s.budget_remaining
                            );
                        }
                    }
                }
                Err(e) => render::error(&format!("list sessions: {e}")),
            }
            continue;
        }
        if line.starts_with("/history ") {
            let sid = line["/history ".len()..].trim();
            match client.get_history(sid).await {
                Ok(h) => {
                    let msgs = h.get("messages").and_then(|m| m.as_array());
                    if let Some(msgs) = msgs {
                        for m in msgs {
                            let role = m.get("role").and_then(|r| r.as_str()).unwrap_or("?");
                            let content = m.get("content").and_then(|c| c.as_str()).unwrap_or("");
                            let prefix = if role == "user" { "▸" } else { "▹" };
                            println!("  {} [{}] {}", prefix, role, content);
                        }
                    }
                }
                Err(e) => render::error(&format!("history: {e}")),
            }
            continue;
        }
        if line.starts_with("/status ") {
            let sid = line["/status ".len()..].trim();
            match client.get_status(sid).await {
                Ok(s) => println!("{}", serde_json::to_string_pretty(&s).unwrap_or_default()),
                Err(e) => render::error(&format!("status: {e}")),
            }
            continue;
        }
        if line.starts_with("/cancel ") {
            let sid = line["/cancel ".len()..].trim();
            match client.cancel_session(sid).await {
                Ok(()) => render::info(&format!("cancelled {sid}")),
                Err(e) => render::error(&format!("cancel: {e}")),
            }
            continue;
        }

        // Otherwise: treat as a goal → create session → chat
        let goal = line;
        let sid = match client.create_session(&goal, REPL_BUDGET, "deepseek").await {
            Ok(id) => {
                render::info(&format!("session {} created", &id[..8.min(id.len())]));
                id
            }
            Err(e) => {
                render::error(&format!("create session: {e}"));
                continue;
            }
        };

        // Start SSE stream
        let stream = match client.stream_chat(&sid, &goal).await {
            Ok(s) => s,
            Err(e) => {
                render::error(&format!("chat: {e}"));
                continue;
            }
        };

        // Read SSE events and also listen for user approval commands
        let (tx, mut rx) = mpsc::channel::<String>(32);
        let client_arc = client.clone();
        let sid_clone = sid.clone();

        // Spawn stdin reader for approval commands
        tokio::spawn(async move {
            let stdin = tokio::io::stdin();
            let reader = BufReader::new(stdin);
            let mut lines = reader.lines();
            loop {
                let line = match lines.next_line().await {
                    Ok(Some(l)) => l.trim().to_string(),
                    _ => break,
                };
                if tx.send(line).await.is_err() {
                    break;
                }
            }
        });

        // Process SSE stream
        tokio::pin!(stream);
        let mut done_seen = false;
        loop {
            tokio::select! {
                event = stream.next() => {
                    match event {
                        Some(Ok(sse)) => {
                            match sse.event_type.as_str() {
                                "Phase" => {
                                    if let Some(p) = sse.data.as_str() {
                                        render::phase(p);
                                    }
                                }
                                "Token" => {
                                    if let Some(d) = sse.data.as_str() {
                                        render::token(d);
                                    }
                                }
                                "ToolCall" | "Tool" => {
                                    let name = sse.data.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                                    let args = sse.data.get("args").unwrap_or(&serde_json::Value::Null);
                                    render::tool_call(name, args);
                                }
                                "ToolResult" => {
                                    // W4/RC20: 结构化 is_error——失败走 ✗
                                    let output = sse.data.get("output").and_then(|o| o.as_str()).unwrap_or("");
                                    let is_error = sse
                                        .data
                                        .get("is_error")
                                        .and_then(|v| v.as_bool())
                                        .unwrap_or(false);
                                    if is_error {
                                        render::error(&format!("工具出错: {output}"));
                                    } else {
                                        render::tool_result(output);
                                    }
                                }
                                "NeedApproval" => {
                                    let aid = sse.data.get("approval_id").and_then(|a| a.as_str()).unwrap_or("?");
                                    let action = sse.data.get("action").and_then(|a| a.as_str()).unwrap_or("?");
                                    render::need_approval(&sid_clone, aid, action);

                                    // Wait for user approval (with 30s timeout)
                                    match tokio::time::timeout(
                                        std::time::Duration::from_secs(30),
                                        rx.recv(),
                                    ).await {
                                        Ok(Some(cmd)) => {
                                            let parts: Vec<&str> = cmd.split_whitespace().collect();
                                            if !parts.is_empty() && parts[0] == "approve" {
                                                let _ = client_arc.submit_approval(&sid_clone, aid, true, None).await;
                                            } else if !parts.is_empty() && parts[0] == "deny" {
                                                let _ = client_arc.submit_approval(&sid_clone, aid, false, None).await;
                                            }
                                        }
                                        _ => {
                                            // timeout → auto-deny
                                            let _ = client_arc.submit_approval(&sid_clone, aid, false, None).await;
                                            render::info("approval timeout — auto-denied");
                                        }
                                    }
                                }
                                "Reflection" | "Reflect" => {
                                    if let Some(v) = sse.data.as_str() {
                                        render::reflection(v);
                                    }
                                }
                                "Done" => {
                                    let steps = sse.data.get("steps").and_then(|s| s.as_u64()).unwrap_or(0);
                                    let ok = sse.data.get("ok").and_then(|o| o.as_bool()).unwrap_or(false);
                                    render::done(steps, ok);
                                    // ── R3-1 收尾三行（REPL 路径返工：与 one-shot
                                    // 同源事实——改了什么/还剩什么/依据）──
                                    if ok {
                                        let artifacts: Vec<String> = sse.data.get("artifacts")
                                            .and_then(|v| v.as_array())
                                            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                                            .unwrap_or_default();
                                        let pending: Vec<String> = sse.data.get("ledger_pending")
                                            .and_then(|v| v.as_array())
                                            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                                            .unwrap_or_default();
                                        let decision = sse.data.get("completion_decision")
                                            .and_then(|v| v.as_str()).unwrap_or("");
                                        let verification = sse.data.get("verification")
                                            .and_then(|v| v.as_str()).unwrap_or("UNVERIFIED");
                                        let known_failing: Vec<String> = sse.data.get("known_failing_open")
                                            .and_then(|v| v.as_array())
                                            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                                            .unwrap_or_default();
                                        // G-B 投影（REPL 同款）：未清已知失败非空 → 禁止裸 ✓ 语境
                                        if !known_failing.is_empty() {
                                            render::error(&format!(
                                                "  ⚠ G-B 拦截：{} 项已知失败未复测通过，本完成声明不可信、禁止计入验收通过：{:?}",
                                                known_failing.len(),
                                                known_failing
                                            ));
                                        }
                                        render::info(&format!(
                                            "  ── 收尾 | 改了什么: {}",
                                            if artifacts.is_empty() {
                                                "本轮无产物落盘".to_string()
                                            } else {
                                                format!("{}（{} 个）", artifacts.join("、"), artifacts.len())
                                            }
                                        ));
                                        render::info(&format!(
                                            "  ── 收尾 | 还剩什么: {}",
                                            if pending.is_empty() {
                                                "账本无未完成项".to_string()
                                            } else {
                                                format!("{}（{} 项未完成）", pending.join("；"), pending.len())
                                            }
                                        ));
                                        render::info(&format!(
                                            "  ── 收尾 | 依据: 完成决策 = {}；验证状态 = {verification}",
                                            if decision.is_empty() { "（无记录）" } else { decision }
                                        ));
                                    }
                                    done_seen = true;
                                }
                                "Error" => {
                                    if let Some(e) = sse.data.as_str() {
                                        render::error(e);
                                    }
                                }
                                _ => {}
                            }
                        }
                        Some(Err(e)) => {
                            render::error(&format!("SSE error: {e}"));
                            break;
                        }
                        None => break,
                    }
                }
            }
            if done_seen {
                break;
            }
        }
    }

    Ok(())
}

/// v0.1.1 (顺手·REPL 升级): 直跑模式 REPL——reedline 行编辑（方向键/历史/编辑）+
/// 多轮循环（读目标 → run_local 直跑内核 → 渲染 → 再读）。不做 Claude Code 的
/// 底部固定输入框流式重绘（中高难度，终端重绘竞态易花屏）——基础版输出完再提示。
pub async fn run_local_repl(cfg: &crate::config::ResolvedConfig, budget: u64) -> Result<()> {
    use reedline::{DefaultPrompt, DefaultPromptSegment, Reedline, Signal};

    // B2 (v0.1.3): 连续对话——整个 REPL 会话复用同一 AgentLoop（Thread 语义）：
    // ctx_mgr 历史跨轮保留，agent 引用前文。每轮 run_take 拿回 agent 状态。
    let provider = crate::run_local::build_provider(cfg)?;
    let workspace = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let dispatcher = crate::run_local::build_dispatcher(workspace.clone());
    let session_id = uuid::Uuid::new_v4().to_string();
    let goal0 = agent_core::Goal::with_budget(
        "conversation".to_string(),
        agent_types::Budget {
            max_steps: budget,
            ..Default::default()
        },
    );
    let snap_cwd = workspace.clone();
    let mut agent: Option<agent_core::AgentLoop> = Some(agent_core::AgentLoop::new(
        provider.clone(),
        std::sync::Arc::new(planner::DefaultPlanner::new(provider)),
        dispatcher.clone(),
        tool_runtime::ToolContext {
            cwd: workspace,
            env: crate::run_local::tool_env(cfg),
            ..Default::default()
        },
        goal0,
    ));
    crate::run_local::attach_session_safety(agent.as_mut().unwrap(), &session_id, &snap_cwd);
    if let Some(a) = agent.as_mut() {
        a.set_session_id(session_id.clone());
    }
    // B2: 每轮共用 session_id——observer/transcript 落盘到同一会话文件
    println!(
        "{}  (session {})",
        crate::run_local::isolation_badge(),
        &session_id[..8]
    );
    if let Some(w) = crate::run_local::cgroup_warning() {
        crate::render::info(&format!("⚠ cgroup: {w}"));
    }

    let mut line_editor = Reedline::create();
    let prompt = DefaultPrompt::new(
        DefaultPromptSegment::Basic("hearth> ".into()),
        DefaultPromptSegment::Empty,
    );
    crate::render::info(
        "Hearth REPL（直跑模式，连续对话）——输入目标开始，/quit 退出，Ctrl-D 退出。",
    );
    crate::render::info(
        "  📋 粘贴全文：输入 `{` 开始多行（贴完用单独一行 `}` 结束）；`/file <路径>` 直接读文件；运行中 Ctrl-C 取消本轮。",
    );
    // 盲区A (v0.2): 重启后自动恢复上次会话——提示最近本地会话（resume <id> 恢复）。
    // 落盘在每轮结束自动完成（session_store）；这里只做"窗口废了重开"的恢复引导。
    let local_sessions = crate::session_store::list_sessions();
    if !local_sessions.is_empty() {
        let (latest_id, latest_turns) = &local_sessions[0];
        if local_sessions.len() == 1 {
            crate::render::info(&format!(
                "  💾 检测到上次会话（{latest_id}，{latest_turns} 轮）——输入 `hearth resume {latest_id}` 可恢复继续"
            ));
        } else {
            crate::render::info(&format!(
                "  💾 检测到 {} 个本地会话（最近: {latest_id}，{latest_turns} 轮）——`hearth resume <id>` 恢复，`hearth sessions` 列出",
                local_sessions.len()
            ));
        }
    }
    loop {
        let line: Option<String> = match line_editor.read_line(&prompt) {
            Ok(Signal::CtrlC) | Ok(Signal::CtrlD) => None,
            Ok(Signal::Success(buf)) => Some(buf.trim().to_string()),
            Err(_) => {
                // reedline 光标查询失败（受限终端/测试 pty 不响应 ESC[6n）——
                // fallback 普通读取：功能可用（无行编辑），真实终端仍走 reedline。
                use std::io::Write as _;
                eprint!("hearth> ");
                let _ = std::io::stdout().flush();
                let mut buf = String::new();
                if std::io::stdin().read_line(&mut buf).is_err() {
                    None
                } else {
                    Some(buf.trim().to_string())
                }
            }
        };
        let Some(line) = line else { break };
        if matches!(line.as_str(), "/quit" | "/exit" | "/q") {
            break;
        }
        if line.is_empty() {
            continue;
        }
        // REPL 体验 (v0.2.1): 命令处理——/file 读文件全文 / 帮助
        if let Some(path) = line.strip_prefix("/file ") {
            match std::fs::read_to_string(path.trim()) {
                Ok(content) => {
                    let trimmed = content.trim().to_string();
                    if trimmed.is_empty() {
                        crate::render::error("文件为空");
                        continue;
                    }
                    crate::render::user_prompt(&format!(
                        "📄 /file {path}（{} 字符）",
                        trimmed.chars().count()
                    ));
                    submit(&mut agent, cfg, &session_id, &trimmed, budget).await?;
                    continue;
                }
                Err(e) => {
                    crate::render::error(&format!("读文件失败: {e}"));
                    continue;
                }
            }
        }
        if matches!(line.as_str(), "/help" | "/h") {
            crate::render::info(
                "命令：/quit 退出 · /file <路径> 读文件全文 · `{`…`}` 多行粘贴（贴完单独一行 } 提交）· trust on|off 会话级审批委托 · 运行中 Ctrl-C 取消本轮 · Ctrl-D 退出",
            );
            continue;
        }
        // RC29/RC24-C: 会话级审批委托——trust on / trust off（显式 opt-in，可随时撤销）
        if matches!(line.as_str(), "trust on" | "trust off") {
            let on = line == "trust on";
            use agent_core::ApprovalPolicy;
            if let Some(a) = agent.as_mut() {
                a.set_approval_policy(if on {
                    ApprovalPolicy::DelegateSession
                } else {
                    ApprovalPolicy::Interactive
                });
            }
            if on {
                crate::render::info(
                    "  🔓 trust on——会话级审批委托已开启：命令表级破坏性操作（rm/dd 等）自动放行，逐条记审计日志；fork bomb/设备写/内核接口（硬红线）仍需审批",
                );
            } else {
                crate::render::info("  🔒 trust off——已撤销委托，恢复逐条审批");
            }
            continue;
        }
        // REPL 体验 (v0.2.1): `{` 开头进入多行模式——收集到单独一行 `}` 提交（粘贴全文）
        let goal_text = if line.trim() == "{" {
            let mut lines: Vec<String> = Vec::new();
            crate::render::info("多行输入模式（粘贴全文，单独一行 } 结束，Ctrl-C 放弃）：");
            let mut cancelled = false;
            loop {
                let l = match line_editor.read_line(&prompt) {
                    Ok(Signal::CtrlC) | Ok(Signal::CtrlD) => {
                        cancelled = true;
                        break;
                    }
                    Ok(Signal::Success(buf)) => buf.trim().to_string(),
                    Err(_) => {
                        use std::io::Write as _;
                        eprint!("...> ");
                        let _ = std::io::stdout().flush();
                        let mut buf = String::new();
                        if std::io::stdin().read_line(&mut buf).is_err() {
                            break;
                        } else {
                            buf.trim().to_string()
                        }
                    }
                };
                if l == "}" {
                    break;
                }
                if l.is_empty() {
                    continue; // 空行保留在内容中（文档结构需要）
                }
                lines.push(l);
            }
            if cancelled || lines.is_empty() {
                crate::render::info("多行输入已放弃");
                continue;
            }
            lines.join("\n")
        } else {
            line
        };
        // B2: 每轮 run_take 复用 agent（历史跨轮保留）
        // X1-3 (v0.1.6): agent 崩溃（None）不退出 REPL——重建 agent 继续
        submit(&mut agent, cfg, &session_id, &goal_text, budget).await?;
        println!();
    }
    crate::render::info("bye 👋");
    Ok(())
}

/// REPL 体验 (v0.2.1): 单轮提交（Option 持有 agent——mem::take 语义；None=崩溃/取消 → 重建 + 恢复历史继续）。
async fn submit(
    agent: &mut Option<agent_core::AgentLoop>,
    cfg: &crate::config::ResolvedConfig,
    session_id: &str,
    goal_text: &str,
    budget: u64,
) -> anyhow::Result<()> {
    let current = match agent.take() {
        Some(a) => a,
        None => rebuild_agent(cfg, session_id, budget)?, // 兜底（正常不会走到）
    };
    match crate::run_local::run_local_continue(current, session_id, goal_text, budget, Vec::new())
        .await
    {
        Ok((Some(agent_back), report)) => {
            *agent = Some(agent_back);
            let _ = report;
        }
        Ok((None, report)) => {
            // agent 已丢失（panic 隔离 / 用户 Ctrl-C 取消）——重建 + 恢复历史继续
            // T-B (W8/N-2): cancelled 的 ⏹ 投影已统一到 run_local_continue 终态
            // 投影层（one-shot/REPL 各渲染一次）——此处不再重复渲染。
            let status = report
                .get("status")
                .and_then(|s| s.as_str())
                .unwrap_or("failed");
            if status != "cancelled" {
                crate::render::info("agent 已崩溃隔离——重建后继续（历史已恢复）");
            }
            let mut rebuilt = match rebuild_agent(cfg, session_id, budget) {
                Ok(a) => a,
                Err(e) => {
                    crate::render::error(&format!("agent 重建失败: {e:#}"));
                    return Err(e);
                }
            };
            // 取消/崩溃后恢复历史（上一轮快照）——继续对话不重说
            rebuilt.restore_history(crate::session_store::load_turns(session_id));
            *agent = Some(rebuilt);
        }
        Err(e) => {
            crate::render::error(&format!("{e:#}"));
            // 失败轮 agent 状态丢失——重建（保历史能力降级，但 REPL 不退出）
            let mut rebuilt = match rebuild_agent(cfg, session_id, budget) {
                Ok(a) => a,
                Err(e) => {
                    crate::render::error(&format!("agent 重建失败: {e:#}"));
                    return Err(e);
                }
            };
            rebuilt.restore_history(crate::session_store::load_turns(session_id));
            *agent = Some(rebuilt);
        }
    }
    Ok(())
}

/// X1-3 (v0.1.6): REPL 崩溃后重建 AgentLoop（同一 session_id——历史可经 resume 恢复）。
/// 实现已移至 run_local::rebuild_agent（resume 命令共用）。
fn rebuild_agent(
    cfg: &crate::config::ResolvedConfig,
    session_id: &str,
    budget: u64,
) -> anyhow::Result<agent_core::AgentLoop> {
    crate::run_local::rebuild_agent(cfg, session_id, budget)
}
