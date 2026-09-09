use agent_types::{Budget, Message, MessageContent, Role, RunState, Turn};
use serde_json::Value;

/// Manages the conversation context, including history compression.
pub struct ContextManager {
    /// T3 (v0.2.3): 系统提示词字符数（build_messages 算完后记录）——introspect 暴露
    /// 真实总负载（此前油箱表只计 history，漏掉 system_text 这块固定大开销）。
    system_chars: u64,

    /// H2 (v0.2.4): 本次 run 起点（deadline 判定用）。
    run_started_at: Option<std::time::Instant>,

    /// G3-03 (v0.2.5): 归属会话（压缩归档按会话隔离——archive/<sid>.jsonl）。
    /// 空 = 未绑定（旧调用方/纯测试），回落共享归档文件。
    session_id: String,

    state: RunState,

    /// P2 Node 12: provider-aware 注入阈值（AgentLoop 按 caps 计算）。
    injected_compact_threshold: Option<usize>,
}

impl ContextManager {
    pub fn new(goal: String, budget: Budget) -> Self {
        Self {
            state: RunState::new(goal, budget),
            injected_compact_threshold: None,
            system_chars: 0,
            run_started_at: Some(std::time::Instant::now()),
            session_id: String::new(),
        }
    }

    /// G3-03: 绑定归属会话（AgentLoop::set_session_id 同步调用）。
    pub fn set_session_id(&mut self, sid: &str) {
        self.session_id = sid.to_string();
    }

    /// G3-03: 归属会话 id（空 = 未绑定）。
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// T3: 记录系统提示词体量（build_messages 算完调用）。
    pub fn set_system_chars(&mut self, chars: u64) {
        self.system_chars = chars;
    }

    /// T3: 系统提示词字符数。
    pub fn system_chars(&self) -> u64 {
        self.system_chars
    }

    /// T3: 真实总负载 = history + system_text（油箱表口径修正）。
    pub fn total_chars(&self) -> u64 {
        self.effective_estimate() as u64 + self.system_chars
    }

    /// Get the current run state (read-only).
    pub fn state(&self) -> &RunState {
        &self.state
    }

    /// Get mutable access to the run state.
    pub fn state_mut(&mut self) -> &mut RunState {
        &mut self.state
    }

    /// Record a completed turn.
    pub fn record_turn(&mut self, turn: Turn) {
        self.state.history.push(turn);
    }

    /// Store a value in the scratch space.
    pub fn set_scratch(&mut self, key: &str, value: Value) {
        self.state.scratch.insert(key.to_string(), value);
    }

    /// Get a value from scratch space.
    pub fn get_scratch(&self, key: &str) -> Option<&Value> {
        self.state.scratch.get(key)
    }

    /// Check if budget is exhausted.
    pub fn budget_exhausted(&self) -> bool {
        self.state.steps_used >= self.state.budget.max_steps
    }

    /// H2 (v0.2.4): 任务级 wall-clock deadline——预算耗尽判定。
    /// `max_time_secs` 此前是死字段（构造后无人消费——手工实测 hearth chat 在
    /// 工具层反复失败/重试间打转数小时不退出，需人工 kill）。现在激活：
    /// 超时即与步数耗尽同语义（走同一干净收尾路径）。
    pub fn deadline_exceeded(&self) -> bool {
        match self.state.budget.max_time_secs {
            None => false,
            Some(cap) => self.run_elapsed_secs() >= cap,
        }
    }

    /// P1-LTR-01: 任务级绝对截止时刻（H2 同源派生——run_started_at + max_time_secs，
    /// **不建第二套 deadline 类型**）。None = 无时间上限。
    /// 每轮 continue_turn 重置 run_started_at → 本值每轮自然重建（6.2）；
    /// Instant 不持久化（6.3）——resume 时按新一轮预算重建。
    pub fn task_deadline(&self) -> Option<std::time::Instant> {
        match self.state.budget.max_time_secs {
            None => None,
            Some(cap) => self
                .run_started_at
                .map(|t| t + std::time::Duration::from_secs(cap)),
        }
    }

    /// H2 (v0.2.4): 本次 run 已流逝秒数（run_started_at 起）。
    /// None = 未记录起点（无法判定，视为未超时）。
    pub fn run_elapsed_secs(&self) -> u64 {
        self.run_started_at
            .map(|t| t.elapsed().as_secs())
            .unwrap_or(0)
    }

    /// H2 (v0.2.4): 记录 run 起点（continue_turn / new 时调用）。
    pub fn mark_run_start(&mut self) {
        self.run_started_at = Some(std::time::Instant::now());
    }

    /// Increment step counter.
    pub fn inc_step(&mut self) {
        self.state.steps_used += 1;
    }

    /// Number of steps used so far.
    pub fn steps_used(&self) -> u64 {
        self.state.steps_used
    }

    /// Append a user message to the conversation history (F1: multi-turn support).
    pub fn add_user_message(&mut self, content: String) {
        // If the latest turn exists, push into it; otherwise start a new turn
        if let Some(turn) = self.state.history.last_mut() {
            turn.messages.push(Message::new(
                "user".into(),
                Role::User,
                MessageContent::Text(content),
            ));
        } else {
            let mut turn = agent_types::Turn::new(self.state.history.len() as u64);
            turn.messages.push(Message::new(
                "user".into(),
                Role::User,
                MessageContent::Text(content),
            ));
            self.state.history.push(turn);
        }
    }

    /// B2 (v0.1.3): 连续对话（对齐 Codex Thread/Turn）——保留历史（会话记忆），
    /// 更新 goal/budget 并重置 run 级计数。REPL 每轮调用，agent 能引用前文。
    pub fn continue_turn(&mut self, goal: String, budget: Budget) {
        self.state.goal = goal;
        self.state.budget = budget;
        self.state.steps_used = 0;
        self.state.tokens_used = 0;
        // H2 (v0.2.4): 每轮新 run 重置 deadline 起点。
        self.mark_run_start();
    }

    // ── WS4 (v0.1.5): 轻量 compaction ──
    // 长会话 token 管理（v0.2 任务书 WS4 必做项）。规则式摘要、不调 LLM——
    // 确定性、可测试、不阻塞（v0.2 设计铁律：compaction 先于提步数）。
    // 旧轮次折叠为一条摘要消息（保留每轮 goal + 工具清单 + 写盘文件，不丢架构决策）。

    /// 触发阈值：历史消息字符粗估超过此值即压缩（≈32k chars ≈ 8k tokens）。
    pub const COMPACT_CHAR_THRESHOLD: usize = 32_000;

    /// 保留最近 N 轮完整（细节留给当前工作区）。
    pub const COMPACT_KEEP_TURNS: usize = 2;

    /// 历史消息字符粗估——P2 Node 12 修正（批-3 仪器纪律）：此前用
    /// `format!("{:?}").len()`（Debug 包装 + UTF-8 字节）系统性虚增
    /// 1.39-2.49×（Node 01 实测），导致压缩提前触发。现按 content 的
    /// 真实字符计数（naive chars）。
    pub fn estimate_chars(&self) -> usize {
        self.state
            .history
            .iter()
            .flat_map(|t| &t.messages)
            .map(|m| match &m.content {
                agent_types::MessageContent::Text(s) => s.chars().count(),
                agent_types::MessageContent::ToolCalls(calls) => calls
                    .iter()
                    .map(|c| c.args.to_string().chars().count())
                    .sum(),
                agent_types::MessageContent::ToolResults(rs) => {
                    rs.iter().map(|r| r.output.chars().count()).sum()
                }
            })
            .sum()
    }

