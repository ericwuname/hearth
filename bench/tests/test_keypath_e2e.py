#!/usr/bin/env python3
"""M1-6 关键路径 E2E 清单门禁：6 条红线路径各自**必须**有一个还在场的 E2E 断言。

为什么需要清单门禁而不是再写一套 E2E：这 6 条路径的 E2E **已经存在**（散落在各 crate 的
`tests/` 与 `src/**/tests` 里），真正的风险不是"没写"，而是**它们悄悄消失或改名**——
门禁文件被合并、断言被注释掉、函数名被重命名，测试仍全绿，覆盖却没了。
本门禁用「文件在场 + 锚点字符串在场」把这条防线钉住：任一条消失即 FAIL。

清单（路径 → 门禁文件 → 锚点）：
  1. 红线 `blocked`（D-208）        service/tests/circuit_break_fail_closed_gate.rs
  2. observer ④ 资源指标（D-219）    service/tests/observer_resource_metrics_wired_gate.rs
  3. 简化步 `Simplify`（D-214）      agent-core/tests/simplify_override_wired_gate.rs
  4. `resume`（重启后会话可恢复）     service/tests/integration_test.rs
  5. 审批闸（不静默执行破坏性命令）   bench/stress_local.py（运行时 E2E，ST8 场景）
  6. 非 UTF-8 子进程输出（D-155）    sandbox/src/lib.rs

**关于第 5 条**：审批路径目前**没有 Rust 集成测试**（`submit_approval` 只被 API 形状测试
覆盖），真正验证"审批会拦住执行"的是压力场 ST8——它跑真 service、真 SSE、真调度器超时，
比脚本化 mock 更接近生产。缺点是耗时（≈60s/轮）且依赖本地二进制，所以它留在
`stress_local.py` 里由 M1-4 常态跑，而不是塞进 `cargo test`。这是**如实登记的选择**，
不是"用别的东西冒充 E2E"。

跑法：`python -m unittest discover -s bench/tests -p "test_*.py"`
"""
import pathlib
import unittest

REPO = pathlib.Path(__file__).resolve().parents[2]

# (路径名, 文件, 必须在文件里出现的锚点)
KEY_PATHS = [
    (
        "红线 blocked（D-208）",
        "crates/service/tests/circuit_break_fail_closed_gate.rs",
        "sandbox_red_line_fail_closed_sets_blocked_terminal_phase",
    ),
    (
        "observer ④ 资源指标（D-219）",
        "crates/service/tests/observer_resource_metrics_wired_gate.rs",
        "observer_resource_metrics_are_fed_by_real_sessions",
    ),
    (
        "简化步 Simplify（D-214）",
        "crates/agent-core/tests/simplify_override_wired_gate.rs",
        "simplify_override_really_disables_tools_and_lowers_budget",
    ),
    (
        "resume（重启后可恢复）",
        "crates/service/tests/integration_test.rs",
        "test_p5_session_survives_restart",
    ),
    (
        "审批闸（不静默执行破坏性命令）",
        "bench/stress_local.py",
        "def st8_approval_gate(ctx, p):",
    ),
    (
        "非 UTF-8 子进程输出（D-155）",
        "crates/sandbox/src/lib.rs",
        "test_d155_non_utf8_child_output_is_explicitly_marked",
    ),
]


class TestKeyPathE2EManifest(unittest.TestCase):
    def test_every_key_path_has_a_live_e2e_gate(self):
        missing = []
        for label, rel, anchor in KEY_PATHS:
            p = REPO / rel
            if not p.is_file():
                missing.append(f"{label}: 门禁文件不存在 {rel}")
                continue
            if anchor not in p.read_text(encoding="utf-8", errors="replace"):
                missing.append(f"{label}: {rel} 里找不到锚点 `{anchor}`")
        self.assertEqual([], missing, "关键路径 E2E 覆盖被削弱：\n" + "\n".join(missing))

    def test_manifest_covers_the_six_paths_the_taskbook_names(self):
        """任务书 M1-6 列了 6 条；清单少一条就等于静默摊薄（D-226 同口径）。"""
        self.assertEqual(6, len(KEY_PATHS), "M1-6 要求 6 条关键路径各一条 E2E")


if __name__ == "__main__":
    unittest.main()
