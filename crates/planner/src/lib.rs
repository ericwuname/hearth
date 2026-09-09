//! Planner crate: task decomposition (decompose → TaskGraph) + reflection (reflect → continue|replan|give_up).
//!
//! P3 core: turns the agent loop from "blind execution" into "strategic planning with stopping judgment".
//! Depends only on llm-gateway (LlmProvider::chat) + agent-types. No direct provider dependency.

use agent_types::{Gap, PlanContext, TaskGraph, TaskNode, TaskStatus};
use anyhow::Result;
use async_trait::async_trait;
use llm_gateway::{ChatRequest, ChatResponse, LlmProvider};
use std::sync::Arc;

/// WP-3 (v23 phase3): 规划缺口推导——plan 产出后的可审查缺口扫描。
/// 门禁契约：每条 gap 必带 from+why（构造器强制）；blocking=false ⇒ auto_assumed=true。
/// blocking=true 的缺口由 loop 走通用交互澄清（kind="clarification"，内核不加分支）。
pub fn derive_gaps(goal: &str, graph: &TaskGraph) -> Vec<Gap> {
    let mut gaps = Vec::new();

    // 1. 目标无来源标注 → missing_goal_source（非阻塞：按用户直接指令执行，已假设）
    //    W8/A3 (RC26): 收紧为"仅真歧义"——goal 引用外部上下文（上文/刚才/该文件
    //    等指代）而无来源标注时才产出。无差别标注是噪音（574×2=1148，每份草案
    //    ×2 渲染问题另在 CLI render 层修复）。用户直接指令本就无需来源。
    let g = goal.to_lowercase();
    const EXTERNAL_REFS: [&str; 9] = [
        "上面",
        "刚才",
        "之前",
        "上一个",
        "同上",
        "该文件",
        "这个文件",
        "那个文件",
        "前面",
    ];
    let has_external_ref = EXTERNAL_REFS.iter().any(|k| g.contains(k));
    if has_external_ref && !g.contains("来源") && !g.contains("source:") && !g.contains("based on")
    {
        gaps.push(Gap::assumed(
            "missing_goal_source",
            "目标引用了外部上下文但未标注来源，按用户直接指令执行（已假设）",
        ));
    }

    // 2. 任务描述含未澄清选项 → ambiguous_option（阻塞：必须澄清才能继续）
    // 每个 plan 最多产出一个阻塞缺口（避免连环澄清——一次只问最关键的一个）
    //
    // B2-A (backend-intelligence 审计发现)：旧检测对"或"字一刀切——LLM 分解的
    // 任务描述常含并列"或"（如"创建 src 目录或检查现有代码"）→ 50% 误触发
    // blocking clarify（产品红线"AI 不得问废话"）。收紧为"二选一实现"语义：
    // 「用/使用/采用/选 + X + 或/或者 + Y + 实现/方案/语言/框架/库」或
    // 「实现/写 + X 或 Y」——真正的技术取舍才阻塞。
    // B2-A 三轮（backend-intelligence 实测）：LLM 分解可能把用户目标的"或"消化掉
    // （"用 Rust 或 Go 实现"→ 任务写成单语言）——**goal（用户原话）优先检测**，
    // task 描述兜底。goal 保留用户意图，歧义不丢失。
    let goal_lower = goal.to_lowercase();
    let goal_ambiguous = {
        let g = &goal_lower;
        let has_or =
            g.contains("或") || g.contains("或者") || g.contains(" or ") || g.contains("either");
        let cn = ["用", "使用", "采用", "选", "基于"]
            .iter()
            .any(|kw| g.contains(kw))
            && has_or
            && (g.contains("实现")
                || g.contains("写")
                || g.contains("语言")
                || g.contains("框架")
                || g.contains("库")
                || g.contains("方案"));
        let en = ["use ", "implement", "write ", "choose", " in "]
            .iter()
            .any(|kw| g.contains(kw))
            && has_or
            && (g.contains("implement")
                || g.contains("language")
                || g.contains("framework")
                || g.contains("library"));
        cn || en || g.contains("待定") || g.contains("可选") || g.contains("tbd")
    };
    if goal_ambiguous {
        let snippet: String = goal_lower.chars().take(60).collect();
        // R10-C2 (v0.1.6): 从目标抽"或"两侧候选做选项式澄清（best-effort，
        // 失败兜底默认两选项）。零内核改动：Gap 数据模型 + payload 透传。
        let opts = extract_options(&goal_lower);
        gaps.push(Gap::blocking_with_options(
            "ambiguous_option",
            format!(
                "目标含未澄清技术选项（'{}'），请选择或补充说明",
                snippet.trim()
            ),
            opts,
        ));
    }
    // WS7 (v0.2): 用户陈述的"事实"默认待验证——弱证据词（据说/听说/我记得/网上说/
    // 应该是/官方说）触发非阻塞 unverified_claim gap：提示"若影响结果先核验"。
    // 非阻塞（assumed）——不打断，只把"这是待验证断言"显式上浮给执行层。
    {
        let weak = [
            "据说",
            "听说",
            "我记得",
            "网上说",
            "应该是",
            "估计是",
            "官方说",
            "文档说",
        ];
        // R2-4 否定前缀检测（对话可用性根治任务书 v1.0；E16）：**"不应该是"
        // 包含子串"应该是"**——contains() 命中 = 误报（真机会话实测 18 次，
        // 同警告重复刷屏）。判定：弱证据词的**未被否定的出现**存在才触发——
        // 否定前缀（不/没/未/非/别/勿/无）紧邻弱证据词 = 用户在表达确定性
        // 判断而非转述传闻。去重：多词命中只取第一条（find 语义保持），
        // 同一 gap 类别单轮 ≤1 条（任务书"同警告 ≤1 次"）。
        let negations = ["不", "没", "未", "非", "别", "勿", "无"];
        if let Some(kw) = weak.iter().find(|k| goal_lower.contains(**k)) {
            let mut has_unnegated = false;
            let mut from = 0;
            while let Some(pos) = goal_lower[from..].find(*kw) {
                let abs = from + pos;
                let prefix = &goal_lower[..abs];
                if !negations.iter().any(|n| prefix.ends_with(n)) {
                    has_unnegated = true;
                    break;
                }
                from = abs + kw.len();
            }
            if has_unnegated {
                gaps.push(Gap::assumed(
                    "unverified_claim",
                    format!(
                        "用户陈述含弱证据词'{}'——该断言默认未验证；若影响结果正确性，落地前须核验（读源码/跑命令/查官方）",
                        kw
                    ),
                ));
            }
        }
    }
    for node in &graph.nodes {
        let d = node.description.to_lowercase();
        // 二选一实现模式（真歧义）——B2-A 二轮收紧：
        // 中文「用/使用/采用/选/基于 + 或 + 实现/写/语言/框架/库」；
        // 英文「(use|implement|write|choose|in) + (or|either) + (implement|language|framework|library)」。
        let has_or =
            d.contains("或") || d.contains("或者") || d.contains(" or ") || d.contains("either");
        let cn_choice = ["用", "使用", "采用", "选", "基于"]
            .iter()
            .any(|kw| d.contains(kw))
            && has_or
            && (d.contains("实现")
                || d.contains("写")
                || d.contains("语言")
                || d.contains("框架")
                || d.contains("库")
                || d.contains("方案"));
        let en_choice = ["use ", "implement", "write ", "choose", " in "]
            .iter()
            .any(|kw| d.contains(kw))
            && has_or
            && (d.contains("implement")
                || d.contains("language")
                || d.contains("framework")
                || d.contains("library"));
        let real_choice = cn_choice || en_choice;
        // 显式待定/可选（用户未拍板）——B2-A 二轮收紧：去掉 or/either 泛匹配
        // （LLM 分解的英文任务描述常见 "or"（并列工具/动作）→ 50% 误触发 blocking）。
        // 只保留强"未拍板"信号：中文待定词 + tbd。
        let explicit_pending = d.contains("待定") || d.contains("可选") || d.contains("tbd");
        if real_choice || explicit_pending {
            // why 带上选项片段（可追溯：用户能看到"什么或什么"）
            let snippet: String = d.chars().take(60).collect();
            // R10-C2 (v0.1.6): 选项式澄清（与 goal 级同构）
            let opts = extract_options(&d);
            gaps.push(Gap::blocking_with_options(
                "ambiguous_option",
                format!(
                    "任务 '{}' 含未澄清技术选项（'{}'），请选择或补充说明",
                    node.id,
                    snippet.trim()
                ),
                opts,
            ));
            break;
        }
    }

    gaps
}

