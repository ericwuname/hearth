//! R2-A (v0.2.6): Prompt Cache 证据采集——"先测量，再定位，再优化"。
//!
//! 目的（派工令 R2-02/R2-A）：ContextBuilder（G5）重构前必须有 ≥20 请求的
//! 真实 cache 数据。禁止先重构 build_messages() 再解释 cache 结果。
//!
//! 协议（批注 2/修正 2）：每 LLM 请求成功后追加一行 JSONL：
//! `{ts, session_id, provider, model, prompt_tokens, completion_tokens,
//!   total_tokens, cache_hit_tokens, cache_miss_tokens,
//!   msgs_count, system_hash, prefix_hash, full_hash}`
//!
//! hash 语义（12 位截断 sha256，可读性优先）：
//! - `system_hash`：首条 system 消息内容 hash——同一会话内是否字节稳定（核心指标）
//! - `prefix_hash`：除末条外全部消息 hash——稳定前缀是否真的稳定
//! - `full_hash`：全部消息 hash——相邻请求变化量
//!
//! 开关：env `HEARTH_CACHE_TELEMETRY` = jsonl 路径（未设 = 关闭，零开销）。
//! 采集失败只 warn 不阻断主流程（telemetry 是增益不是依赖）。

use crate::Usage;

/// 每请求一条遥测记录。
pub struct CacheTelemetryRecord<'a> {
    pub session_id: &'a str,
    pub provider: &'a str,
    pub model: &'a str,
    pub usage: Option<&'a Usage>,
    pub msgs_count: usize,
    pub system_hash: &'a str,
    pub prefix_hash: &'a str,
    pub full_hash: &'a str,
    /// Closure-1: 逐消息 hash chain（每消息一条 8-hex；空请求为空数组）。
    pub msg_chain: &'a [String],
    /// Closure-2: tool schema 全量 hash（只观测）。
    pub tools_hash: &'a str,
}

/// Closure-1（顶层复核 §1）：逐消息 hash chain——
/// `chain_k = H(chain_{k-1} ‖ role ‖ content_hash)`，每消息一条 8-hex。
/// 离线分析据此可算相邻请求公共前缀长度（B 层 Message Prefix Stability），
/// 并区分 system 消息 / stable prefix / dynamic tail（role 字段在链输入中）。
/// Debug 格式在本构建内确定性一致（遥测用途，不跨版本比较）。
pub fn message_chain(messages: &[agent_types::Message]) -> Vec<String> {
    use sha2::{Digest, Sha256};
    let mut chain: Vec<String> = Vec::with_capacity(messages.len());
    let mut prev = String::from("genesis");
    for m in messages {
        let mut h = Sha256::new();
        h.update(prev.as_bytes());
        h.update(format!("{:?}", m.role).as_bytes());
        h.update(format!("{:?}", m.content).as_bytes());
        let digest = h.finalize();
        let hex = format!("{:x}", digest);
        let cur = hex[..8].to_string();
        chain.push(cur.clone());
        prev = cur;
    }
    chain
}

/// Closure-2（顶层复核 §2）：tool schema 全量 hash——只观测（`ChatRequest.tools`
/// 保持全量稳定，本 hash 用于**证明**其稳定，不做 phase pruning、不改注册生命周期）。
pub fn compute_tools_hash(tools: &[crate::ToolSchema]) -> String {
    let bytes = serde_json::to_vec(tools).unwrap_or_default();
    short_hash(&bytes)
}

/// 12 位截断 sha256（hex）——碰撞概率对 50 请求级采集可忽略。
pub fn short_hash(input: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(input);
    let out = h.finalize();
    out.iter().map(|b| format!("{b:02x}")).collect::<String>()[..12].to_string()
}

