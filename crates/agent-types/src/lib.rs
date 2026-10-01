use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use utoipa::ToSchema;

// ── 原 P3 TaskGraph 家族的残留（D-76 已删除其余） ──

/// 任务节点状态。
///
/// D-76（2026-10-01, traecode）：本文件的 `TaskGraph` 家族（`TaskGraph` /
/// `TaskNode` / `TaskResult` / `PlanContext` 及其 `impl` 与单测）**已删除**——
/// TaskGraph 子系统随线C手术拆除（见 `agent-core/src/loop.rs` 的相关注释、
/// P1-32 已删 CLI 残壳），全仓**零生产消费者**。本枚举保留，因为
/// `SessionLedger::sync_from_task_graph` 的入参形态仍为 `(TaskStatus, String)`
/// （该方法自身的生产接线归属债务 D-79 的裁决范围）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Skipped,
}

// D-76（2026-10-01, traecode）：原 `Observation`（运行观测快照）与 `PlanState`
// （图执行状态）两类型**已删除**——二者均以已拆除的 TaskGraph 为核心字段，且全仓
// **零生产消费者**（仅类型定义自身）。`FileChange` **已随 D-83 删除**——其唯一载体
// `RunReport.files_changed` 与合并逻辑（`merge_file_changes` /
// `extract_files_from_tool_calls`）均零生产调用方。

/// WP-3 (v23 phase3): 规划缺口——plan 产出后的可审查缺口记录。
/// 每条缺口必须带 `from`（来源）+ `why`（原因）；非阻塞缺口必须 `auto_assumed=true`
/// （已自动假设，不打断执行）。blocking=true 的缺口走通用交互澄清
/// （InteractionRequested kind="clarification"——内核不加分支）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Gap {
    /// 缺口来源（如 "missing_goal_source" / "ambiguous_option"）。
    pub from: String,
    /// 缺口原因说明（给人看）。
    pub why: String,
    /// true=阻塞执行，必须澄清才能继续；false=已自动假设，不打断。
    pub blocking: bool,
    /// blocking=false 时 100% 为 true（审计：非阻塞缺口必须显式声明"已假设"）。
    pub auto_assumed: bool,
    /// R10 (v0.1.6): 澄清选项（选项式交互用）。空 = 自由文本。
    /// serde(default) 保证旧序列化数据（无此字段）反序列化不炸。
    #[serde(default)]
    pub options: Vec<String>,
    /// R10 (v0.1.6): 交互样式 "free_text" | "single_select" | "confirm"。默认 free_text。
    #[serde(default = "default_gap_style")]
    pub style: String,
}

fn default_gap_style() -> String {
    "free_text".into()
}

impl Gap {
    /// 构造非阻塞缺口——强制 auto_assumed=true（门禁：blocking=false ⇒ assume）。
    pub fn assumed(from: impl Into<String>, why: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            why: why.into(),
            blocking: false,
            auto_assumed: true,
            options: vec![],
            style: "free_text".into(),
        }
    }

    /// 构造阻塞缺口——必须澄清。
    pub fn blocking(from: impl Into<String>, why: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            why: why.into(),
            blocking: true,
            auto_assumed: false,
            options: vec![],
            style: "free_text".into(),
        }
    }

    /// R10 (v0.1.6): 构造阻塞缺口 + 选项（single_select 样式）——含糊目标选项式澄清。
    pub fn blocking_with_options(
        from: impl Into<String>,
        why: impl Into<String>,
        options: Vec<String>,
    ) -> Self {
        Self {
            from: from.into(),
            why: why.into(),
            blocking: true,
            auto_assumed: false,
            options,
            style: "single_select".into(),
        }
    }
}

// ── Original types ──

/// Message role in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// Unique message identifier.
pub type MessageId = String;

/// Message metadata (token count, source, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MessageMeta {
    pub token_count: Option<u32>,
    pub source: Option<String>,
}

/// Content of a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageContent {
    Text(String),
    ToolCalls(Vec<ToolCall>),
    ToolResults(Vec<ToolResult>),
}

/// A single message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub role: Role,
    pub content: MessageContent,
    pub created_at: DateTime<Utc>,
    pub meta: MessageMeta,
    /// v24-post: deepseek thinking mode 的 reasoning_content（assistant 文本消息），
    /// 必须回传——否则 API 400。None=无思考内容。
    pub reasoning_content: Option<String>,
}

/// A tool call requested by the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub call_id: String,
    pub name: String,
    pub args: Value,
}

