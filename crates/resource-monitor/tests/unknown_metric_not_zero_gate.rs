//! D-206（2026-10-08, traecode）源码级门禁：`ResourceSnapshot` 的**未知指标必须为 `null`**，不得用 `0` 冒充。
//!
//! **盲区（自报）**：仅断言源码文本（逐行剥 `//` 注释后）的类型声明与兜底形态——不执行、不构造
//! "无根盘"环境（无法在本机伪造 `no root disk`）。口径「宁可漏报不误报」。
//!
//! 病灶：`snapshot()` 在找不到根盘（`/` 或 `C:\`）时返回 `(0.0, 0.0)` ⇒ 上报
//! `disk_free_gb=0, disk_total_gb=0`。**0 是合法测量值**（磁盘全满），消费方（本服务自身
//! 每小时遥测 `main.rs` 的 `"disk_gb"`、`GET /api/v1/resources`）会把"探测失败"读成
//! **"磁盘 0GB 可用"**、并把它计入告警/均值（业界口径：未知度量应为 `null` 并从聚合中排除，
//! 见 Azure Monitor 文档）。同类：进程内存取不到时 `.unwrap_or(0.0)`、`mem_total<=0` 时
//! `memory_percent=0.0`。与 D-46「未知价 → 显式不可用，绝不按 0 冒充」、D-176「无结构性 0
//! 假指标」同族。

use std::fs;
use std::path::PathBuf;

fn code_only(s: &str) -> String {
    s.lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn unknown_metrics_are_null_not_zero() {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    let code =
        code_only(&fs::read_to_string(&p).expect("crates/resource-monitor/src/lib.rs 必须存在"));

    for decl in [
        "pub cpu_percent: Option<f32>",
        "pub memory_mb: Option<f64>",
        "pub memory_percent: Option<f32>",
        "pub disk_free_gb: Option<f64>",
        "pub disk_total_gb: Option<f64>",
    ] {
        assert!(
            code.contains(decl),
            "D-206：未知度量须建模为 `Option`（`{decl}`）——`0` 是合法测量值，不能用来冒充\"未知\""
        );
    }
    assert!(
        !code.contains("(0.0, 0.0)"),
        "D-206：找不到根盘时不得回落到 `(0.0, 0.0)`（读成\"磁盘 0GB 可用/全满\"）——应为 `None`"
    );
    assert!(
        !code.contains(".unwrap_or(0.0)"),
        "D-206：进程内存取不到时不得 `.unwrap_or(0.0)`（读成\"占用 0MB\"）——应为 `None`"
    );
}