    /// 遗留口径（P1 旧行为：Debug 包装 + UTF-8 字节）——仅
    /// HEARTH_COMPACTION_MODE=legacy 回滚通道使用（§30，保留一个 release cycle）。
    pub fn legacy_estimate_chars(&self) -> usize {
        self.state
            .history
            .iter()
            .flat_map(|t| &t.messages)
            .map(|m| format!("{:?}", m.content).len())
            .sum()
    }

    /// 生效估算：正常 = naive chars；legacy 模式 = Debug+bytes（旧行为整保）。
    pub fn effective_estimate(&self) -> usize {
        if std::env::var("HEARTH_COMPACTION_MODE").as_deref() == Ok("legacy") {
            self.legacy_estimate_chars()
        } else {
            self.estimate_chars()
        }
    }

    /// P2 Node 12: 当前生效的压缩阈值（provider-aware，批-2 裁决）——
    /// 优先级：run 注入值（AgentLoop 按 provider caps×比例计算）>
    /// env HEARTH_COMPACT_CHAR_THRESHOLD（测试/实验）> 遗留常量（未知
    /// provider 回退）。`HEARTH_COMPACTION_MODE=legacy` 整体回滚旧口径
    /// （Debug+bytes est + 32k 常量——§30 回滚通道，保留一个 release cycle）。
    pub fn compact_char_threshold(&self) -> usize {
        if std::env::var("HEARTH_COMPACTION_MODE").as_deref() == Ok("legacy") {
            return Self::legacy_estimate_threshold();
        }
        // P2-CFR Node 06 修正（真机 COMPACT_DBG 实证）：env 测试仪器必须压过
        // provider-aware 注入——否则注入值（如 Agnes caps 128K→19.6 万字符）
        // 使 HEARTH_COMPACT_CHAR_THRESHOLD 完全失效（探针 est=68 thr=195840），
        // 受控压缩实验与 Node 13 类 stress 全部静默失灵。生产语义不变：
        // 未设 env 时仍走注入值（provider-aware 主路径）。
        if let Some(t) = Self::env_char_threshold() {
            return t;
        }
        if let Some(t) = self.injected_compact_threshold {
            return t;
        }
        Self::COMPACT_CHAR_THRESHOLD
    }

    /// 遗留口径阈值（HEARTH_COMPACTION_MODE=legacy 时配合 legacy est 使用）。
    fn legacy_estimate_threshold() -> usize {
        std::env::var("HEARTH_COMPACT_CHAR_THRESHOLD")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(Self::COMPACT_CHAR_THRESHOLD)
    }

    fn env_char_threshold() -> Option<usize> {
        std::env::var("HEARTH_COMPACT_CHAR_THRESHOLD")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
    }

    /// AgentLoop 按 provider caps 注入（caps.max_context_tokens × ratio ×
    /// chars/token 系数）。None = 未注入（未知 provider 走遗留常量）。
    pub fn set_compact_threshold(&mut self, threshold: usize) {
        self.injected_compact_threshold = Some(threshold);
    }

    /// P2 Node 01 实测系数（combined workload naive chars/token = 2.55）。
    pub const CHARS_PER_TOKEN: f64 = 2.55;

    /// 超阈值时折叠旧轮为摘要。返回是否发生了压缩。
    /// 摘要消息以 `[compacted]` 前缀标记，插入历史开头（保持时序）。
    /// H3 (v0.2.4): 折叠前原始轮次先落盘归档（archive_compacted_turns）——
    /// "压缩"不再等于"永久丢失"（手工实测：压缩后 agent 答不上任何被压缩
    /// 的历史细节，用户追问只能得到"无法读取"）。归档只 best-effort：
    /// 落盘失败不阻塞压缩（内存上下文管理优先），错误只记 tracing。
    pub fn maybe_compact(&mut self) -> bool {
        if self.effective_estimate() < self.compact_char_threshold() {
            return false;
        }
        let keep_from = self
            .state
            .history
            .len()
            .saturating_sub(Self::COMPACT_KEEP_TURNS);
        if keep_from == 0 {
            return false; // 轮数太少，无折叠意义
        }
        let old: Vec<Turn> = self.state.history.drain(..keep_from).collect();
        // H3: 先归档原文（best-effort），再折叠摘要
        // G3-03 (v0.2.5): 归档按会话隔离（archive/<sid>.jsonl）——多 session 不再
        // 堆进同一共享文件（检索命中别的会话内容=噪声+串扰）。
        if let Err(e) = archive_compacted_turns(&self.session_id, &old) {
            tracing::warn!(error = %e, "compaction archive write failed (non-fatal)");
        }
        let mut summary_msgs: Vec<Message> = Vec::new();
        for (i, t) in old.iter().enumerate() {
            summary_msgs.push(summarize_turn(i, t));
        }
        // 摘要插入 history 最前（保持对话时序，模型看到"早期决策摘要"）。
        // P0-ATTRIBUTION（env-gated）：dump 压缩摘要全文（归因用，零控制流影响）
        if std::env::var("HEARTH_DEBUG_PLANNER_INPUT").as_deref() == Ok("1") {
            let dbg_dir = std::env::var("HOME")
                .map(|h| std::path::PathBuf::from(h).join(".config/hearth/debug"))
                .unwrap_or_else(|_| std::path::PathBuf::from("/tmp/hearth-debug"));
            let sid = if self.session_id.is_empty() {
                "nosession".to_string()
            } else {
                self.session_id.clone()
            };
            let dir = dbg_dir.join(&sid);
            let _ = std::fs::create_dir_all(&dir);
            let millis = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);
            let mut body = String::new();
            for m in &summary_msgs {
                if let agent_types::MessageContent::Text(t) = &m.content {
                    body.push_str(t);
                    body.push_str("\n---\n");
                }
            }
            let _ = std::fs::write(dir.join(format!("compact-{millis}-summary.txt")), body);
        }
        let mut merged: Vec<Message> = summary_msgs;
        for turn in &mut self.state.history {
            merged.append(&mut turn.messages);
        }
        self.state.history.clear();
        if !merged.is_empty() {
            self.state.history.push(Turn {
                index: 0,
                messages: merged,
                actions: Vec::new(),
            });
        }
        // H3 (v0.2.4): 摘要头部注入归档检索提示——让模型知道自己有找回历史的手段，
        // 而不是回答"无法读取被压缩的历史"。
        // G3-03: 提示路径=本会话归档文件（隔离后 grep 不串别的会话）。
        if let Some(first) = self
            .state
            .history
            .first_mut()
            .and_then(|t| t.messages.first_mut())
        {
            let archive_display = archive_path(&self.session_id).display().to_string();
            let hint = format!(
                " [如需被压缩轮次的完整原文，用 bash: grep -n \"关键词\" {archive_display} 检索本会话归档]"
            );
            if !matches!(first.content, MessageContent::Text(ref s) if s.contains("archive/")) {
                if let MessageContent::Text(ref mut s) = first.content {
                    s.push_str(&hint);
                }
            }
        }
        tracing::info!(
            old_turns = old.len(),
            est_chars = self.estimate_chars(),
            "context compacted (WS4)"
        );
        true
    }
}

/// H3 (v0.2.4) + G3-03 (v0.2.5): 被压缩轮次的原文归档——按**会话隔离**追加写
/// JSONL：`archive/<session_id>.jsonl`（每会话独立文件，检索不串扰）。
/// `HEARTH_ARCHIVE_FILE` env 仍可整体覆盖（测试/自定义）；session_id 为空
/// （旧调用方/纯测试）回落共享文件 `compacted.jsonl`。
/// best-effort：失败由调用方降级（不阻塞压缩）。
pub(crate) fn archive_compacted_turns(session_id: &str, turns: &[Turn]) -> std::io::Result<()> {
    use std::io::Write;
    let path = archive_path(session_id);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    for t in turns {
        let line = serde_json::to_string(t)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        writeln!(f, "{line}")?;
    }
    Ok(())
}