/// An artifact produced by a tool (e.g. file diff).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub name: String,
    pub content_type: String,
    pub data: Value,
}

/// P1-FAILURE-ADAPTATION-01 Node 06: 工具错误的结构化类别——消费方据此做
/// 确定性 failure 分类（RC20 纪律：禁止解析错误文本）。增量字段向后兼容
/// （serde default + 跳过 None 序列化，不改事件契约）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolErrorKind {
    /// 任务 deadline 截断（tool-runtime TaskDeadlineExceeded 的投影）
    DeadlineExceeded,
    /// P2-LR Node 02（RC46 接线，批-4）：bash 进程退出码 > 0（断言/测试失败族）
    ExitNonZero(i32),
    /// 退出码 < 0（sandbox 报告的信号终止——seccomp KILL 等）
    ExitSignal(i32),
    /// 工具自身超时（sandbox 层 timed_out，非任务 deadline）
    ToolTimeout,
}

/// Result of executing a tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub call_id: String,
    pub is_error: bool,
    pub output: String,
    pub artifacts: Vec<Artifact>,
    /// 结构化错误类别（None = 未分类；不进 LLM 可见内容）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_kind: Option<ToolErrorKind>,
}

/// A single turn in the agent loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Turn {
    pub index: u64,
    pub messages: Vec<Message>,
    pub actions: Vec<Action>,
}

/// An action performed during a turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub name: String,
    pub outcome: String,
    pub elapsed_ms: u64,
}

/// Budget constraints for a run.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Budget {
    pub max_steps: u64,
    pub max_tokens: Option<u64>,
    pub max_time_secs: Option<u64>,
    /// WS9 (v0.2): 预算档位 "premium"|"standard"|"economy"（默认 standard）。
    /// 按目标方向倾斜分配（规划阶段打标）；非全局硬杀数——跨偏离阈值走预警+ask。
    #[serde(default = "default_budget_tier")]
    pub tier: String,
    /// WS9 (v0.2): 本档分配单元（相对单位，上策100/中策50/下策20）。
    #[serde(default)]
    pub allocated_units: Option<u64>,
    /// WS9 (v0.2): 偏离预警阈值（占 max_steps 比例）。跨阈值 → BudgetDeviation
    /// 预警（非阻塞）+ 跨 1.0 → blocking ask（继续/重新评估），非静默终止。
    #[serde(default = "default_deviation_warn_at")]
    pub deviation_warn_at: Vec<f64>,
}

fn default_budget_tier() -> String {
    "standard".into()
}

fn default_deviation_warn_at() -> Vec<f64> {
    vec![0.5, 1.0]
}

