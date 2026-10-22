#!/usr/bin/env python3
"""bench/scorecard.py 的口径单测（M1-5 每解成本 / M1-3 静默失败列）。

守的核心不变式是**未知 ≠ 0**：
  · 未定价解（`cost_usd is None`）不得计入分母、不得被算成 $0；
  · 没有任何一条已定价成功解时，"每解成本"必须是**未知**，而不是 `$0.0000`；
  · 有已定价解时，未定价解要**单列**（不静默丢弃）。
"""
import pathlib
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))

import scorecard  # noqa: E402


def rec(ok=True, cost=None, **kw):
    d = {"task": "T", "external_ok": ok, "cost_usd": cost, "claim_ok": True}
    d.update(kw)
    return d


class TestCostPerSolve(unittest.TestCase):
    def test_all_unpriced_is_unknown_not_zero(self):
        c = scorecard._cost_breakdown([rec(True, None), rec(True, None)])
        self.assertEqual(2, c["unpriced_solved"])
        self.assertEqual(0, c["priced_solved"])
        self.assertIsNone(c["cost_per_solve"], "未定价 ⇒ 未知，绝不按 0 冒充")
        self.assertIsNone(c["total_cost_usd"])

    def test_priced_denominator_excludes_unpriced(self):
        c = scorecard._cost_breakdown([rec(True, 0.30), rec(True, 0.10), rec(True, None)])
        self.assertEqual(2, c["priced_solved"])
        self.assertEqual(1, c["unpriced_solved"], "未定价解必须单列")
        self.assertAlmostEqual(0.2, c["cost_per_solve"], msg="0.40 / 2 条已定价解")

    def test_failed_runs_do_not_enter_denominator(self):
        """每解成本的分母是**成功解**——失败会话的成本不摊到成功解上。"""
        c = scorecard._cost_breakdown([rec(True, 0.2), rec(False, 0.9)])
        self.assertEqual(1, c["priced_solved"])
        self.assertAlmostEqual(0.2, c["cost_per_solve"])

    def test_zero_cost_is_a_real_value_not_unknown(self):
        """`0.0` 是**有效**成本（真的免费），必须与 `None`（未知）区分开。"""
        c = scorecard._cost_breakdown([rec(True, 0.0)])
        self.assertEqual(1, c["priced_solved"])
        self.assertEqual(0.0, c["cost_per_solve"])
        self.assertIsNotNone(c["cost_per_solve"])

    def test_cell_text_is_honest(self):
        self.assertIn("未知", scorecard._cost_cell(scorecard._cost_breakdown([rec(True, None)])))
        priced = scorecard._cost_cell(scorecard._cost_breakdown([rec(True, 0.5), rec(True, None)]))
        self.assertIn("未定价 1 条单列", priced)


class TestSilentColumn(unittest.TestCase):
    def test_silent_rate_unknown_without_tool_calls(self):
        row = scorecard.summarize("s", [rec(True, None)])
        self.assertIn("未知", row["silent_rate"])

    def test_silent_rate_reported_when_metrics_present(self):
        row = scorecard.summarize(
            "s",
            [rec(True, None, tool_metrics={"tool_calls": 8, "tool_errors": 2, "error_events": 0, "silent_errors": 2})],
        )
        self.assertIn("25.0%", row["silent_rate"])
        self.assertIn("2/8", row["silent_rate"])


if __name__ == "__main__":
    unittest.main()