/// H3/G3-03: 归档文件路径（env 覆盖 → 按 session 隔离 → 未绑定回落共享）。
pub(crate) fn archive_path(session_id: &str) -> std::path::PathBuf {
    if let Some(f) = std::env::var_os("HEARTH_ARCHIVE_FILE") {
        return std::path::PathBuf::from(f);
    }
    // 文件名安全：session_id 是 uuid 或自定义 id——过滤路径分隔符
    let name = if session_id.is_empty() {
        "compacted.jsonl".to_string()
    } else {
        let safe: String = session_id
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        format!("{safe}.jsonl")
    };
    if let Some(home) = std::env::var_os("HOME") {
        return std::path::PathBuf::from(home)
            .join(".config/hearth/archive")
            .join(name);
    }
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return std::path::PathBuf::from(appdata)
            .join("hearth/archive")
            .join(name);
    }
    std::path::PathBuf::from(".hearth_archive").join(name)
}

/// R5-6（智能性根治长程任务包 v1.0）：archive 读回通道——归档清单（digest）。
/// 根因七读侧：R2-2 落盘后"读侧全仓为 0"——事实换个地方销毁。本函数把
/// 已归档轮次解析回结构化清单（turn 索引 + 首条用户消息摘要），让模型在
/// 收到切片提示时**同时知道"归档里有什么"**，可答"还记得早期事实吗"
/// （引用归档内容），而非只拿到一个 grep 路径。
///
/// best-effort：文件不存在/解析失败 → None（调用方降级为原有提示）。
/// 同一 turn 多次归档（切片边界前移重写）按 index 去重——清单不膨胀。
pub(crate) fn archive_digest(session_id: &str, max_turns: usize) -> Option<String> {
    let path = archive_path(session_id);
    let content = std::fs::read_to_string(&path).ok()?;
    let mut seen: std::collections::HashSet<u64> = std::collections::HashSet::new();
    let mut lines: Vec<String> = Vec::new();
    for line in content.lines() {
        if lines.len() >= max_turns {
            break;
        }
        let Ok(turn) = serde_json::from_str::<Turn>(line) else {
            continue;
        };
        if !seen.insert(turn.index) {
            continue;
        }
        let head: String = turn
            .messages
            .iter()
            .filter(|m| matches!(m.role, agent_types::Role::User))
            .find_map(|m| match &m.content {
                agent_types::MessageContent::Text(t) => {
                    let trimmed = t.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.chars().take(120).collect::<String>())
                    }
                }
                _ => None,
            })
            .unwrap_or_else(|| "(no user text)".to_string());
        lines.push(format!("turn#{}: {head}", turn.index));
    }
    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

/// WS4: 单轮折叠为规则式摘要消息——goal（首条 user 文本）+ 工具清单 + 写盘文件。
/// 不调 LLM；截断长 goal（60 字符）防摘要本身膨胀。
fn summarize_turn(idx: usize, t: &Turn) -> Message {
    use agent_types::{MessageContent, Role};
    let goal = t
        .messages
        .iter()
        .find(|m| matches!(m.role, Role::User))
        .map(|m| format!("{:?}", m.content))
        .unwrap_or_default();
    // R8 (v0.1.6): 按字符计数/截取——`&goal[..60]` 按字节切，多字节字符（中文）中间切
    // 即 panic（真机 `end byte index 60 is not a char boundary`，整个 REPL 进程死）。
    let goal_short: String = if goal.chars().count() > 60 {
        format!("{}…", goal.chars().take(60).collect::<String>())
    } else {
        goal
    };
    // ── R2-3 语义摘要·内存侧（对话可用性根治任务书 v1.0）──
    // 旧：只有元数据（"轮次目标 60 字 | 工具名 | 写盘路径"）——名词清单没有
    // 动作语义，压缩后问"之前做了什么、为什么"答不出（W-D 前置缺口）。
    // 新：**动作-结果语义行**（确定性提取，零 LLM——不烧 token）：
    // 按 call_id 配对 ToolCalls↔ToolResults，产出"写了 `x` / 改了 `y` /
    // 执行 `cmd`（失败）/ 读取 N 次"的可读动作流 + 成败统计。失败动作带
    // （失败）标记——压缩后"之前为什么失败"也可答。
    // 与 R2-2 互补：归档管原文（grep 找回），摘要管快速理解。
    let mut results_by_call: std::collections::HashMap<String, &agent_types::ToolResult> =
        std::collections::HashMap::new();
    for m in &t.messages {
        if let MessageContent::ToolResults(rs) = &m.content {
            for r in rs {
                results_by_call.insert(r.call_id.clone(), r);
            }
        }
    }
    let mut actions: Vec<String> = Vec::new();
    let mut failed = 0usize;
    let mut total = 0usize;
    let mut reads = 0usize;
    for m in &t.messages {
        if let MessageContent::ToolCalls(calls) = &m.content {
            for c in calls {
                total += 1;
                let err = results_by_call
                    .get(&c.call_id)
                    .map(|r| r.is_error)
                    .unwrap_or(false);
                if err {
                    failed += 1;
                }
                let mark = if err { "（失败）" } else { "" };
                match c.name.as_str() {
                    "write_file" | "edit" | "apply_patch" => {
                        let p = c.args.get("path").and_then(|v| v.as_str()).unwrap_or("?");
                        let verb = if c.name == "write_file" {
                            "写了"
                        } else {
                            "改了"
                        };
                        actions.push(format!("{verb} `{p}`{mark}"));
                    }
                    "bash" => {
                        let cmd = c
                            .args
                            .get("command")
                            .and_then(|v| v.as_str())
                            .map(|s| {
                                s.lines()
                                    .next()
                                    .unwrap_or("")
                                    .chars()
                                    .take(60)
                                    .collect::<String>()
                            })
                            .unwrap_or_default();
                        actions.push(format!("执行 `{cmd}`{mark}"));
                    }
                    _ => {
                        reads += 1;
                    }
                }
            }
        }
    }
    if reads > 0 {
        actions.push(format!("读取/其他工具调用 {reads} 次"));
    }
    let actions_part = if actions.is_empty() {
        "无".to_string()
    } else {
        actions.join("；")
    };
    let status_part = if total == 0 {
        String::new()
    } else if failed == 0 {
        format!(" | 结果: {total} 个动作全部成功")
    } else {
        format!(" | 结果: {total} 个动作中 {failed} 个失败（见'（失败）'标记——压缩后'之前为什么失败'仍可答）")
    };
    Message::new(
        format!("summary-{idx}"),
        Role::Assistant,
        MessageContent::Text(format!(
            "[compacted 会话摘要] 轮次目标: {goal_short}\n动作: {actions_part}{status_part}"
        )),
    )
}

#[cfg(test)]
pub(crate) mod tests {
    // P3 观察到的 gate flake（123+1）：env-mutating 测试在 cargo 并行下互相竞争
    // （HEARTH_COMPACTION_MODE / HEARTH_COMPACT_CHAR_THRESHOLD / HEARTH_ARCHIVE_FILE /
    //  HOME）。以下 env 触碰测试统一持锁串行。
    pub(crate) static ENV_SER: std::sync::Mutex<()> = std::sync::Mutex::new(());
    use super::*;

    /// env 覆盖类测试串行锁（HEARTH_ARCHIVE_FILE 进程全局，防并行互踩）。
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_context_creation() {
        let ctx = ContextManager::new("test".into(), Budget::default());
        assert_eq!(ctx.state().goal, "test");
        assert!(!ctx.budget_exhausted());
    }