/// 从 ChatRequest.messages 提取三段 hash（序列化用 serde_json 保证字节确定性——
/// 同一请求重发必须同 hash；不同请求差异在 hash 上必须可见）。
pub fn hashes_for(messages: &[agent_types::Message]) -> (String, String, String) {
    let system_hash = messages
        .first()
        .filter(|m| matches!(m.role, agent_types::Role::System))
        .map(|m| {
            short_hash(
                serde_json::to_string(&m.content)
                    .unwrap_or_default()
                    .as_bytes(),
            )
        })
        .unwrap_or_else(|| "none".into());
    // prefix = 除最后一条外的全部消息（最后一条通常是本轮 user/工具结果——动态）
    let prefix_hash = if messages.len() > 1 {
        let sers: Vec<String> = messages[..messages.len() - 1]
            .iter()
            .map(|m| serde_json::to_string(m).unwrap_or_default())
            .collect();
        short_hash(sers.join("\n").as_bytes())
    } else {
        "none".into()
    };
    let full_hash = {
        let sers: Vec<String> = messages
            .iter()
            .map(|m| serde_json::to_string(m).unwrap_or_default())
            .collect();
        short_hash(sers.join("\n").as_bytes())
    };
    (system_hash, prefix_hash, full_hash)
}

/// 追加一条记录到 jsonl（best-effort）。dir/file 来自 env `HEARTH_CACHE_TELEMETRY`。
pub fn record(rec: CacheTelemetryRecord) {
    record_with_exit(rec, "loop")
}

/// v2: 带 exit 标签（TelemetryProvider 装饰器传 "cli"/"service"——区分来源）。
pub fn record_with_exit(rec: CacheTelemetryRecord, exit: &str) {
    use std::io::Write as _;
    let Ok(path) = std::env::var("HEARTH_CACHE_TELEMETRY") else {
        return; // 未开启采集——零开销
    };
    let (hit, miss) = rec
        .usage
        .map(|u| (u.prompt_cache_hit_tokens, u.prompt_cache_miss_tokens))
        .unwrap_or((None, None));
    let entry = serde_json::json!({
        "ts": chrono::Utc::now().to_rfc3339(),
        "session_id": rec.session_id,
        "provider": rec.provider,
        "model": rec.model,
        "prompt_tokens": rec.usage.map(|u| u.prompt_tokens),
        "completion_tokens": rec.usage.map(|u| u.completion_tokens),
        "total_tokens": rec.usage.map(|u| u.total_tokens),
        "cache_hit_tokens": hit,
        "cache_miss_tokens": miss,
        "msgs_count": rec.msgs_count,
        "system_hash": rec.system_hash,
        "prefix_hash": rec.prefix_hash,
        "full_hash": rec.full_hash,
        "exit": exit,
        // Closure-1/2: B 层 hash 链 + tool schema hash（只观测）。
        "msg_chain": rec.msg_chain,
        "chain_head": rec.msg_chain.last().cloned().unwrap_or_default(),
        "tools_hash": rec.tools_hash,
        // 补充 1/补充 4 固定分析口径：phase 为 msgs_count 推导（主链 = msgs>=3
        // 的正式组装请求；planner decompose/reflect 独立调用 msgs<=2 = aux）。
        //gateway 层拿不到真实 phase——此推导与离线分段口径一致，防混统计。
        "phase": if rec.msgs_count >= 3 { "main" } else { "aux" },
    });
    let mut line = match serde_json::to_string(&entry) {
        Ok(l) => l,
        Err(e) => {
            tracing::warn!(error = %e, "cache telemetry serialize failed");
            return;
        }
    };
    line.push('\n');
    if let Some(parent) = std::path::Path::new(&path).parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            tracing::warn!(error = %e, "cache telemetry dir create failed");
            return;
        }
    }
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        Ok(mut f) => {
            if let Err(e) = f.write_all(line.as_bytes()) {
                tracing::warn!(error = %e, "cache telemetry write failed");
            }
        }
        Err(e) => tracing::warn!(error = %e, "cache telemetry open failed"),
    }
}