/// R10-C2 (v0.1.6): 从含"或"的目标/任务描述抽二选一候选选项。
/// 按首个 or-sep（或/或者/ or /either）切左右片段，各 trim 到 ≤20 字符；
/// 失败/无候选兜底默认两选项（best-effort，不阻塞主路径）。
fn extract_options(s: &str) -> Vec<String> {
    // 找最早出现的 or-sep（"或"1 字符/"或者"2/" or "4/"either"6——切点按实际长度，
    // 修 R10-C2 bug：旧 `s[i+2..]` 对单字符"或"会跳过 2 字节 → 越界/错切）
    let hit = ["或", "或者", " or ", "either"]
        .iter()
        .filter_map(|sep| s.find(sep).map(|i| (sep, i)))
        .min_by_key(|(_, i)| *i);
    if let Some((sep, i)) = hit {
        // T8 (v0.2.3): 用 trim 保留词间空格——旧实现 filter(非空白) 把多词选项
        // "recall the rules of china" 压成 "recalltherulesofchin"（21 截 20 再丢尾）。
        let left: String = s[..i].trim().to_string();
        let right: String = s[i + sep.len()..].trim().to_string();
        let mut opts = Vec::new();
        for o in [left, right] {
            if !o.is_empty() {
                // 限长按词截断（保留完整词），不删空格
                let t: String = o.split_whitespace().take(5).collect::<Vec<_>>().join(" ");
                opts.push(t);
            }
            if opts.len() >= 2 {
                break;
            }
        }
        if opts.len() == 2 {
            return opts;
        }
    }
    vec!["按默认实现".into(), "我另有指定".into()]
}