impl Default for Budget {
    fn default() -> Self {
        Self {
            // R6-6（判定权归还长程任务书 v1.0）：预算可配置——默认 50 改配置项
            // （HEARTH_MAX_STEPS env；CLI config 层后续接线同一语义）。>0 校验，
            // 非法值回落 50（护栏不得被配没）。
            max_steps: std::env::var("HEARTH_MAX_STEPS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .filter(|&v| v > 0)
                .unwrap_or(50),
            max_tokens: None,
            max_time_secs: None,
            tier: "standard".into(),
            allocated_units: None,
            deviation_warn_at: vec![0.5, 1.0],
        }
    }
}

// D-76（2026-10-01, traecode）：原 `impl TaskGraph`（`is_acyclic` / `topo_order` /
// `next_ready` / `count_by_status` / `next_action_deterministic` / `completed_titles`
// / `remaining_titles`）随 `TaskGraph` 类型一并删除——全仓零生产消费者。

/// ── R2-1 SessionLedger（对话可用性根治任务书 v1.0，2026-09-04）──
///
/// ⚠️ **D-79（2026-10-01, traecode）取证结论：本账本在生产路径"只读不写"。**
/// 写入侧只有 [`SessionLedger::add`] / [`SessionLedger::sync_from_task_graph`]，
/// 而全仓生产代码**从不调用**它们（`add` 仅出现在单测；`sync_from_task_graph`
/// 的调用方仅 `agent-core` 的单测）——其原生产者为已拆除的 TaskGraph。
/// 读取侧则是活的且**用户可见**：`agent-core/src/loop.rs:2554` 的 prompt 注入
/// （`render_for_prompt`）、`ledger_pending_texts()` 进 report、CLI
/// `run_local.rs` 的 `ledger_pending` 字段。⇒ 当前**账本恒空**，
/// 注入块恒为 None（即"零遗忘"机制实际不生效）。
/// 处置归属 **D-79（待裁：重新设计生产者接线 / 整体退役）**，不在本卡夹带。
///
/// 会话状态账本：五栏（未完成/已知失败/承诺/已验证事实/待验证）。
/// 三条硬约束（顶层批复《R1包与R2两件顶层验收批复》§四）：
///   ① 五栏齐全；
///   ② 注入时点在**切片之后**（agent-core build_messages 侧保证）——早轮
///     被裁，账本仍在 prompt 内（G-D 门"列 5 项→5 轮继续→零遗忘"的机制基础）；
///   ③ 条目**只 close() 不 remove()**——"未完成清单蒸发"（0.9-0.3 会话
///     十项全蒸发）在**数据结构层面**不可能发生：关闭是状态迁移，不是删除。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LedgerColumn {
    /// 未完成
    Pending,
    /// 已知失败（R1-4 known-failing 的数据源：未复测通过前禁止标 ✅）
    KnownFailing,
    /// 承诺（agent 对用户承诺过要做的事）
    Commitment,
    /// 已验证事实
    VerifiedFact,
    /// 待验证
    UnverifiedClaim,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub id: u64,
    pub column: LedgerColumn,
    pub text: String,
    pub opened_at_step: u64,
    /// None = 仍开放；Some(step) = 关闭时点。**关闭 ≠ 删除**——条目永久留档。
    pub closed_at_step: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionLedger {
    /// 只追加；关闭 = 状态迁移。**无任何 remove 通道**（结构级防蒸发）。
    entries: Vec<LedgerEntry>,
    next_id: u64,
}

impl SessionLedger {
    /// 登记新条目，返回 id。
    pub fn add(&mut self, column: LedgerColumn, text: impl Into<String>, step: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.entries.push(LedgerEntry {
            id,
            column,
            text: text.into(),
            opened_at_step: step,
            closed_at_step: None,
        });
        id
    }

    /// 关闭条目（唯一状态出口）。返回是否找到**开放**条目。
    pub fn close(&mut self, id: u64, step: u64) -> bool {
        match self
            .entries
            .iter_mut()
            .find(|e| e.id == id && e.closed_at_step.is_none())
        {
            Some(e) => {
                e.closed_at_step = Some(step);
                true
            }
            None => false,
        }
    }

    /// 按文本前缀关闭（agent 语义关闭入口："这项做完了"）。返回关闭的 id。
    pub fn close_by_prefix(
        &mut self,
        column: LedgerColumn,
        prefix: &str,
        step: u64,
    ) -> Option<u64> {
        let e = self.entries.iter_mut().find(|e| {
            e.column == column
                && e.closed_at_step.is_none()
                && e.text.to_lowercase().starts_with(&prefix.to_lowercase())
        })?;
        e.closed_at_step = Some(step);
        Some(e.id)
    }

    pub fn open_in(&self, column: LedgerColumn) -> Vec<&LedgerEntry> {
        self.entries
            .iter()
            .filter(|e| e.column == column && e.closed_at_step.is_none())
            .collect()
    }

    pub fn open_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.closed_at_step.is_none())
            .count()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 某栏是否存在**开放**且文本以 prefix 开头的条目（派生去重用）。
    pub fn has_open_prefix(&self, column: LedgerColumn, prefix: &str) -> bool {
        self.entries.iter().any(|e| {
            e.column == column
                && e.closed_at_step.is_none()
                && e.text.to_lowercase().starts_with(&prefix.to_lowercase())
        })
    }

