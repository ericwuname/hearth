//! D-196（2026-10-08, traecode）源码级门禁：**恢复路径不得静默丢弃约束/验收标准**。
//!
//! **盲区（自报）**：仅断言**源码文本**（逐行剥 `//` 注释后）含指定构造——不执行、不验证
//! 运行时行为。之所以用源码级：解析点内联在 `lib.rs` 的 `run` 恢复分支里，修复前**无**可由
//! 外部测试直接驱动的**纯函数**（行为侧构造需整套 CLI/agent 恢复 harness）；口径「宁可漏报不误报」。
//!
//! 病灶：resume 时用 `tg.get("constraints").and_then(|v| serde_json::from_value(v.clone()).ok())
//! .unwrap_or_default()` —— 形状不符（旧版本 / 部分写 / 损坏）时**静默置空**，而用户仍被
//! 告知"任务目标已恢复" ⇒ 任务在**失去原约束/验收标准**下继续（D-164/D-177/D-184 同族）。
//! 修复：抽出**纯函数** `parse_taskgoal_lists`（形状不符 → `Err` 供调用方并入 stale 留痕）。

use std::fs;
use std::path::PathBuf;

fn code_only(s: &str) -> String {
    s.lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn resume_parses_taskgoal_lists_without_silent_drop() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src = fs::read_to_string(root.join("src/lib.rs")).expect("read src/lib.rs");
    let code = code_only(&src);
    assert!(
        code.contains("fn parse_taskgoal_lists("),
        "D-196：恢复路径必须经**纯函数** `parse_taskgoal_lists` 解析 constraints/acceptance_criteria\
         （形状不符须返回 Err 供留痕，不得静默置空）"
    );
    assert!(
        code.contains("parse_taskgoal_lists(")
            && code.matches("parse_taskgoal_lists(").count() >= 2,
        "D-196：`parse_taskgoal_lists` 必须被**调用**（定义 + 至少一处调用），否则是零调用方死物"
    );
}