/// The Planner trait — task decomposition and reflection.
#[async_trait]
pub trait Planner: Send + Sync {
    /// Decompose a goal into a structured TaskGraph (DAG).
    async fn decompose(&self, goal: &str, ctx: &PlanContext) -> Result<TaskGraph>;
}

/// WS3 (v0.1.3): 单节点 fallback——网络/解析失败时任务不卡死（原内联逻辑抽公共函数）。
fn single_node(goal: &str) -> TaskNode {
    TaskNode {
        id: "execute".into(),
        description: goal.to_string(),
        deps: vec![],
        status: TaskStatus::Pending,
        delegable: false,
        result: None,
    }
}

fn single_node_graph(goal: &str) -> TaskGraph {
    TaskGraph {
        nodes: vec![single_node(goal)],
    }
}

// ── DefaultPlanner ──

pub struct DefaultPlanner {
    provider: Arc<dyn LlmProvider>,
    /// WS3 (v0.1.3 R2/B4): 规划失败短时缓存——同 goal 5s 内不重复打 20-25s 的
    /// 规划调用（真机：EOF/网络失败每次 replan 都重打 25s，任务总时长爆炸）。
    /// 只缓存"失败→单节点 fallback"（每次生成新 TaskGraph，无共享可变状态风险）。
    fail_cache: std::sync::Mutex<std::collections::HashMap<String, std::time::Instant>>,
}