    /// 渲染为注入 prompt 的文本。None = 账本为空（零注入不占上下文）。
    /// 上限：每栏最多 12 条开放条目（防账本本身吃爆上下文）；已关闭只计总数。
    pub fn render_for_prompt(&self) -> Option<String> {
        const PER_COLUMN_CAP: usize = 12;
        let cols = [
            (LedgerColumn::Pending, "未完成"),
            (
                LedgerColumn::KnownFailing,
                "已知失败（未复测通过前禁止标记为完成）",
            ),
            (LedgerColumn::Commitment, "承诺"),
            (LedgerColumn::VerifiedFact, "已验证事实"),
            (LedgerColumn::UnverifiedClaim, "待验证"),
        ];
        let closed_count = self
            .entries
            .iter()
            .filter(|e| e.closed_at_step.is_some())
            .count();
        let mut sections: Vec<String> = Vec::new();
        for (col, label) in cols {
            let open: Vec<_> = self.open_in(col);
            if open.is_empty() {
                continue;
            }
            let items: Vec<String> = open
                .iter()
                .take(PER_COLUMN_CAP)
                .map(|e| format!("  {}) {}", e.id, e.text))
                .collect();
            let more = if open.len() > PER_COLUMN_CAP {
                format!("\n  （…另有 {} 条）", open.len() - PER_COLUMN_CAP)
            } else {
                String::new()
            };
            sections.push(format!(
                "· {} ({}):\n{}{}",
                label,
                open.len(),
                items.join("\n"),
                more
            ));
        }
        if sections.is_empty() {
            if self.entries.is_empty() {
                return None;
            }
            return Some(format!(
                "[session ledger] 会话账本：无未结条目（历史已关闭 {closed_count} 条）。"
            ));
        }
        let closed_note = if closed_count > 0 {
            format!("（另有已关闭 {closed_count} 条从略——关闭≠删除，完整账本随会话留档）")
        } else {
            String::new()
        };
        Some(format!(
            "[session ledger] 会话状态账本（事实账本——条目只会被关闭，不会被删除；任何此前的未完成/失败项都在下面，禁止假装它们不存在或宣称完成未列出的核验）：\n{}\n{closed_note}",
            sections.join("\n")
        ))
    }

    /// 从 TaskGraph 派生同步（自动采集）：Failed 节点 → KnownFailing、
    /// 未完成节点 → Pending、Completed 节点 → 关闭对应开放条目。
    /// 去重 = 同栏同文本前缀已有开放条目则不重复登记。
    /// 返回本次新增条目数。
    pub fn sync_from_task_graph(&mut self, nodes: &[(TaskStatus, String)], step: u64) -> usize {
        let mut added = 0;
        for (status, desc) in nodes {
            match status {
                TaskStatus::Failed => {
                    if !self.has_open_prefix(LedgerColumn::KnownFailing, desc) {
                        self.add(LedgerColumn::KnownFailing, desc.clone(), step);
                        added += 1;
                    }
                }
                TaskStatus::Pending | TaskStatus::InProgress => {
                    if !self.has_open_prefix(LedgerColumn::Pending, desc) {
                        self.add(LedgerColumn::Pending, desc.clone(), step);
                        added += 1;
                    }
                }
                TaskStatus::Completed | TaskStatus::Skipped => {
                    // 终结（完成/跳过）→ 关闭对应开放条目（Pending 栏优先，
                    // KnownFailing 次之）。只 close 不 remove。
                    if let Some(id) = self.close_by_prefix(LedgerColumn::Pending, desc, step) {
                        let _ = id;
                    } else {
                        let _ = self.close_by_prefix(LedgerColumn::KnownFailing, desc, step);
                    }
                }
            }
        }
        added
    }
}

/// The running state of an agent execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunState {
    pub goal: String,
    pub history: Vec<Turn>,
    pub scratch: HashMap<String, Value>,
    pub budget: Budget,
    pub steps_used: u64,
    pub tokens_used: u64,
    /// R2-D (v0.2.7): 任务原始目标——**immutable**（批示 1：禁止覆盖）。
    /// 首次任务初始化时写入一次；用户换目标只改 `goal`（current_goal）并
    /// goal_revision++，original_goal 永久保留作 Goal Preservation 锚点。
    /// None = 旧会话/未初始化。
    #[serde(default)]
    pub original_goal: Option<String>,
    /// R2-D: 当前目标修订序号（每轮 current_goal 变化 ++——goal_changed 事件载荷）。
    #[serde(default)]
    pub goal_revision: u64,
    /// R2-D: 约束清单（批示 4：第一版 provenance 仅限 Hearth.md/CLI 显式/
    /// InteractionRequest 用户答复，不做自由文本推断）。
    #[serde(default)]
    pub constraints: Vec<String>,
    /// R2-D: 验收标准（规划阶段显式产出或用户显式指定；空 = 退化到
    /// artifact-level verification，acceptance_verification 恒 "none"——批示 5）。
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,
    /// R2-1 SessionLedger：会话状态账本（五栏，条目只 close 不 remove——
    /// 结构级防"未完成清单蒸发"）。注入在切片之后（build_messages）——
    /// G-D 门"零遗忘"的机制基础。随 state 序列化（会话落盘/恢复自带）。
    #[serde(default)]
    pub ledger: SessionLedger,
}

