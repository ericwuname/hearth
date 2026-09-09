//! project-xray: 顶层透视工具（v13 S1）。
//!
//! 两个能力（v13 范围，graph/report/diff 延至 v14）：
//! - `facts`: 扫描 workspace 产出 facts.json（crate 数 / .rs 数 / LOC / 测试声明数实算，
//!   替代守门员手算）。
//! - `wiring`: 接线断言引擎——按声明式规格（TOML）检查"宣称的能力在源码里真的接着"，
//!   链上任一环零命中 = 断裂，severity=red 的断裂使进程 exit 1（CI 第四门）。
//!
//! 设计出处: docs/project-xray-design.md Part C；v13 定版: docs/forge-v13-plan.md §2 线 A。

pub mod facts;
pub mod wiring;