impl DefaultPlanner {
    pub fn new(provider: Arc<dyn LlmProvider>) -> Self {
        Self {
            provider,
            fail_cache: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// v12.4: LLM calls made by the planner previously used a bare `.await?`, so a
    /// single transient failure (network reset, HTTP 429/1302 rate limit, provider
    /// 5xx) propagated straight up and killed the whole session at Plan step 1.
    /// This mirrors the exponential-backoff retry already present in the agent loop.
    async fn chat_with_retry(&self, req: ChatRequest) -> Result<ChatResponse> {
        const MAX_RETRIES: u32 = 6;
        let mut attempt = 0u32;
        loop {
            match self.provider.chat(req.clone()).await {
                Ok(resp) => return Ok(resp),
                Err(e) => {
                    attempt += 1;
                    let msg = e.to_string();
                    // D4 (hearth-cli R5): 认证/权限错误（401/403）重试无意义——立即失败，
                    // 坏 key 时避免浪费重试窗口（CLI 才能快速给可行动错误）。
                    let is_auth = msg.contains("401") || msg.contains("403");
                    if attempt >= MAX_RETRIES || is_auth {
                        tracing::error!(error = %e, attempt, "planner chat failed after retries");
                        return Err(e);
                    }
                    // Rate limits need a longer cool-down than plain network blips.
                    let is_rate_limit = msg.contains("429")
                        || msg.contains("1302")
                        || msg.contains("rate limit")
                        || msg.contains("Too Many Requests");
                    let delay = if is_rate_limit {
                        // 5,10,20,40,60 — quota windows are usually per-minute.
                        (5u64 << (attempt - 1).min(3)).min(60)
                    } else {
                        2u64.pow(attempt)
                    };
                    tracing::warn!(
                        error = %e,
                        attempt,
                        delay_sec = delay,
                        "planner chat failed, retrying"
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                }
            }
        }
    }
}

#[async_trait]
impl Planner for DefaultPlanner {
    async fn decompose(&self, goal: &str, ctx: &PlanContext) -> Result<TaskGraph> {
        // WS3 (v0.1.3): 失败缓存命中——最近 5s 内该 goal 规划失败过 → 直接单节点
        // fallback，不重打 20-25s 的 LLM 规划调用（R2 预算药方）。
        let cache_key = goal.to_string();
        {
            let cache = self.fail_cache.lock().unwrap();
            if let Some(t) = cache.get(&cache_key) {
                if t.elapsed() < std::time::Duration::from_secs(5) {
                    tracing::warn!(
                        "plan fail-cache hit for goal (recent failure) — single-node fallback without LLM call"
                    );
                    return Ok(single_node_graph(goal));
                }
            }
        }

        // Build a prompt asking the LLM to decompose the goal into tasks
        // WS3 (v0.1.3): deepseek prompt 适配——真机 8 次 `EOF at line 1 column 0`
        // （deepseek 不吐 JSON 或包 markdown）。加强约束：纯 JSON、禁代码块、
        // 给完整 few-shot 示例、失败允许输出 []。
        let mut prompt = format!(
            "You are a task planner. Decompose the following goal into a structured plan.\n\
             Output a JSON array of task objects. Each task must have:\n\
             - \"id\": short unique identifier (snake_case)\n\
             - \"description\": what to do\n\
             - \"deps\": array of task ids that must complete first (empty if none)\n\
             - \"delegable\": true if this task can be delegated to a sub-agent\n\n\
             STRICT FORMAT RULES (deepseek compatibility):\n\
             - Output ONLY the raw JSON array. No markdown code fences (no ```json), no\n\
               explanation, no comments, no trailing text.\n\
             - The first character MUST be '[' and the last character MUST be ']'.\n\
             - If the goal is trivial (a simple question or a single step), output\n\
               [{{\"id\":\"do_it\",\"description\":\"<goal>\",\"deps\":[],\"delegable\":false}}].\n\n\
             Example (correct):\n\
             [{{\"id\":\"setup\",\"description\":\"Set up project\",\"deps\":[],\"delegable\":false}},{{\"id\":\"code\",\"description\":\"Write the code\",\"deps\":[\"setup\"],\"delegable\":false}}]\n\n\
             IMPORTANT: sub-agents are READ-ONLY (read/grep/glob only, no write tools).\n\
             ONLY mark delegable=true for pure research/lookup tasks. Any task that\n\
             modifies code (write/edit/refactor/create/delete files) MUST be\n\
             delegable=false — the parent agent must do the edit itself.\n\n\
             Goal: {}\n\n\
             Available tools: {}\n",
            goal,
            ctx.available_tools.join(", ")
        );

        // R1: Inject P2 intelligence into decomposition prompt
        if let Some(ref retrieval) = ctx.retrieval_context {
            if !retrieval.is_empty() {
                prompt.push_str(&format!("\nRelevant code context:\n{}\n", retrieval));
            }
        }
        if let Some(ref diags) = ctx.lsp_diagnostics {
            if !diags.is_empty() {
                prompt.push_str(&format!("\nLSP diagnostics (issues to fix):\n{}\n", diags));
            }
        }

        prompt.push_str("\nOutput ONLY the JSON array, no other text. Example: [{\"id\":\"setup\",\"description\":\"Set up project\",\"deps\":[],\"delegable\":false}]");

        let resp = self
            .chat_with_retry(ChatRequest {
                messages: vec![
                    agent_types::Message::new(
                        "sys".into(),
                        agent_types::Role::System,
                        agent_types::MessageContent::Text(prompt),
                    ),
                    agent_types::Message::new(
                        "user".into(),
                        agent_types::Role::User,
                        agent_types::MessageContent::Text("Decompose the goal into tasks.".into()),
                    ),
                ],
                tools: vec![],
                temperature: Some(0.2),
                // R9-B3 (v0.1.6): 2048 → 8192——与主循环对齐（loop.rs max_tokens=8192）；
                // 象棋/8000 字长文等大任务目标分解需要大输出（真机 2048 截断 → 空/坏 JSON）。
                max_tokens: Some(8192),
                stream: false,
            })
            .await;

        // v0.1.1 用户实测（象棋 20:21-20:28）：deepseek 瞬时 API/网络故障时
        // `read body: error decoding response body` 6 次重试全失败 → do_plan Err
        // → 整个任务失败。鲁棒性：plan 网络错误 → fallback 单节点计划（任务可继续，
        // 不强依赖规划质量；JSON 解析失败已有 fallback，网络错误同样降级）。
        let resp = match resp {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(
                    "plan chat failed ({e:.120}) — fallback to single-node plan (network degraded)"
                );
                // WS3 (v0.1.3): 记录失败缓存——同 goal 5s 内不再重打
                self.fail_cache
                    .lock()
                    .unwrap()
                    .insert(cache_key.clone(), std::time::Instant::now());
                return Ok(single_node_graph(goal));
            }
        };

        // R7 (v0.1.4): 空响应 ≠ 无任务——content 为 None/空 = provider 故障
        // （HTTP 200 但空体/截断），上抛 Transient 让 loop 层按瞬时故障重试，
        // 不能静默当 [] 降级单节点（否则"provider 挂了"被伪装成"任务无需分解"）。
        let json_text = match resp.content {
            Some(c) if !c.trim().is_empty() => c,
            _ => {
                tracing::warn!("plan chat returned EMPTY content — provider fault, surfacing");
                return Err(llm_gateway::LlmError::Transient(
                    "planner: empty LLM response (provider fault)".into(),
                )
                .into());
            }
        };
        let json_text = json_text.trim();

        // Try to extract JSON array from the response (LLM may wrap in markdown)
        let json_text = if let Some(start) = json_text.find('[') {
            if let Some(end) = json_text.rfind(']') {
                &json_text[start..=end]
            } else {
                json_text
            }
        } else {
            json_text
        };

        // Parse into TaskGraph
        let nodes: Vec<TaskNode> = match serde_json::from_str::<Vec<serde_json::Value>>(json_text) {
            Ok(arr) => arr
                .into_iter()
                .enumerate()
                .map(|(i, v)| TaskNode {
                    id: v["id"]
                        .as_str()
                        .unwrap_or(&format!("task_{}", i))
                        .to_string(),
                    description: v["description"].as_str().unwrap_or("unknown").to_string(),
                    deps: v["deps"]
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(|d| d.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default(),
                    status: TaskStatus::Pending,
                    delegable: v["delegable"].as_bool().unwrap_or(false),
                    result: None,
                })
                .collect(),
            Err(e) => {
                tracing::warn!(
                    "Failed to parse LLM task plan as JSON: {e}. Using fallback single-node plan."
                );
                // WS3 (v0.1.3): 解析失败也记录缓存（EOF 高频——同 goal 不重打 25s）
                self.fail_cache
                    .lock()
                    .unwrap()
                    .insert(cache_key.clone(), std::time::Instant::now());
                vec![single_node(goal)]
            }
        };

        let tg = TaskGraph { nodes };

        tracing::info!(
            "Decomposed goal into {} tasks (acyclic={})",
            tg.nodes.len(),
            tg.is_acyclic()
        );

        Ok(tg)
    }
}

// ── Tests ──

#[cfg(test)]
mod tests {

    // ── WP-3 (v23 phase3): 规划缺口推导门禁 ──

    fn gap_graph(desc: &str) -> TaskGraph {
        TaskGraph {
            nodes: vec![TaskNode {
                id: "t1".into(),
                description: desc.into(),
                deps: vec![],
                status: TaskStatus::Pending,
                delegable: false,
                result: None,
            }],
        }
    }
    use super::*;
    use agent_types::{TaskGraph, TaskNode, TaskStatus};
    use futures::stream::{self, BoxStream};
    use llm_gateway::{Capabilities, ChatResponse, Embedding, StreamEvent};

    /// Mock LLM provider for testing.
    struct MockProvider {
        chat_response: std::sync::Mutex<Vec<ChatResponse>>,
    }

    impl MockProvider {
        fn new(responses: Vec<ChatResponse>) -> Self {
            Self {
                chat_response: std::sync::Mutex::new(responses),
            }
        }
    }

    #[async_trait]
    impl LlmProvider for MockProvider {
        fn name(&self) -> &str {
            "mock"
        }
        fn model(&self) -> &str {
            "mock"
        }
        fn capabilities(&self) -> Capabilities {
            Capabilities {
                chat: true,
                stream: false,
                function_calling: false,
                embeddings: false,
                max_context_tokens: Some(4096),
            }
        }
        async fn chat(&self, _req: ChatRequest) -> Result<ChatResponse> {
            let mut r = self.chat_response.lock().unwrap();
            if r.is_empty() {
                Ok(ChatResponse {
                    content: Some("[]".into()),
                    tool_calls: vec![],
                    finish_reason: Some("stop".into()),
                    usage: None,
                    reasoning_content: None,
                })
            } else {
                Ok(r.remove(0))
            }
        }
        fn stream(&self, _req: ChatRequest) -> BoxStream<'static, Result<StreamEvent>> {
            Box::pin(stream::empty())
        }
        async fn embed(&self, _inputs: &[String]) -> Result<Vec<Embedding>> {
            Ok(vec![])
        }
    }

    fn make_plan_ctx() -> PlanContext {
        PlanContext {
            goal: "write a Rust function to parse JSON".into(),
            retrieval_context: Some(
                "// src/parser.rs\npub fn parse(input: &str) -> Result<Value> { ... }".into(),
            ),
            lsp_diagnostics: Some("src/parser.rs:10:5: ERROR: unused variable `x`".into()),
            available_tools: vec!["bash".into(), "read".into(), "edit".into()],
        }
    }

    // ── A1: decompose builds a DAG ──

    #[tokio::test]
    async fn test_p3_decompose_builds_dag() {
        let json_plan = r#"[
            {"id":"read_code","description":"Read the parser source","deps":[],"delegable":false},
            {"id":"write_fn","description":"Write the parse function","deps":["read_code"],"delegable":false},
            {"id":"test_fn","description":"Test the function","deps":["write_fn"],"delegable":true}
        ]"#;

        let mock = Arc::new(MockProvider::new(vec![ChatResponse {
            content: Some(json_plan.into()),
            tool_calls: vec![],
            finish_reason: Some("stop".into()),
            usage: None,
            reasoning_content: None,
        }]));

        let planner = DefaultPlanner::new(mock);
        let ctx = make_plan_ctx();
        let tg = planner
            .decompose("write a Rust function to parse JSON", &ctx)
            .await
            .unwrap();

        eprintln!("Decomposed graph: {} nodes", tg.nodes.len());
        for n in &tg.nodes {
            eprintln!(
                "  {} (deps: {:?}, delegable: {})",
                n.id, n.deps, n.delegable
            );
        }

        // A1: node count > 0
        assert!(!tg.nodes.is_empty(), "should have at least one node");

        // A1: acyclic
        assert!(tg.is_acyclic(), "plan must be acyclic");

        // A1: topo order satisfies deps
        let order = tg.topo_order().unwrap();
        let read_pos = order.iter().position(|&i| tg.nodes[i].id == "read_code");
        let write_pos = order.iter().position(|&i| tg.nodes[i].id == "write_fn");
        let test_pos = order.iter().position(|&i| tg.nodes[i].id == "test_fn");
        assert!(read_pos < write_pos, "read_code before write_fn");
        assert!(write_pos < test_pos, "write_fn before test_fn");

        // A1: covers goal sub-intents (at least "parse" somewhere in descriptions)
        let all_desc: String = tg
            .nodes
            .iter()
            .map(|n| &n.description)
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            all_desc.contains("parse") || all_desc.contains("Parse"),
            "plan should cover goal intent"
        );

        eprintln!("A1 PASS: decompose builds valid DAG");
    }