impl RunState {
    pub fn new(goal: String, budget: Budget) -> Self {
        Self {
            goal,
            history: Vec::new(),
            scratch: HashMap::new(),
            budget,
            steps_used: 0,
            tokens_used: 0,
            original_goal: None,
            goal_revision: 0,
            constraints: Vec::new(),
            acceptance_criteria: Vec::new(),
            ledger: SessionLedger::default(),
        }
    }
}

impl Message {
    pub fn new(id: MessageId, role: Role, content: MessageContent) -> Self {
        Self {
            id,
            role,
            content,
            created_at: Utc::now(),
            meta: MessageMeta::default(),
            reasoning_content: None,
        }
    }
}

impl Turn {
    pub fn new(index: u64) -> Self {
        Self {
            index,
            messages: Vec::new(),
            actions: Vec::new(),
        }
    }
}

// D-76（2026-10-01, traecode）：原 P2「提示注入防护类型」`ContentSource`、
// `MAX_INJECTED_CHARS` 与 `format_injected_content`（见下方原位置）**已删除**——
// 三者全仓**零调用方**（仅 `format_injected_content` 内部引用常量），即"来源标注 /
// 注入防护"在**提示层从未接线**。保留只会让审计者误以为该防护已生效。
// **能力缺口已登记为债务 D-80**（接线属提示契约特性：需先在注入点定好标注格式与
// 截断语义，单独立卡），不靠死代码充数。

/// P2 Node 10-13 决议（P4 Node 13 落地）：静默截断 → 显式标记。
/// 事实投影侧的截断必须让"信息被销毁"可见（压缩=信息销毁家族防线）：
/// 截断发生时追加 `\n[... N chars truncated]`（与 loop.rs 分块标记同格式）。
/// 适用边界：LLM 上下文注入 / 失败事实保全 / 诊断消息等**事实投影**；
/// 内部 query 构造与纯 UI 摘要豁免（见 docs 截断清单登记）。
/// 修复 4（手术包二 P0）：最大输出 token 数统一取参。
/// 背景：agnes-3.0-flash 的 thinking 与正文**共享输出预算**（评估报告 c16d133 实测：
/// max_tokens=2048 时正文 0 字）；8192 硬编码（loop.rs:2855/2994 等）
/// 会导致重推理轮正文被截断（真机复现："模型输出残缺"）。env `HEARTH_MAX_TOKENS`
/// 覆盖，默认 65536（3.0-flash 约束）；下限 256 防呆（过小回退默认并告警），
/// 上限 131072（防误配）。
pub fn max_output_tokens() -> u32 {
    max_output_tokens_from(std::env::var("HEARTH_MAX_TOKENS").ok().as_deref())
}

/// 纯函数（无 env 读取——测试无竞态）：解析覆盖值。
/// None/非法 → 默认 65536；<256 回退默认；>131072 收敛上限。
pub fn max_output_tokens_from(raw: Option<&str>) -> u32 {
    const DEFAULT: u32 = 65536;
    const MIN: u32 = 256;
    const MAX: u32 = 131072;
    match raw {
        None => DEFAULT,
        Some(v) => match v.trim().parse::<u32>() {
            Ok(n) if n < MIN => {
                eprintln!("[warn] HEARTH_MAX_TOKENS={n} 过小（<{MIN}），回退默认 {DEFAULT}");
                DEFAULT
            }
            Ok(n) if n > MAX => {
                eprintln!("[warn] HEARTH_MAX_TOKENS={n} 超上限（>{MAX}），收敛到 {MAX}");
                MAX
            }
            Ok(n) => n,
            Err(_) => {
                eprintln!("[warn] HEARTH_MAX_TOKENS 非法值 {v:?}，回退默认 {DEFAULT}");
                DEFAULT
            }
        },
    }
}

pub fn truncate_marked(s: &str, max_chars: usize) -> String {
    let total = s.chars().count();
    if total <= max_chars {
        return s.to_string();
    }
    let dropped = total - max_chars;
    let mut out: String = s.chars().take(max_chars).collect();
    out.push_str(&format!("\n[... {dropped} chars truncated]"));
    out
}

#[cfg(test)]
mod truncate_marked_tests {
    use super::*;

