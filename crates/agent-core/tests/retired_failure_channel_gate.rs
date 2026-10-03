//! 门禁：**已退役的"失败分类 / 恢复策略"决策支持层不得半途复活**（D-114，2026-10-02, traecode）。
//!
//! 背景：`terminal.rs` 曾含一组纯函数（`classify_failure` / `failure_strategy` /
//! `strategy_suggestion` / `completion_readiness`）与两个 scratch 键
//! （`last_failure_class` / `last_recovery_strategy`）。它们的**唯一下游通道**
//! （R5-1 "scratch 中转注入块"）早于 R6-5 依"判定权归还范式"主动删除——失败事实
//! 改为直接内联于工具结果消息（`ERROR[class=…]`，class 取自调度层 `error_kind`
//! 的结构化投影）。此后这组纯函数**全仓零消费者**、两个 scratch 键**零生产者**，
//! 属"定义了但无人用"（同 D-57/D-111 族），已整体退役。
//!
//! 本门禁钉住该退役：源码里**不得**再出现这些标识符。若日后确要恢复该能力，必须
//! **先设计再接线**（生产者与消费者同批到位），并同步更新本门禁——以此强制一次
//! 显式决策，而不是让"读方还在、生产方已无"的半接线状态悄悄回来。
//!
//! D-139（2026-10-04, traecode）：**同一门禁追加三条"清理漏网残链"**——它们是
//! D-98/D-114 退役手术的遗漏（退役口径同族，均属"定义了但无人用"）：
//!   • `last_success`——`AgentLoop` 只写不读字段；D-98 删了 `GuardContext.last_success`
//!     却漏删 `AgentLoop` 侧同名字段（两个现役 guard 只读 `last_action`/`cost_ratio`）。
//!   • `approval_denied_flag`——`AgentLoop` 只写不读字段；其唯一消费端（telemetry
//!     投影）已随 D-114 退役 ⇒ 孤儿。
//!   • `reflect_fact_conflict`（含分类器 `classify_reflect_fact_conflict`）——scratch
//!     键**有读无写**：白名单/报告/store 三处读点仍在，全仓零 `set_scratch` 生产者，
//!     分类器从未落地（悬空注释头）。同 D-114 的"读方还在、生产方已无"半接线。
//!
//! 扫描规则：**逐行剥掉 `//` 行注释后再匹配**——这样"退役说明注释"里可以照常点名
//! 这些标识符（可读性），而真正的**代码**一旦复用它们即报红。
//!
//! 文件头自报盲区：① 只剥 `//` 行注释，不剥块注释/字符串（退役标识符预期只可能以
//! **代码**形态复活，故保守足够；若出现在字符串字面量里可能漏报）；② 只做文本级
//! 检查、不判断语义；③ 只扫 `src/`，不检查非 Rust 文件。

use std::path::Path;

/// 已退役标识符（出现即视为"半接线复活"）。
const RETIRED_IDENTIFIERS: &[&str] = &[
    "classify_failure",
    "failure_strategy",
    "strategy_suggestion",
    "completion_readiness",
    "CompletionReadiness",
    "FailureKind",
    "RecoveryStrategy",
    "last_failure_class",
    "last_recovery_strategy",
    // D-139：D-98/D-114 退役手术的漏网残链（只写不读字段 / 有读无写的 scratch 键）。
    "last_success",
    "approval_denied_flag",
    "reflect_fact_conflict",
    "classify_reflect_fact_conflict",
];

fn collect_rs(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read src dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

#[test]
fn retired_failure_channel_identifiers_are_absent() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_rs(&src, &mut files);
    assert!(!files.is_empty(), "必须扫到 agent-core/src 下的 .rs 文件");

    let mut hits = Vec::new();
    for file in &files {
        let text = std::fs::read_to_string(file).expect("read source");
        for (lineno, line) in text.lines().enumerate() {
            // 剥掉行注释（`///` 亦以 `//` 开头，一并剥离），只看代码部分。
            let code = line.split("//").next().unwrap_or("");
            for id in RETIRED_IDENTIFIERS {
                if code.contains(id) {
                    hits.push(format!("{}:{}: {id}", file.display(), lineno + 1));
                }
            }
        }
    }

    assert!(
        hits.is_empty(),
        "已退役的失败分类/决策支持层与清理漏网残链**不得复活**（D-114 / D-139）——检测到：\n{}\n\
         该层零消费者、其 scratch 下游通道已被 R6-5 主动删除（禁止复活旧 scratch 通道）；\
         D-139 追加的字段/键亦为零消费者残链。\
         若确要恢复，请先设计（生产者+消费者同批接线）并同步更新本门禁。",
        hits.join("\n")
    );
}