    // ── A1: TaskGraph no-cycle test ──

    #[test]
    fn test_p3_taskgraph_no_cycle() {
        // Valid DAG
        let valid = TaskGraph {
            nodes: vec![
                TaskNode {
                    id: "a".into(),
                    description: "a".into(),
                    deps: vec![],
                    status: TaskStatus::Pending,
                    delegable: false,
                    result: None,
                },
                TaskNode {
                    id: "b".into(),
                    description: "b".into(),
                    deps: vec!["a".into()],
                    status: TaskStatus::Pending,
                    delegable: false,
                    result: None,
                },
            ],
        };
        assert!(valid.is_acyclic());

        // Cycle: a → b → a
        let cyclic = TaskGraph {
            nodes: vec![
                TaskNode {
                    id: "a".into(),
                    description: "a".into(),
                    deps: vec!["b".into()],
                    status: TaskStatus::Pending,
                    delegable: false,
                    result: None,
                },
                TaskNode {
                    id: "b".into(),
                    description: "b".into(),
                    deps: vec!["a".into()],
                    status: TaskStatus::Pending,
                    delegable: false,
                    result: None,
                },
            ],
        };
        assert!(!cyclic.is_acyclic());
    }

    #[test]
    fn test_derive_gaps_missing_goal_source_non_blocking() {
        // W8/A3 (RC26): 收紧为"仅真歧义"——goal 引用外部上下文才标注
        let gaps = derive_gaps("按上面说的写一个 CLI 工具", &gap_graph("do the thing"));
        let g = gaps.iter().find(|g| g.from == "missing_goal_source");
        assert!(
            g.is_some(),
            "引用外部上下文且无来源 → missing_goal_source gap"
        );
        let g = g.unwrap();
        assert!(!g.blocking, "来源缺失非阻塞");
        assert!(g.auto_assumed, "非阻塞 gap 必须 auto_assumed=true");

        // 用户直接指令（无外部指代）→ 不再无差别标注（RC26 噪音治理）
        let gaps2 = derive_gaps("写一个 CLI 工具", &gap_graph("do the thing"));
        assert!(
            !gaps2.iter().any(|g| g.from == "missing_goal_source"),
            "无外部指代的直接指令不得标注 missing_goal_source（RC26）"
        );
        eprintln!("WP-3 PASS: missing_goal_source 非阻塞 + auto_assumed + RC26 收紧");
    }