/// v1 便捷入口（保留兼容签名——session_id 已知场景）。
pub fn record_from_messages(
    session_id: &str,
    provider: &str,
    model: &str,
    messages: &[agent_types::Message],
    usage: Option<&Usage>,
) {
    record_from_messages_full(session_id, provider, model, messages, usage, "loop");
}

/// Closure-1/2 统一入口：TelemetryProvider 调用——chain/tools_hash 在此计算，
/// schema 三项（msg_chain/tools_hash/phase）一次落（守门员补充 1：合并一次动刀）。
pub fn record_chat_full(
    session_id: &str,
    provider: &str,
    model: &str,
    messages: &[agent_types::Message],
    usage: Option<&Usage>,
    exit: &str,
    tools: &[crate::ToolSchema],
) {
    if std::env::var("HEARTH_CACHE_TELEMETRY").is_err() {
        return;
    }
    let (system_hash, prefix_hash, full_hash) = hashes_for(messages);
    let chain = message_chain(messages);
    let thash = compute_tools_hash(tools);
    let rec = CacheTelemetryRecord {
        session_id,
        provider,
        model,
        usage,
        msgs_count: messages.len(),
        system_hash: &system_hash,
        prefix_hash: &prefix_hash,
        full_hash: &full_hash,
        msg_chain: &chain,
        tools_hash: &thash,
    };
    record_with_exit(rec, exit);
}

/// v2 便捷入口（TelemetryProvider 装饰器调用——全出口单点，带 exit 标签）。
pub fn record_from_messages_full(
    session_id: &str,
    provider: &str,
    model: &str,
    messages: &[agent_types::Message],
    usage: Option<&Usage>,
    exit: &str,
) {
    if std::env::var("HEARTH_CACHE_TELEMETRY").is_err() {
        return;
    }
    let (system_hash, prefix_hash, full_hash) = hashes_for(messages);
    let chain = message_chain(messages);
    let rec = CacheTelemetryRecord {
        session_id,
        provider,
        model,
        usage,
        msgs_count: messages.len(),
        system_hash: &system_hash,
        prefix_hash: &prefix_hash,
        full_hash: &full_hash,
        msg_chain: &chain,
        tools_hash: "",
    };
    record_with_exit(rec, exit);
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_types::{Message, MessageContent, Role};

    /// env 是进程全局——并行测试互删变量（教训：context.rs 同款）。
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_hashes_stable_and_sensitive() {
        // 注意：Message::new 会打 Utc::now() 时间戳——夹具必须 clone 复用对象
        // （模拟运行时语义：历史消息对象在相邻请求间被复用，序列化字节稳定）。
        let m1 = vec![
            Message::new(
                "u0".into(),
                Role::User,
                MessageContent::Text("hello".into()),
            ),
            Message::new(
                "u1".into(),
                Role::User,
                MessageContent::Text("world".into()),
            ),
        ];
        let (s1, p1, f1) = hashes_for(&m1);
        // 稳定性：同输入重算 hash 一致（stable prefix 前提——序列化字节确定）
        let (s2, p2, f2) = hashes_for(&m1);
        assert_eq!((s1.clone(), p1.clone(), f1.clone()), (s2, p2, f2));
        // 敏感性：仅末条内容变化 → full_hash 变、prefix_hash 不变
        // （首条 clone 复用——运行时历史对象复用语义）
        let m2 = vec![
            m1[0].clone(),
            Message::new(
                "u1".into(),
                Role::User,
                MessageContent::Text("WORLD".into()),
            ),
        ];
        let (_, p3, f3) = hashes_for(&m2);
        assert_ne!(f1, f3, "消息内容变化必须反映在 full_hash");
        // prefix 语义：只变最后一条 → prefix_hash 不变（稳定前缀判定的关键性质）
        assert_eq!(p1, p3, "仅末条变化时 prefix_hash 必须不变");
        assert_ne!(f1, f3);
    }

    #[test]
    fn test_record_appends_jsonl() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = std::env::temp_dir().join(format!("hearth_ct_{}", uuid::Uuid::new_v4()));
        std::env::set_var("HEARTH_CACHE_TELEMETRY", dir.join("ct.jsonl"));
        let usage = Usage {
            prompt_tokens: 100,
            completion_tokens: 20,
            total_tokens: 120,
            prompt_cache_hit_tokens: Some(80),
            prompt_cache_miss_tokens: Some(20),
        };
        let msgs = vec![
            Message::new(
                "sys".into(),
                Role::System,
                MessageContent::Text("sys prompt".into()),
            ),
            Message::new("u".into(), Role::User, MessageContent::Text("q".into())),
        ];
        record_from_messages("sid-1", "openai", "m", &msgs, Some(&usage));
        record_from_messages("sid-1", "openai", "m", &msgs, Some(&usage));
        let path = dir.join("ct.jsonl");
        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2, "两次请求两行");
        let v: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(v["cache_hit_tokens"], 80, "cache_hit 落盘");
        assert_eq!(v["system_hash"], v["system_hash"]);
        assert!(v["system_hash"].as_str().unwrap().len() == 12, "12 位截断");
        std::env::remove_var("HEARTH_CACHE_TELEMETRY");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 负面：未开 env 时 record 零行为（零开销承诺）。
    #[test]
    fn test_record_noop_without_env() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("HEARTH_CACHE_TELEMETRY");
        // 不 panic 即可（无文件副作用）
        let msgs = vec![Message::new(
            "u".into(),
            Role::User,
            MessageContent::Text("x".into()),
        )];
        record_from_messages("s", "p", "m", &msgs, None);
    }
}

