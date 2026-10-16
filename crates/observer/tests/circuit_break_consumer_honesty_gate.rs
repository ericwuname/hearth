//! D-208（2026-10-08, traecode）源码级门禁：`circuit`/`lib` 关于"停机接线"的**声称必须与事实一致**。
//!
//! **盲区（自报）**：仅断言源码文本（逐行剥 `//` 注释后）——不执行。口径「宁可漏报不误报」。
//!
//! 沿革：本门禁此前断言"如实标注**无消费方/未接线**"（当时属实：`run_and_report` 的返回值
//! 在唯一调用点被丢弃）。D-208 同轮**已接线**（执行方侧 `red_line_terminal_phase` fail-closed
//! 置终态）⇒ 门禁随之改为断言**接线后的事实**，防止文案回退成"未接线"或虚报"评估失败也停机"。
//!
//! 行为级门禁见 `crates/service/tests/circuit_break_fail_closed_gate.rs`。

use std::fs;
use std::path::PathBuf;

fn raw_of(file: &str) -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(file);
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("读 {file} 失败: {e}"))
}

#[test]
fn circuit_break_wiring_claim_matches_reality() {
    // 同族两落点：`circuit.rs` 模块头 + `lib.rs` 的 `CircuitBreak` 结构体文档。
    for file in ["src/circuit.rs", "src/lib.rs"] {
        let raw = raw_of(file);

        // ① 不得回退成"未接线/无消费方"（D-208 已接线）。
        assert!(
            !raw.contains("无消费方") && !raw.contains("未接线"),
            "{file}：D-208 已接线，文案不得回退成「无消费方/未接线」"
        );
        // ② 须点名**消费点**与 fail-closed 语义（不靠注释许诺，靠事实）。
        assert!(
            raw.contains("red_line_terminal_phase") && raw.contains("blocked"),
            "{file}：D-208 须写明消费点（`red_line_terminal_phase`）与 fail-closed 终态（`blocked`）"
        );
        // ③ 须写明**边界**：Observer 评估失败恒 fail-open（D-45）——不得让人以为审计失败也停机。
        assert!(
            raw.contains("fail-open"),
            "{file}：D-208 须写明边界「Observer 评估失败 = fail-open」（D-45）"
        );
    }
}