    /// P4 Node 13：截断必须显式标记（静默截断=信息销毁不可见）。
    #[test]
    fn test_truncate_marked_appends_marker() {
        let short = truncate_marked("hello", 10);
        assert_eq!(short, "hello", "未超限不得加标记");
        let long = truncate_marked("0123456789abc", 10);
        assert!(long.starts_with("0123456789"), "保留前缀");
        assert!(
            long.contains("[... 3 chars truncated]"),
            "须标注被砍字符数：{long}"
        );
        let cjk = truncate_marked("一二三四五六七", 3);
        assert!(
            cjk.contains("[... 4 chars truncated]"),
            "中文字符按 chars 计：{cjk}"
        );
    }

    /// 修复 4（P0）：输出预算 env 解析——默认/合法/非法/下限/上限。
    #[test]
    fn test_max_output_tokens_from_table() {
        // 未设 → 默认 65536（3.0-flash thinking 共享预算约束）
        assert_eq!(max_output_tokens_from(None), 65536, "未设 env 用默认 65536");
        // 合法值直通
        assert_eq!(max_output_tokens_from(Some("8192")), 8192, "显式 8192 直通");
        assert_eq!(max_output_tokens_from(Some(" 32768 ")), 32768, "容忍空白");
        // 非法值 → 默认
        assert_eq!(max_output_tokens_from(Some("abc")), 65536, "非法回退默认");
        assert_eq!(max_output_tokens_from(Some("")), 65536, "空串回退默认");
        // 下限保护（<256 回退默认）
        assert_eq!(max_output_tokens_from(Some("16")), 65536, "过小回退默认");
        assert_eq!(max_output_tokens_from(Some("255")), 65536, "边界 255 回退");
        assert_eq!(max_output_tokens_from(Some("256")), 256, "边界 256 生效");
        // 上限保护
        assert_eq!(max_output_tokens_from(Some("999999")), 131072, "超限收敛");
        // 负数是非法 parse（u32）→ 默认
        assert_eq!(max_output_tokens_from(Some("-5")), 65536, "负数回退默认");
    }
}

// D-76（2026-10-01, traecode）：原 `format_injected_content`（外部内容来源标注 +
// 长度截断）**已删除**——全仓零调用方，防护从未接线（能力缺口见 D-80）。

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::new(
            "m1".into(),
            Role::User,
            MessageContent::Text("hello".into()),
        );
        assert_eq!(msg.id, "m1");
        assert_eq!(msg.role, Role::User);
    }

    #[test]
    fn test_run_state_defaults() {
        let state = RunState::new("test goal".into(), Budget::default());
        assert_eq!(state.goal, "test goal");
        assert_eq!(state.steps_used, 0);
        assert!(state.history.is_empty());
    }

    #[test]
    fn test_tool_call_serde() {
        let tc = ToolCall {
            call_id: "c1".into(),
            name: "bash".into(),
            args: serde_json::json!({"cmd": "cargo test"}),
        };
        let json = serde_json::to_string(&tc).unwrap();
        let back: ToolCall = serde_json::from_str(&json).unwrap();
        assert_eq!(back.call_id, "c1");
        assert_eq!(back.name, "bash");
    }

    #[test]
    fn test_budget_default() {
        let b = Budget::default();
        assert_eq!(b.max_steps, 50);
    }

    // D-76：对已删除的 TaskGraph 家族（类型 + impl）的 4 条单测已随实现一并删除。
}

// ── 6C v6.0: Civilization Line types ──

/// A single entry on the civilization line (collective AI memory).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CivEntry {
    pub id: String,
    pub author: CivAuthor,
    pub content: String,
    pub category: CivCategory,
    pub context: Option<CivContext>,
    pub created_at: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CivAuthor {
    pub provider_model: String,
    pub session_id: String,
    pub bridge_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CivCategory {
    Insight,
    Milestone,
    Lesson,
    Decision,
    Reflection,
    Question,
    Announcement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CivContext {
    pub task_goal: String,
    pub files_touched: Vec<String>,
    pub duration_secs: u64,
}

// ── 6D v6.0: Work Line types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkNode {
    pub id: String,
    pub parent_id: Option<String>,
    pub description: String,
    pub status: WorkStatus,
    pub progress: f32,
    pub category: WorkCategory,
    pub assignee: Option<String>,
    pub deps: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WorkStatus {
    Pending,
    InProgress,
    Blocked,
    Review,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WorkCategory {
    LongTerm,
    ShortTerm,
    Scheduled,
    Completed, // auto-archived on completion
}
// D-76（2026-10-01, traecode）：原 `mod r2d_tests`（T5 next_action_deterministic）
// 已随 `TaskGraph`/`TaskNode`/`next_action_deterministic` 一并删除。
