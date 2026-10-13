//! D-208（2026-10-08, traecode）源码级门禁：`circuit.rs` 关于"停机"的**声称必须与事实一致**。
//!
//! **盲区（自报）**：仅断言源码文本（逐行剥 `//` 注释后）——不执行、不构造运行时会话。口径「宁可漏报不误报」。
//!
//! 病灶：`circuit.rs` 模块头称"**停机动作由执行方（service）执行**"。事实：`Observer::run_and_report`
//! 返回的 `Vec<CircuitBreak>` 在唯一调用点（`agent-runtime/src/session.rs`）**返回值被丢弃**
//! （`if let Err(e) = … { warn }`），全仓**无任何消费方**执行停机；breaks 仅随 `report.md/json`
//! 落盘。⇒ 声称的"停机"**未接线**（"声称≠实现/已设计未接线"族；不得靠注释许诺——同 D-72）。
//!
//! 处置（遵"已设计未接线不得擅自删除/接线"）：**如实化**文案 + 单独立卡登记接线债。

use std::fs;
use std::path::PathBuf;

fn code_only(s: &str) -> String {
    s.lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn circuit_break_stop_claim_is_honest() {
    // 同族两落点：`circuit.rs` 模块头 + `lib.rs` 的 `CircuitBreak` 结构体文档都曾声称停机由执行方执行。
    for file in ["src/circuit.rs", "src/lib.rs"] {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(file);
        let raw = fs::read_to_string(&p).unwrap_or_else(|e| panic!("读 {file} 失败: {e}"));
        let code = code_only(&raw);

        assert!(
            !raw.contains("停机动作由执行方（service）执行"),
            "{file}：D-208 不得声称「停机动作由执行方（service）执行」——实际 CircuitBreak 全仓零消费方（只随报告落盘）"
        );
        assert!(
            code.contains("无消费方") || raw.contains("无消费方") || raw.contains("未接线"),
            "{file}：D-208 须如实标注 CircuitBreak 当前**无消费方**（停机未接线，另立债），不得靠注释许诺"
        );
    }
}