    /// R2-3 语义摘要·内存侧（对话可用性根治任务书 v1.0）：压缩摘要必须
    /// 携带**动作-结果语义**——"写了什么/执行了什么/哪个失败"压缩后可答
    /// （W-D 前置）。旧格式只有元数据（工具名/路径名词清单），无动作语义。
    /// 确定性提取零 LLM（不烧 token）。
    #[test]
    fn test_r23_semantic_summary_answers_what_and_why() {
        use agent_types::{Message, MessageContent, Role, ToolCall, ToolResult, Turn};
        let turn = Turn {
            index: 7,
            messages: vec![
                Message::new(
                    "g7".into(),
                    Role::User,
                    MessageContent::Text("做一个网页并跑测试".into()),
                ),
                Message::new(
                    "a7".into(),
                    Role::Assistant,
                    MessageContent::ToolCalls(vec![
                        ToolCall {
                            call_id: "c1".into(),
                            name: "write_file".into(),
                            args: serde_json::json!({"path": "plan.html", "content": "hi"}),
                        },
                        ToolCall {
                            call_id: "c2".into(),
                            name: "bash".into(),
                            args: serde_json::json!({"command": "cargo build --release"}),
                        },
                        ToolCall {
                            call_id: "c3".into(),
                            name: "read_file".into(),
                            args: serde_json::json!({"path": "README.md"}),
                        },
                    ]),
                ),
                Message::new(
                    "t7".into(),
                    Role::Tool,
                    MessageContent::ToolResults(vec![
                        ToolResult {
                            call_id: "c1".into(),
                            is_error: false,
                            output: "wrote 2 bytes".into(),
                            artifacts: vec![],
                            error_kind: None,
                        },
                        ToolResult {
                            call_id: "c2".into(),
                            is_error: true,
                            output: "error: could not compile".into(),
                            artifacts: vec![],
                            error_kind: None,
                        },
                        ToolResult {
                            call_id: "c3".into(),
                            is_error: false,
                            output: "ok".into(),
                            artifacts: vec![],
                            error_kind: None,
                        },
                    ]),
                ),
            ],
            actions: vec![],
        };
        let summary = match super::summarize_turn(7, &turn).content {
            MessageContent::Text(s) => s,
            other => panic!("摘要必须是 Text，实际 {other:?}"),
        };
        assert!(summary.contains("[compacted 会话摘要]"), "前缀兼容");
        assert!(
            summary.contains("写了 `plan.html`"),
            "写入动作语义可见，实际 {summary}"
        );
        assert!(
            summary.contains("执行 `cargo build --release`（失败）"),
            "bash 动作 + 失败标记可见（压缩后'之前为什么失败'可答），实际 {summary}"
        );
        assert!(summary.contains("1 个失败"), "成败统计可见，实际 {summary}");
        assert!(
            summary.contains("读取/其他工具调用 1 次"),
            "读取类聚合可见，实际 {summary}"
        );
        assert!(
            !summary.contains("| 工具: "),
            "旧元数据格式已升级（名词清单 → 动作语义行）"
        );
    }

    #[test]
    fn test_budget_exhausted() {
        let mut ctx = ContextManager::new(
            "test".into(),
            Budget {
                max_steps: 3,
                max_tokens: None,
                max_time_secs: None,
                ..Budget::default()
            },
        );
        ctx.inc_step();
        ctx.inc_step();
        ctx.inc_step();
        assert!(ctx.budget_exhausted());
    }

    #[test]
    fn test_add_user_message_survives_ctx_reset() {
        // F1: prove add_user_message writes into history, and a "reset"
        // (new ContextManager) doesn't carry old messages — the pending mechanism in AgentLoop handles that.
        let mut ctx = ContextManager::new("goal".into(), Budget::default());
        ctx.record_turn(agent_types::Turn::new(0));
        ctx.add_user_message("hello from user".into());

        // The message should be in history
        let has_user_msg = ctx.state().history.iter().any(|turn| {
            turn.messages.iter().any(|m| {
                matches!(m.role, agent_types::Role::User)
                    && format!("{:?}", m.content).contains("hello from user")
            })
        });
        assert!(
            has_user_msg,
            "user message should be in history after add_user_message"
        );

        // Now simulate what run() does: reset ctx_mgr
        let mut ctx2 = ContextManager::new("goal".into(), Budget::default());
        ctx2.record_turn(agent_types::Turn::new(0));
        // ctx2 is fresh — old message is NOT there (this is expected; AgentLoop.pending_user_messages bridges this)
        let has_old_msg = ctx2.state().history.iter().any(|turn| {
            turn.messages
                .iter()
                .any(|m| format!("{:?}", m.content).contains("hello from user"))
        });
        assert!(!has_old_msg, "fresh ContextManager should NOT have old messages (pending_user_messages bridges this)");
    }

    #[test]
    fn test_scratch() {
        let mut ctx = ContextManager::new("test".into(), Budget::default());
        ctx.set_scratch("key", serde_json::json!("val"));
        assert_eq!(ctx.get_scratch("key").unwrap(), &serde_json::json!("val"));
    }

    /// 回归 WS4 (v0.1.5): 历史超阈值时旧轮折叠为规则式摘要——
    /// 保留最近轮完整、旧轮摘要含 goal/工具/写盘文件、总字符显著下降。
    /// 无 compaction 时此测试红（长会话 token 腐烂）。
    #[test]
    fn test_maybe_compact_folds_old_turns() {
        let mut ctx = ContextManager::new("build a game".into(), Budget::default());
        // 造 8 个轮次（旧轮带 user goal + write_file 工具调用 + 大量填充文本）——
        // 8 轮 × ~9000 字符，压缩保留最近 2 轮后 after 应 < before/3。
        for i in 0..8 {
            let mut turn = Turn::new(i);
            turn.messages.push(Message::new(
                format!("u{i}"),
                agent_types::Role::User,
                agent_types::MessageContent::Text(format!(
                    "第{i}轮目标：写一个贪吃蛇游戏，文件放 snake_game/"
                )),
            ));
            turn.messages.push(Message::new(
                format!("a{i}"),
                agent_types::Role::Assistant,
                agent_types::MessageContent::ToolCalls(vec![agent_types::ToolCall {
                    call_id: format!("c{i}"),
                    name: "write_file".into(),
                    args: serde_json::json!({"path": "snake_game/index.html", "content": "x".repeat(9000)}),
                }]),
            ));
            turn.messages.push(Message::new(
                format!("r{i}"),
                agent_types::Role::Assistant,
                agent_types::MessageContent::Text("ok".into()),
            ));
            ctx.record_turn(turn);
        }
        let before = ctx.estimate_chars();
        assert!(
            before > ContextManager::COMPACT_CHAR_THRESHOLD,
            "测试夹具须超阈值, got {before}"
        );

        let compacted = ctx.maybe_compact();
        assert!(compacted, "超阈值必须触发压缩");
        let after = ctx.estimate_chars();
        assert!(
            after < before / 3,
            "压缩后字符应显著下降: before={before} after={after}"
        );

        // 摘要保留关键信息：goal 词 + 写盘文件路径
        let all: String = ctx
            .state()
            .history
            .iter()
            .flat_map(|t| &t.messages)
            .map(|m| format!("{:?}", m.content))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(all.contains("compacted"), "摘要消息须带 [compacted] 标记");
        assert!(all.contains("贪吃蛇"), "摘要须保留轮次目标, got: {all}");
        assert!(
            all.contains("snake_game/index.html"),
            "摘要须保留写盘文件, got: {all}"
        );
    }