    /// 回归 WS7 (v0.2): 用户陈述含弱证据词（"据说"等）→ 非阻塞 unverified_claim gap
    /// （待验证断言显式上浮，不打断）。无弱证据词 → 不出。无此 gap 时此测试红。
    #[test]
    fn test_derive_gaps_unverified_claim() {
        let gaps = derive_gaps(
            "据说本项目用 Rust 框架，帮我加个功能",
            &gap_graph("add feature"),
        );
        let g = gaps.iter().find(|g| g.from == "unverified_claim");
        assert!(g.is_some(), "含'据说'必须出 unverified_claim gap");
        let g = g.unwrap();
        assert!(!g.blocking, "unverified_claim 非阻塞（提示核验，不打断）");
        assert!(g.auto_assumed, "非阻塞 gap 必须 auto_assumed=true");

        // 无弱证据词的正常目标不出（防误报）
        let gaps2 = derive_gaps("实现快速排序并附单元测试", &gap_graph("sort"));
        assert!(
            !gaps2.iter().any(|g| g.from == "unverified_claim"),
            "正常目标不应误报 unverified_claim"
        );

        // R2-4（对话可用性根治任务书 v1.0；E16）：否定前缀 → 零误报——
        // "不应该是"包含子串"应该是"，旧 contains() 误报（真机 18 次）。
        let gaps3 = derive_gaps("结果不应该是乱码，帮我修复输出编码", &gap_graph("fix"));
        assert!(
            !gaps3.iter().any(|g| g.from == "unverified_claim"),
            "被否定的弱证据词（'不应该是'）不得触发 unverified_claim（E16 零误报判据），实际 {:?}",
            gaps3
                .iter()
                .filter(|g| g.from == "unverified_claim")
                .collect::<Vec<_>>()
        );
        // 同词未否定出现仍触发（否定检测不误杀真传闻）
        let gaps4 = derive_gaps(
            "据说这个库不应该是线程安全的，你确认下",
            &gap_graph("check"),
        );
        assert!(
            gaps4.iter().any(|g| g.from == "unverified_claim"),
            "'据说'未被否定 → 仍须触发（否定检测不得误杀真弱证据）"
        );
        eprintln!("WS7 PASS: unverified_claim 上浮（弱证据词触发）");
    }

