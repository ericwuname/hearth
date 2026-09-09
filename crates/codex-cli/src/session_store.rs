//! X1-1/X1-4 (v0.1.6): 会话持久化——完整历史 Turn 追加落盘 JSONL，
//! resume 从磁盘重建上下文（对齐 codex Thread 持久化；治"窗口一废全废"）。
//!
//! 位置：`~/.config/hearth/sessions/<session_id>.jsonl`（HEARTH_SESSIONS_DIR 可覆盖）。
//! 格式：JSONL，每行一个完整 Turn（agent_types::Turn 序列化——含全部消息）。
//! 与 transcript.rs 分工：transcript=单 run 摘要（一行）；本模块=完整历史（可恢复）。

use agent_types::Turn;
use anyhow::{Context, Result};
use std::path::PathBuf;

/// 会话目录：HEARTH_SESSIONS_DIR > $HOME/.config/hearth/sessions > ./.hearth_sessions。
pub fn sessions_dir() -> PathBuf {
    if let Some(d) = std::env::var_os("HEARTH_SESSIONS_DIR") {
        return PathBuf::from(d);
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".config/hearth/sessions");
    }
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return PathBuf::from(appdata).join("hearth/sessions");
    }
    PathBuf::from(".hearth_sessions")
}

/// 追加一个 Turn 到会话文件（每轮结束调用一次；幂等追加，重开不丢）。
pub fn save_turn(session_id: &str, turn: &Turn) -> Result<()> {
    let dir = sessions_dir();
    std::fs::create_dir_all(&dir).context("create sessions dir")?;
    let path = dir.join(format!("{session_id}.jsonl"));
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .context("open session file")?;
    let line = serde_json::to_string(turn).context("serialize turn")?;
    writeln!(f, "{line}").context("write turn")?;
    Ok(())
}

/// 全量快照写——把完整历史 Turn 列表原子重写（tmp + rename，防半写损坏）。
/// 连续对话历史累积，追加会重复，故每轮结束用快照覆盖。
pub fn save_snapshot(session_id: &str, turns: &[Turn]) -> Result<()> {
    let dir = sessions_dir();
    std::fs::create_dir_all(&dir).context("create sessions dir")?;
    let path = dir.join(format!("{session_id}.jsonl"));
    let tmp = dir.join(format!("{session_id}.jsonl.tmp"));
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&tmp).context("create tmp session file")?;
        for t in turns {
            let line = serde_json::to_string(t).context("serialize turn")?;
            writeln!(f, "{line}").context("write turn")?;
        }
        f.sync_all().ok();
    }
    std::fs::rename(&tmp, &path).context("rename session file")?;
    Ok(())
}

/// 任务状态持久化 (v0.2): task_graph 单独落盘（<sid>.graph.json，原子写）——
/// 甘特图反复横跳的根：resume 时恢复任务图，不重新 decompose（agent 知道自己
/// 已写到哪步、哪些节点 Completed，不再 replan 回旧方案）。
pub fn save_graph(session_id: &str, graph: &serde_json::Value) -> Result<()> {
    save_graph_with_revision(session_id, graph, 0)
}

/// R2-D (批示 2): 带 state_revision 的 graph 落盘——与 taskgoal.json 同值，
/// resume 时校验一致性（crash 落在两次写盘之间的分叉检测）。
pub fn save_graph_with_revision(
    session_id: &str,
    graph: &serde_json::Value,
    state_revision: u64,
) -> Result<()> {
    let dir = sessions_dir();
    std::fs::create_dir_all(&dir).context("create sessions dir")?;
    let path = dir.join(format!("{session_id}.graph.json"));
    let tmp = dir.join(format!("{session_id}.graph.json.tmp"));
    let wrapped = serde_json::json!({ "state_revision": state_revision, "graph": graph });
    std::fs::write(&tmp, serde_json::to_string_pretty(&wrapped)?).context("write graph tmp")?;
    std::fs::rename(&tmp, &path).context("rename graph file")?;
    Ok(())
}

/// 读取任务图（无则返回 Null——resume 时无图可恢复）。
/// R2-D (v0.2.7): 兼容新旧两格式——新版带 state_revision 包装
/// `{"state_revision": N, "graph": ...}`；旧版裸 graph（revision 视为 0）。
pub fn load_graph(session_id: &str) -> serde_json::Value {
    let wrapped = load_graph_with_revision(session_id);
    wrapped.map(|(_, g)| g).unwrap_or(serde_json::Value::Null)
}

/// R2-D (批示 2): 读 graph + 其 state_revision（一致性校验用）。
/// 返回 None = 文件不存在/损坏。
pub fn load_graph_with_revision(session_id: &str) -> Option<(u64, serde_json::Value)> {
    let path = sessions_dir().join(format!("{session_id}.graph.json"));
    let s = std::fs::read_to_string(&path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&s).ok()?;
    if let Some(rev) = v.get("state_revision").and_then(|r| r.as_u64()) {
        Some((
            rev,
            v.get("graph").cloned().unwrap_or(serde_json::Value::Null),
        ))
    } else {
        Some((0, v)) // 旧格式：裸 graph
    }
}

