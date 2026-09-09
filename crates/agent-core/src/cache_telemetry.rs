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

use llm_gateway::Usage;

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
        .map(|m| short_hash(serde_json::to_string(&m.content).unwrap_or_default().as_bytes()))
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

/// 便捷入口：从 messages 生成 hash 并记录（loop 层 chat 成功后调用）。
pub fn record_from_messages(
    session_id: &str,
    provider: &str,
    model: &str,
    messages: &[agent_types::Message],
    usage: Option<&Usage>,
) {
    if std::env::var("HEARTH_CACHE_TELEMETRY").is_err() {
        return;
    }
    let (system_hash, prefix_hash, full_hash) = hashes_for(messages);
    record(CacheTelemetryRecord {
        session_id,
        provider,
        model,
        usage,
        msgs_count: messages.len(),
        system_hash: &system_hash,
        prefix_hash: &prefix_hash,
        full_hash: &full_hash,
    });
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
            Message::new("u0".into(), Role::User, MessageContent::Text("hello".into())),
            Message::new("u1".into(), Role::User, MessageContent::Text("world".into())),
        ];
        let (s1, p1, f1) = hashes_for(&m1);
        // 稳定性：同输入重算 hash 一致（stable prefix 前提——序列化字节确定）
        let (s2, p2, f2) = hashes_for(&m1);
        assert_eq!((s1.clone(), p1.clone(), f1.clone()), (s2, p2, f2));
        // 敏感性：仅末条内容变化 → full_hash 变、prefix_hash 不变
        // （首条 clone 复用——运行时历史对象复用语义）
        let m2 = vec![m1[0].clone(), Message::new("u1".into(), Role::User, MessageContent::Text("WORLD".into()))];
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
            Message::new("sys".into(), Role::System, MessageContent::Text("sys prompt".into())),
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
        let msgs = vec![Message::new("u".into(), Role::User, MessageContent::Text("x".into()))];
        record_from_messages("s", "p", "m", &msgs, None);
    }
}