    /// 回归 WS4 (v0.1.5): 短会话（低于阈值）不触发压缩（防误伤）。
    #[test]
    fn test_maybe_compact_skips_short_history() {
        let mut ctx = ContextManager::new("hi".into(), Budget::default());
        let mut turn = Turn::new(0);
        turn.messages.push(Message::new(
            "u".into(),
            agent_types::Role::User,
            agent_types::MessageContent::Text("hello".into()),
        ));
        ctx.record_turn(turn);
        assert!(!ctx.maybe_compact(), "短会话不应触发压缩（低于阈值）");
        assert_eq!(ctx.state().history.len(), 1, "历史不得被改动");
    }

    /// H2 (v0.2.4): max_time_secs deadline——此前是死字段（构造后无人消费）。
    /// 负面：elapsed >= cap 时 deadline_exceeded 必须为真（修复前恒 false）。
    #[test]
    fn test_deadline_exceeded_activates() {
        let ctx = ContextManager::new(
            "goal".into(),
            Budget {
                max_steps: 50,
                max_time_secs: Some(0), // cap=0：起点已过即超时（确定性触发）
                ..Budget::default()
            },
        );
        assert!(
            ctx.deadline_exceeded(),
            "cap=0 时任何流逝时间都应判超时（修复前恒 false——死字段）"
        );
        // 无上限 → 永不超时
        let ctx2 = ContextManager::new("goal".into(), Budget::default());
        assert!(!ctx2.deadline_exceeded(), "max_time_secs=None 不判超时");
    }

    /// H2: continue_turn 重置 run 起点（每轮新 deadline）。
    #[test]
    fn test_continue_turn_resets_deadline() {
        let mut ctx = ContextManager::new(
            "g1".into(),
            Budget {
                max_time_secs: Some(0),
                ..Budget::default()
            },
        );
        assert!(ctx.deadline_exceeded(), "cap=0 首轮即超");
        ctx.continue_turn(
            "g2".into(),
            Budget {
                max_time_secs: Some(3600),
                ..Budget::default()
            },
        );
        assert!(!ctx.deadline_exceeded(), "新轮新起点+新 cap 不超时");
    }