/// R2-D (批示 2, v0.2.7): TaskGoal 持久化——taskgoal.json 载体（非第四套任务
/// 模型，批示 8）：original_goal/constraints/criteria/revision + state_revision
/// （与 graph.json 同值——一致性校验锚点）。原子写。
pub fn save_taskgoal(
    session_id: &str,
    taskgoal: &serde_json::Value,
    state_revision: u64,
) -> Result<()> {
    let dir = sessions_dir();
    std::fs::create_dir_all(&dir).context("create sessions dir")?;
    let path = dir.join(format!("{session_id}.taskgoal.json"));
    let tmp = dir.join(format!("{session_id}.taskgoal.json.tmp"));
    let wrapped = serde_json::json!({ "state_revision": state_revision, "taskgoal": taskgoal });
    std::fs::write(&tmp, serde_json::to_string_pretty(&wrapped)?).context("write taskgoal tmp")?;
    std::fs::rename(&tmp, &path).context("rename taskgoal file")?;
    Ok(())
}

/// R2-D: 读 taskgoal + state_revision。None = 文件不存在（首会话）。
pub fn load_taskgoal(session_id: &str) -> Option<(u64, serde_json::Value)> {
    let path = sessions_dir().join(format!("{session_id}.taskgoal.json"));
    let s = std::fs::read_to_string(&path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&s).ok()?;
    let rev = v
        .get("state_revision")
        .and_then(|r| r.as_u64())
        .unwrap_or(0);
    Some((
        rev,
        v.get("taskgoal")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    ))
}

/// 读取会话全部 Turn（按落盘顺序）。文件不存在/损坏行 → 跳过（不 panic）。
pub fn load_turns(session_id: &str) -> Vec<Turn> {
    let path = sessions_dir().join(format!("{session_id}.jsonl"));
    let Ok(content) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    content
        .lines()
        .filter_map(|l| serde_json::from_str::<Turn>(l).ok())
        .collect()
}

/// 列出本地会话（resume 提示用）。
pub fn list_sessions() -> Vec<(String, usize)> {
    let dir = sessions_dir();
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    rd.filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "jsonl").unwrap_or(false))
        .map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let sid = name.trim_end_matches(".jsonl").to_string();
            let turns = load_turns(&sid);
            (sid, turns.len())
        })
        .collect()
}

#[cfg(test)]
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

    // 环境变量 HEARTH_SESSIONS_DIR 是进程全局——并行测试会互相覆盖 env 致
    // 读回 0（v0.2.2 暴露：加 talent 后测试线程顺序变化）。串行化 env 竞争测试。
    #[test]
    fn test_roundtrip_turn() {
        let _g = super::ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join(format!("hearth_ss_test_{}", uuid::Uuid::new_v4()));
        std::env::set_var("HEARTH_SESSIONS_DIR", &dir);
        let mut turn = Turn::new(0);
        turn.messages.push(agent_types::Message::new(
            "u0".into(),
            agent_types::Role::User,
            agent_types::MessageContent::Text("写一个贪吃蛇".into()),
        ));
        save_turn("test-sid", &turn).unwrap();
        let loaded = load_turns("test-sid");
        assert_eq!(loaded.len(), 1, "落盘后应能读回 1 个 Turn");
        assert_eq!(loaded[0].messages.len(), 1);
        assert!(
            format!("{:?}", loaded[0].messages[0].content).contains("贪吃蛇"),
            "消息内容应完整恢复"
        );
        // 不存在的 session → 空（不 panic）
        assert!(load_turns("nope").is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_append_multiple_turns() {
        let _g = super::ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join(format!("hearth_ss_test_{}", uuid::Uuid::new_v4()));
        std::env::set_var("HEARTH_SESSIONS_DIR", &dir);
        for i in 0..3 {
            save_turn("multi", &Turn::new(i)).unwrap();
        }
        let loaded = load_turns("multi");
        assert_eq!(loaded.len(), 3, "追加 3 轮应读回 3 个 Turn");
        assert_eq!(loaded[2].index, 2, "顺序应保持");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
#[cfg(test)]
mod r2d_tests {
    use super::*;

    /// T2 (批示 2 + 补充 3, R2-D): crash between persistence——revision 不一致
    /// 可检测（recovery path 数据层前提）；旧格式裸 graph 兼容（rev=0）。
    /// env 竞争：HEARTH_SESSIONS_DIR 是进程全局——与既有测试共用 ENV_LOCK 串行。
    #[test]
    fn test_t2_revision_mismatch_detectable() {
        let _g = super::ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = std::env::temp_dir().join(format!("hearth_ss_t2_{}", uuid::Uuid::new_v4()));
        std::env::set_var("HEARTH_SESSIONS_DIR", &dir);
        let sid = format!("t2-{}", uuid::Uuid::new_v4());
        save_taskgoal(&sid, &serde_json::json!({"original_goal": "goal A"}), 4).unwrap();
        save_graph_with_revision(&sid, &serde_json::json!({"nodes": []}), 4).unwrap();
        let (tg_rev, _) = load_taskgoal(&sid).unwrap();
        let (g_rev, _) = load_graph_with_revision(&sid).unwrap();
        assert_eq!(tg_rev, g_rev, "正常路径 revision 一致");
        save_graph_with_revision(&sid, &serde_json::json!({"nodes": []}), 5).unwrap();
        let (tg_rev, _) = load_taskgoal(&sid).unwrap();
        let (g_rev, _) = load_graph_with_revision(&sid).unwrap();
        assert_ne!(tg_rev, g_rev, "crash 场景 revision 不一致必须可检测");
        let sid2 = format!("t2-old-{}", uuid::Uuid::new_v4());
        let dir = sessions_dir();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(format!("{sid2}.graph.json")),
            serde_json::json!({"nodes": []}).to_string(),
        )
        .unwrap();
        let (rev, _) = load_graph_with_revision(&sid2).unwrap();
        assert_eq!(rev, 0, "旧格式裸 graph 视为 rev 0");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