#[cfg(test)]
mod closure_tests {
    use super::*;
    use agent_types::{Message, MessageContent, Role};

    fn msg(role: Role, text: &str) -> Message {
        Message::new("m".into(), role, MessageContent::Text(text.to_string()))
    }

    /// Closure-1 最小证明（顶层复核 §1）: 同输入 → 同 chain（确定性）；
    /// 追加尾部消息 → 前缀 chain 值完全相同（stable prefix 语义）；
    /// system 消息可由 role 在链中定位。
    #[test]
    fn test_message_chain_prefix_stability() {
        let base = vec![msg(Role::System, "stable system"), msg(Role::User, "goal")];
        let c1 = message_chain(&base);
        // 确定性
        assert_eq!(c1, message_chain(&base), "同输入 chain 必须确定");
        // 尾部追加（L4 动态）→ 前缀不变
        let mut extended = base.clone();
        extended.push(msg(Role::System, "[System Context / Past Experience]..."));
        let c2 = message_chain(&extended);
        assert_eq!(c1.len(), 2);
        assert_eq!(c2.len(), 3);
        assert_eq!(
            &c1[..],
            &c2[..2],
            "尾部追加不得改变已有消息的 chain 值（stable prefix）"
        );
        // system 可定位（role 在链输入中——篡改 role 必断链）
        let mut changed_role = base.clone();
        changed_role[0] = msg(Role::User, "stable system");
        let c3 = message_chain(&changed_role);
        assert_ne!(c1[0], c3[0], "role 变化必须改变 chain");
        assert_ne!(c1[1], c3[1], "链式传播——后续值也变");
    }

    /// Closure-2: tools hash——同 schema 稳定 / 变化可检（只观测）。
    #[test]
    fn test_tools_hash_stable() {
        let t1 = vec![crate::ToolSchema {
            name: "bash".into(),
            description: "run cmd".into(),
            parameters: serde_json::json!({"type": "object"}),
        }];
        let h1 = compute_tools_hash(&t1);
        assert_eq!(h1, compute_tools_hash(&t1), "同 schema hash 稳定");
        let mut t2 = t1.clone();
        t2[0].description = "changed".into();
        assert_ne!(h1, compute_tools_hash(&t2), "schema 变化必须可检");
        assert_eq!(h1.len(), 12);
    }
}