    /// T8 (v0.2.3): 多词选项保留空格——"recall the rules of china" 不得压成
    /// "recalltherulesofchin"（旧 bug：filter 删空格 + take(20) 截尾）。
    #[test]
    fn test_extract_options_preserves_spaces() {
        let opts = extract_options("应该 recall the rules of china 或 implement the scheduler");
        assert!(opts.len() >= 2, "应抽出两侧候选: {opts:?}");
        assert!(
            opts.iter().any(|o| o.contains("recall the rules")),
            "左侧多词选项必须保留空格: {opts:?}"
        );
        assert!(
            opts.iter().any(|o| o.contains("implement the scheduler")),
            "右侧多词选项必须保留空格: {opts:?}"
        );
        // 单侧不得为空
        assert!(opts.iter().all(|o| !o.trim().is_empty()), "候选不得为空");
    }

    #[test]
    fn test_derive_gaps_ambiguous_option_blocking() {
        let gaps = derive_gaps("实现一个工具", &gap_graph("用 Rust 或 Go 实现"));
        let amb = gaps.iter().find(|g| g.from == "ambiguous_option");
        assert!(amb.is_some(), "含'或'任务必出 ambiguous_option gap");
        assert!(amb.unwrap().blocking, "未澄清选项必须阻塞");
        // R10-C2 (v0.1.6): 选项式澄清——options 非空且抽到"或"两侧候选
        let amb = amb.unwrap();
        assert!(!amb.options.is_empty(), "选项式澄清必须有候选选项");
        assert!(
            amb.options.iter().any(|o| o.contains("Rust"))
                || amb.options.iter().any(|o| o.contains("rust")),
            "选项应含左侧候选（Rust），got: {:?}",
            amb.options
        );
        eprintln!(
            "WP-3 PASS: ambiguous_option 阻塞 + 选项式（{:?}）",
            amb.options
        );
    }

    #[test]
    fn test_derive_gaps_clean_plan_no_blocking() {
        let gaps = derive_gaps(
            "实现 CLI 工具（来源: 需求文档）",
            &gap_graph("实现 CLI 的 help 子命令"),
        );
        assert!(
            gaps.iter().all(|g| g.auto_assumed),
            "无阻塞 plan 所有 gap 必须 auto_assumed=true, got {:?}",
            gaps
        );
        assert!(
            gaps.iter().all(|g| !g.from.is_empty() && !g.why.is_empty()),
            "gap 必须带 from+why"
        );
        eprintln!("WP-3 PASS: 干净 plan 无阻塞 + 全部带 from/why/assume");
    }
}