    /// H3 (v0.2.4): 压缩归档——折叠的原始轮次必须落盘可检索（不再永久丢失）。
    /// 负面：归档文件不存在/内容缺失 = 压缩即失忆（修复前的行为）。
    /// env（HEARTH_ARCHIVE_FILE）是进程全局——用 ENV_LOCK 串行化（session_store 同法）。
    #[test]
    fn test_compact_archives_original_turns() {
        let _env_ser = ENV_SER.lock().unwrap_or_else(|e| e.into_inner());
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = std::env::temp_dir().join(format!("hearth_arch_{}", uuid::Uuid::new_v4()));
        let archive = dir.join("compacted.jsonl");
        std::env::set_var("HEARTH_ARCHIVE_FILE", &archive);
        let mut ctx = ContextManager::new("archive test".into(), Budget::default());
        for i in 0..5 {
            let mut turn = Turn::new(i);
            turn.messages.push(Message::new(
                format!("u{i}"),
                Role::User,
                MessageContent::Text(format!("UniqueNeedle{i}-宝藏内容必须归档")),
            ));
            turn.messages.push(Message::new(
                format!("a{i}"),
                Role::Assistant,
                MessageContent::ToolCalls(vec![agent_types::ToolCall {
                    call_id: format!("c{i}"),
                    name: "write_file".into(),
                    args: serde_json::json!({
                        "path": "f.rs",
                        "content": "x".repeat(9000)
                    }),
                }]),
            ));
            ctx.record_turn(turn);
        }
        assert!(ctx.maybe_compact(), "超阈值须触发压缩");
        let content = std::fs::read_to_string(&archive).expect("归档文件必须存在");
        assert!(
            content.contains("UniqueNeedle0"),
            "被折叠轮次的原文必须出现在归档（压缩≠丢失）"
        );
        assert!(content.contains("UniqueNeedle2"), "多个被折叠轮次都要归档");
        // 摘要头部带归档检索提示（HEARTH_ARCHIVE_FILE 覆盖时提示即该文件路径）
        let all: String = ctx
            .state()
            .history
            .iter()
            .flat_map(|t| &t.messages)
            .map(|m| format!("{:?}", m.content))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            all.contains("检索本会话归档") && all.contains(".jsonl"),
            "摘要须提示归档检索手段"
        );
        std::env::remove_var("HEARTH_ARCHIVE_FILE");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// R5-6 判据：archive 读回——digest 把已归档轮次解析回结构化清单。
    /// 旧语义：读侧为 0（写完即"换个地方销毁"）→ digest 不存在 → 红。
    /// 含去重判据：同 turn 重复归档（切片边界前移重写）清单不膨胀。
    #[test]
    fn test_r56_archive_digest_reads_back_and_dedups() {
        let _env_ser = ENV_SER.lock().unwrap_or_else(|e| e.into_inner());
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = std::env::temp_dir().join(format!("hearth_arch_r56_{}", uuid::Uuid::new_v4()));
        let archive = dir.join("compacted.jsonl");
        std::env::set_var("HEARTH_ARCHIVE_FILE", &archive);
        let mk_turn = |idx: u64, text: &str| {
            let mut turn = Turn::new(idx);
            turn.messages.push(Message::new(
                format!("u{idx}"),
                Role::User,
                MessageContent::Text(text.to_string()),
            ));
            turn
        };
        // 首次归档 turn 100/101
        archive_compacted_turns(
            "r56sid",
            &[
                mk_turn(100, "早期事实A：API key 配在 .env"),
                mk_turn(101, "早期事实B：用户偏好 tab 缩进"),
            ],
        )
        .unwrap();
        // 重复归档同索引（模拟切片边界前移重写）+ 新 turn 102
        archive_compacted_turns(
            "r56sid",
            &[
                mk_turn(100, "早期事实A：API key 配在 .env"),
                mk_turn(102, "早期事实C：部署走 .133"),
            ],
        )
        .unwrap();
        let digest = archive_digest("r56sid", 8).expect("有归档必有 digest");
        assert!(
            digest.contains("turn#100"),
            "digest 必须含 turn#100: {digest:?}"
        );
        assert!(
            digest.contains("早期事实A"),
            "digest 必须带首条用户消息摘要"
        );
        assert!(digest.contains("早期事实C"));
        let count_100 = digest.lines().filter(|l| l.starts_with("turn#100")).count();
        assert_eq!(count_100, 1, "同 turn 重复归档必须去重（清单不膨胀）");
        // 无归档 → None（best-effort 降级语义）——指向确定不存在的文件
        std::env::set_var("HEARTH_ARCHIVE_FILE", dir.join("no-such-archive.jsonl"));
        assert!(
            archive_digest("r56sid", 8).is_none(),
            "归档文件不存在时 digest 必须返回 None（best-effort）"
        );
        std::env::remove_var("HEARTH_ARCHIVE_FILE");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// G3-03 (v0.2.5): 归档按会话隔离——archive/<sid>.jsonl 每会话独立，
    /// 负面：两个会话的归档内容不得混入同一文件（修复前共享 compacted.jsonl
    /// 检索必串扰）。含文件名安全（非法字符过滤）。
    #[test]
    fn test_archive_per_session_isolation() {
        let _env_ser = ENV_SER.lock().unwrap_or_else(|e| e.into_inner());
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = std::env::temp_dir().join(format!("hearth_arch_iso_{}", uuid::Uuid::new_v4()));
        // 走 HOME 分支（临时 HOME=dir），确保 archive_path 解析可预测
        // var_os 本就返回 OsString（clippy useless_conversion 教训）
        let old_home = std::env::var_os("HOME");
        std::env::set_var("HOME", &dir);

        // 会话 A 与会话 B 各自压缩
        let mut ctx_a = ContextManager::new("goal A".into(), Budget::default());
        ctx_a.set_session_id("session-AAA");
        let mut ctx_b = ContextManager::new("goal B".into(), Budget::default());
        ctx_b.set_session_id("session-BBB");
        for (ctx, needle) in [(&mut ctx_a, "NeedleAAA"), (&mut ctx_b, "NeedleBBB")] {
            for i in 0..5 {
                let mut turn = Turn::new(i);
                turn.messages.push(Message::new(
                    format!("u{i}"),
                    Role::User,
                    MessageContent::Text(format!("{needle}-内容{i}")),
                ));
                turn.messages.push(Message::new(
                    format!("a{i}"),
                    Role::Assistant,
                    MessageContent::ToolCalls(vec![agent_types::ToolCall {
                        call_id: format!("c{i}"),
                        name: "write_file".into(),
                        args: serde_json::json!({"path": "f.rs", "content": "x".repeat(9000)}),
                    }]),
                ));
                ctx.record_turn(turn);
            }
            assert!(ctx.maybe_compact());
        }

        let file_a = dir.join(".config/hearth/archive/session-AAA.jsonl");
        let file_b = dir.join(".config/hearth/archive/session-BBB.jsonl");
        let a = std::fs::read_to_string(&file_a).expect("会话 A 归档必须独立存在");
        let b = std::fs::read_to_string(&file_b).expect("会话 B 归档必须独立存在");
        assert!(
            a.contains("NeedleAAA") && !a.contains("NeedleBBB"),
            "A 归档不得含 B 内容"
        );
        assert!(
            b.contains("NeedleBBB") && !b.contains("NeedleAAA"),
            "B 归档不得含 A 内容"
        );

        // 文件名安全：路径分隔符等非法字符 → '_'（防逃逸出 archive/ 目录）
        let mut ctx_bad = ContextManager::new("goal bad".into(), Budget::default());
        ctx_bad.set_session_id("../evil");
        for i in 0..5 {
            let mut turn = Turn::new(i);
            turn.messages.push(Message::new(
                format!("u{i}"),
                Role::User,
                MessageContent::Text(format!("SafeNeedle{i}-{}", "y".repeat(9000))),
            ));
            ctx_bad.record_turn(turn);
        }
        assert!(
            ctx_bad.maybe_compact(),
            "ctx_bad 夹具须超阈值触发压缩（9000 字符填充）"
        );
        // 过滤规则：非 [alnum/-/_]（含 '.' 和 '/'）→ '_'，故 "../evil"（2 点+1 斜杠）→ "___evil.jsonl"
        let safe_file = dir.join(".config/hearth/archive/___evil.jsonl");
        assert!(
            safe_file.exists(),
            "非法 session_id 字符必须被过滤为 '_'（不得逃逸 archive/）"
        );

        // 恢复 HOME
        match old_home {
            Some(h) => std::env::set_var("HOME", h),
            None => std::env::remove_var("HOME"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 回归 R8 (v0.1.6): 长中文目标（>60 字节，含多字节字符）触发 compaction
    /// 不得 panic——`&goal[..60]` 字节切片在中文中间切即崩（真机整进程死）。
    /// 修复后按字符截取，语义为"60 字符"。
    #[test]
    fn test_compact_long_chinese_goal_no_panic() {
        let _env_ser = ENV_SER.lock().unwrap_or_else(|e| e.into_inner());
        let mut ctx = ContextManager::new("长目标".into(), Budget::default());
        // 中文目标：>60 字节且第 60 字节落在多字节字符中间（真机 panic 场景）
        let long_goal = "帮我写一个完整的、带注释的、可运行的 Rust 实现快速排序并附带单元测试的程序，要求支持泛型和任意比较器";
        for i in 0..4 {
            let mut turn = Turn::new(i);
            turn.messages.push(Message::new(
                format!("u{i}"),
                agent_types::Role::User,
                agent_types::MessageContent::Text(long_goal.to_string()),
            ));
            turn.messages.push(Message::new(
                format!("a{i}"),
                agent_types::Role::Assistant,
                agent_types::MessageContent::ToolCalls(vec![agent_types::ToolCall {
                    call_id: format!("c{i}"),
                    name: "write_file".into(),
                    args: serde_json::json!({"path": "lib.rs", "content": "x".repeat(9000)}),
                }]),
            ));
            ctx.record_turn(turn);
        }
        let compacted = ctx.maybe_compact();
        assert!(compacted, "超阈值必须触发压缩");
        let all: String = ctx
            .state()
            .history
            .iter()
            .flat_map(|t| &t.messages)
            .map(|m| format!("{:?}", m.content))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(all.contains("快速排序"), "摘要须保留长中文目标（不 panic）");
    }

    // ── P2-MEMORY-CONTEXT-01 Node 03 / §21-S7: INV-M01 确定性 fixture ──
    // 「Fact/Verification 不随普通 compaction 丢失」必须可失败（断言不能失败=断言不存在）。
    // 存储位置分类（S-3 前置）：goal/original_goal/constraints/acceptance_criteria/
    // scratch = **state 层，不在压缩路径**；对话原文 = history（会被折叠）；
    // 原文全量 = archive（B 档恢复通道）。
    fn inv_m01_build_ctx(dir: &std::path::Path) -> ContextManager {
        let archive = dir.join("inv_m01").join("archived.jsonl");
        std::env::set_var("HEARTH_ARCHIVE_FILE", &archive);
        let mut ctx = ContextManager::new(
            "修复 mathlib 并通过验收".into(),
            Budget {
                max_steps: 50,
                max_tokens: None,
                max_time_secs: None,
                ..Budget::default()
            },
        );
        ctx.set_session_id("inv-m01-test");
        // state 层事实（压缩路径之外）
        ctx.state_mut().original_goal = Some("修复 mathlib 并通过验收".into());
        ctx.state_mut().goal_revision = 2;
        ctx.state_mut().constraints.push("不要做额外验证".into());
        ctx.state_mut()
            .acceptance_criteria
            .push("cmd: cargo test --manifest-path mathlib/Cargo.toml".into());
        ctx.set_scratch(
            "acceptance_result",
            serde_json::json!({"status": "passed", "failures": []}),
        );
        ctx.set_scratch("last_failure_class", serde_json::json!("AssertionFailure"));

        // history 层：6 个旧轮（每轮含 artifact 事实 FACT-i、verification 结果、用户决策），
        // 大填充文本确保超 32k est 口径触发压缩
        for i in 0..6u64 {
            let mut turn = Turn::new(i);
            turn.messages.push(Message::new(
                format!("u{i}"),
                agent_types::Role::User,
                agent_types::MessageContent::Text(format!(
                    "第{i}轮指令：把 FACT-{i}=value-{i} 写入 abc{i}.txt，然后运行测试。{}",
                    "x".repeat(3500)
                )),
            ));
            turn.messages.push(Message::new(
                format!("a{i}"),
                agent_types::Role::Assistant,
                agent_types::MessageContent::ToolCalls(vec![agent_types::ToolCall {
                    call_id: format!("c{i}"),
                    name: "write_file".into(),
                    args: serde_json::json!({
                        "path": format!("abc{i}.txt"),
                        "content": format!("FACT-{i}=value-{i}")
                    }),
                }]),
            ));
            turn.messages.push(Message::new(
                format!("r{i}"),
                agent_types::Role::Tool,
                agent_types::MessageContent::ToolResults(vec![agent_types::ToolResult {
                    call_id: format!("c{i}"),
                    is_error: false,
                    output: format!(
                        "wrote abc{i}.txt; test result: ok. FACT-{i}=value-{i} verified"
                    ),
                    artifacts: vec![],
                    error_kind: None,
                }]),
            ));
            turn.messages.push(Message::new(
                format!("d{i}"),
                agent_types::Role::User,
                agent_types::MessageContent::Text(format!(
                    "用户决策{i}：测试通过 FACT-{i}，继续下一项。{}",
                    "y".repeat(3500)
                )),
            ));
            ctx.record_turn(turn);
        }
        ctx
    }

    /// INV-M01 绿线：state 层事实全 preserved + archive 原文可恢复（B 档）+
    /// 摘要重构（goal/工具/写盘）+ 检索提示在场。
    #[test]
    fn test_inv_m01_fact_survives_compaction() {
        let _env_ser = ENV_SER.lock().unwrap_or_else(|e| e.into_inner());
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let old_archive = std::env::var("HEARTH_ARCHIVE_FILE").ok();
        let mut ctx = inv_m01_build_ctx(dir.path());

        // ── BEFORE 快照（12 项检查清单 + 存储位置标注）──
        let before = [
            ("goal", "state", ctx.state().goal.clone()),
            (
                "original_goal",
                "state",
                ctx.state().original_goal.clone().unwrap(),
            ),
            (
                "goal_revision",
                "state",
                ctx.state().goal_revision.to_string(),
            ),
            ("constraints", "state", ctx.state().constraints.join("|")),
            (
                "acceptance_criteria",
                "state",
                ctx.state().acceptance_criteria.join("|"),
            ),
            (
                "acceptance_result",
                "scratch",
                ctx.get_scratch("acceptance_result").unwrap().to_string(),
            ),
            (
                "failure_class",
                "scratch",
                ctx.get_scratch("last_failure_class").unwrap().to_string(),
            ),
            ("fact_0", "history→archive", "FACT-0=value-0".into()),
            ("fact_5", "history→archive", "FACT-5=value-5".into()),
            (
                "verification_0",
                "history→archive",
                "test result: ok. FACT-0=value-0 verified".into(),
            ),
            (
                "user_decision_2",
                "history→archive",
                "用户决策2：测试通过 FACT-2".into(),
            ),
            ("artifact_3", "history→archive(摘要)", "abc3.txt".into()),
        ];
        assert!(
            ctx.estimate_chars() >= ContextManager::COMPACT_CHAR_THRESHOLD,
            "前置：必须达到压缩阈值"
        );

        // ── 触发压缩 ──
        assert!(ctx.maybe_compact(), "超阈值必须触发");

        // ── AFTER 逐项判定 ──
        // state/scratch 层（不在压缩路径）→ preserved（逐项相等）
        assert_eq!(ctx.state().goal, before[0].2, "goal preserved");
        assert_eq!(
            ctx.state().original_goal.as_deref(),
            Some("修复 mathlib 并通过验收"),
            "original_goal preserved（immutable 锚）"
        );
        assert_eq!(ctx.state().goal_revision, 2, "goal_revision preserved");
        assert_eq!(ctx.state().constraints.len(), 1, "constraints preserved");
        assert_eq!(
            ctx.state().acceptance_criteria.len(),
            1,
            "acceptance_criteria preserved"
        );
        assert_eq!(
            ctx.get_scratch("acceptance_result").unwrap(),
            &serde_json::json!({"status": "passed", "failures": []}),
            "acceptance_result (Verification) preserved"
        );
        assert_eq!(
            ctx.get_scratch("last_failure_class").unwrap(),
            &serde_json::json!("AssertionFailure"),
            "failure_class (Fact) preserved"
        );

        // archive（B 档）：旧轮（前 4 轮，COMPACT_KEEP_TURNS=2 保留后 2 轮）原文落盘
        let archive = std::env::var("HEARTH_ARCHIVE_FILE").unwrap();
        let archived = std::fs::read_to_string(&archive).expect("archive 必须真实落盘（S-2）");
        for key in [&before[7], &before[9], &before[10]] {
            assert!(
                archived.contains(key.2.as_str()),
                "B 档可恢复：{} 原文必须在 archive 中",
                key.0
            );
        }
        // P3 RC45 分账后 Act/Done verify 重试次数独立 → 交换数可能多于最初
        // fixture（归档行数随 verify 轮次增加）；断言改下界（≥4 行归档），
        // 内容正确性由上方 B 档 contains 断言保证。
        assert!(
            archived.lines().count() >= 4,
            "旧轮全量归档至少 4 行（最近 2 轮保留）"
        );
        // 旧轮事实已离开 history（压缩生效），但 archive 可恢复——INV-M01 的核心
        let hist_now: String = ctx
            .state()
            .history
            .iter()
            .flat_map(|t| &t.messages)
            .map(|m| match &m.content {
                agent_types::MessageContent::Text(s) => s.clone(),
                agent_types::MessageContent::ToolResults(rs) => rs
                    .iter()
                    .map(|r| r.output.clone())
                    .collect::<Vec<_>>()
                    .join("\n"),
                _ => String::new(),
            })
            .collect();
        assert!(
            !hist_now.contains("test result: ok. FACT-0=value-0 verified"),
            "旧轮 tool_result（摘要不承载）离开 context（压缩生效）"
        );

        // summary（reconstructed）：goal/工具/写盘 + 检索提示
        let hist_text: String = ctx
            .state()
            .history
            .iter()
            .flat_map(|t| &t.messages)
            .map(|m| format!("{:?}", m.content))
            .collect();
        assert!(hist_text.contains("[compacted 会话摘要]"), "摘要存在");
        assert!(
            hist_text.contains("第0轮指令"),
            "摘要重构轮次目标（goal_short）"
        );
        assert!(hist_text.contains("write_file"), "摘要重构工具清单");
        assert!(hist_text.contains("abc3.txt"), "摘要重构写盘文件");
        assert!(hist_text.contains("检索本会话归档"), "归档检索提示注入");
        assert!(
            hist_text.contains("FACT-5=value-5"),
            "最近 2 轮原文保留在 history（A 档上下文内保持）"
        );

        // 恢复 env
        match old_archive {
            Some(v) => std::env::set_var("HEARTH_ARCHIVE_FILE", v),
            None => std::env::remove_var("HEARTH_ARCHIVE_FILE"),
        }
    }

    /// INV-M01 损失模式钉死（fixture 能失败的确定性证明）：archive 写入失败时
    /// （best-effort 仅 warn），B 档事实（原文）在 history 中**确实不存在**——
    /// 即"没有 archive 就没有恢复通道"，任何"压缩后仍能答对原文细节"的自述
    /// 由此 fixture 证伪。
    #[test]
    fn test_inv_m01_loss_mode_without_archive() {
        let _env_ser = ENV_SER.lock().unwrap_or_else(|e| e.into_inner());
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        // HEARTH_ARCHIVE_FILE 指向一个**目录**→ open 必败 → archive best-effort 失败
        let bogus = dir.path().join("bogus_dir.jsonl");
        std::fs::create_dir_all(&bogus).unwrap();
        std::env::set_var("HEARTH_ARCHIVE_FILE", &bogus);
        let old_archive = std::env::var("HEARTH_ARCHIVE_FILE").ok();
        // 注意：inv_m01_build_ctx 会重设 env——这里手动构建同构 ctx
        let mut ctx = ContextManager::new(
            "修复 mathlib 并通过验收".into(),
            Budget {
                max_steps: 50,
                max_tokens: None,
                max_time_secs: None,
                ..Budget::default()
            },
        );
        ctx.set_session_id("inv-m01-loss");
        for i in 0..6u64 {
            let mut turn = Turn::new(i);
            turn.messages.push(Message::new(
                format!("u{i}"),
                agent_types::Role::User,
                agent_types::MessageContent::Text(format!(
                    "第{i}轮指令：把 FACT-{i}=value-{i} 写入 abc{i}.txt。{}",
                    "x".repeat(7000)
                )),
            ));
            turn.messages.push(Message::new(
                format!("r{i}"),
                agent_types::Role::Tool,
                agent_types::MessageContent::ToolResults(vec![agent_types::ToolResult {
                    call_id: format!("c{i}"),
                    is_error: false,
                    output: format!("test result: ok. FACT-{i}=value-{i} verified"),
                    artifacts: vec![],
                    error_kind: None,
                }]),
            ));
            ctx.record_turn(turn);
        }
        assert!(ctx.maybe_compact(), "压缩照常触发（archive 失败不阻塞）");

        // 损失模式：archive 文件不存在（open 目录失败），原文在 history 中消失
        assert!(
            !bogus.is_file(),
            "archive 路径是目录，不可能成为文件（best-effort 失败）"
        );
        let hist_text: String = ctx
            .state()
            .history
            .iter()
            .flat_map(|t| &t.messages)
            .map(|m| match &m.content {
                agent_types::MessageContent::Text(s) => s.clone(),
                agent_types::MessageContent::ToolResults(rs) => rs
                    .iter()
                    .map(|r| r.output.clone())
                    .collect::<Vec<_>>()
                    .join("\n"),
                _ => String::new(),
            })
            .collect();
        assert!(
            !hist_text.contains("test result: ok. FACT-0=value-0 verified"),
            "损失模式坐实：无 archive 时 tool_result 原文不在 history（B 档无通道）"
        );
        // state 层事实仍 preserved（INV-M01 的"不可丢"边界只在 state/archive 层成立）
        assert_eq!(ctx.state().goal, "修复 mathlib 并通过验收");

        match old_archive {
            Some(v) => std::env::set_var("HEARTH_ARCHIVE_FILE", v),
            None => std::env::remove_var("HEARTH_ARCHIVE_FILE"),
        }
    }

    /// P2 S-9: env 覆盖阈值——设置小阈值时短历史也触发（受控触发手段可用性）；
    /// 未设置时回落常量（生产默认零变化）。
    #[test]
    fn test_compact_threshold_env_override() {
        let _env_ser = ENV_SER.lock().unwrap_or_else(|e| e.into_inner());
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let old = std::env::var("HEARTH_COMPACT_CHAR_THRESHOLD").ok();
        let old_mode = std::env::var("HEARTH_COMPACTION_MODE").ok();
        std::env::remove_var("HEARTH_COMPACTION_MODE");
        std::env::set_var("HEARTH_COMPACT_CHAR_THRESHOLD", "100");
        let ctx = ContextManager::new("t".into(), Budget::default());
        assert_eq!(ctx.compact_char_threshold(), 100);
        std::env::remove_var("HEARTH_COMPACT_CHAR_THRESHOLD");
        assert_eq!(
            ctx.compact_char_threshold(),
            ContextManager::COMPACT_CHAR_THRESHOLD
        );
        // P2-CFR 修正后优先级：env（测试仪器）> injected（provider-aware）> 常量
        let mut ctx2 = ContextManager::new("t".into(), Budget::default());
        ctx2.set_compact_threshold(123_456);
        assert_eq!(ctx2.compact_char_threshold(), 123_456, "无 env 时注入生效");
        std::env::set_var("HEARTH_COMPACT_CHAR_THRESHOLD", "777");
        assert_eq!(
            ctx2.compact_char_threshold(),
            777,
            "env 测试仪器必须压过注入值（COMPACT_DBG 真机教训）"
        );
        std::env::remove_var("HEARTH_COMPACT_CHAR_THRESHOLD");
        // 回滚通道：legacy 模式整体回落遗留常量
        std::env::set_var("HEARTH_COMPACTION_MODE", "legacy");
        assert_eq!(
            ctx2.compact_char_threshold(),
            ContextManager::COMPACT_CHAR_THRESHOLD
        );
        std::env::remove_var("HEARTH_COMPACTION_MODE");
        match (old.clone(), old_mode) {
            (_, Some(m)) => std::env::set_var("HEARTH_COMPACTION_MODE", m),
            (_, None) => std::env::remove_var("HEARTH_COMPACTION_MODE"),
        }
        match old {
            Some(v) => std::env::set_var("HEARTH_COMPACT_CHAR_THRESHOLD", v),
            None => std::env::remove_var("HEARTH_COMPACT_CHAR_THRESHOLD"),
        }
    }

    /// P2 Node 12（批-3 仪器修正红→绿）：estimate_chars 必须按真实字符计数——
    /// 修复前 Debug+bytes 对中文虚增 ~3×（Node 01 实测 2.49×）。
    #[test]
    fn test_estimate_chars_honest_counting() {
        let ctx = ContextManager::new("t".into(), Budget::default());
        let mut t = Turn::new(0);
        let zh = "中文字符串测试"; // 7 chars, 21 bytes UTF-8
        t.messages.push(Message::new(
            "m1".into(),
            agent_types::Role::User,
            agent_types::MessageContent::Text(zh.into()),
        ));
        let mut ctx = ctx;
        ctx.record_turn(t);
        // naive 口径 = 7（修正后）；旧 Debug+bytes 口径 = 21+2引号 = 23
        assert_eq!(ctx.estimate_chars(), 7, "estimate 必须按真实字符计数");
        assert_eq!(
            ctx.legacy_estimate_chars(),
            format!("{:?}", agent_types::MessageContent::Text(zh.into())).len(),
            "legacy 口径保留旧行为（Debug 枚举 + 字节，回滚通道）"
        );
    }

    /// P2 Node 12（Node 02 重大发现的红测试）：单 run（单一 Turn）下即使 est
    /// 大幅超阈值，maybe_compact 也恒 false——压缩死代码。修复 = turn 粒度对齐
    /// （record_tool_exchange 每次交换新 Turn）。修复前本测试必须红。
    #[test]
    fn test_compact_fires_in_single_run() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let old_archive = std::env::var("HEARTH_ARCHIVE_FILE").ok();
        std::env::set_var("HEARTH_ARCHIVE_FILE", dir.path().join("a.jsonl"));
        let mut ctx = ContextManager::new("g".into(), Budget::default());
        ctx.set_session_id("single-run");
        // 模拟修复后 record_tool_exchange：一次工具交换 = 一个新 Turn
        for i in 0..6u64 {
            let mut t = Turn::new(i);
            t.messages.push(Message::new(
                format!("a{i}"),
                agent_types::Role::Assistant,
                agent_types::MessageContent::ToolCalls(vec![agent_types::ToolCall {
                    call_id: format!("c{i}"),
                    name: "bash".into(),
                    args: serde_json::json!({"cmd": "seq 1 1500"}),
                }]),
            ));
            t.messages.push(Message::new(
                format!("r{i}"),
                agent_types::Role::Tool,
                agent_types::MessageContent::Text("z".repeat(5500)),
            ));
            ctx.record_turn(t);
        }
        assert!(
            ctx.state().history.len() >= 3,
            "前置：多 Turn（修复后粒度）"
        );
        assert!(
            ctx.estimate_chars() >= ContextManager::COMPACT_CHAR_THRESHOLD,
            "前置：est 口径已超阈值"
        );
        assert!(
            ctx.maybe_compact(),
            "单 run 超阈值必须压缩（修复前：恒 false = 死代码）"
        );
        match old_archive {
            Some(v) => std::env::set_var("HEARTH_ARCHIVE_FILE", v),
            None => std::env::remove_var("HEARTH_ARCHIVE_FILE"),
        }
    }
}
