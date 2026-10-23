"""稳定基线聚合的口径（未知 ≠ 0 / 分母 = 有效 run）。"""
import pathlib
import sys
import unittest

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

import stability  # noqa: E402


def rec(task, ok, steps=5, wall=10.0, tokens=None, fc=False, silent=0, calls=2):
    usage = {"total_tokens": tokens} if tokens is not None else None
    return {
        "task": task,
        "ok": ok,
        "steps": steps,
        "wall_s": wall,
        "usage": usage,
        "false_completion": fc,
        "claim": True,
        "tool_metrics": {"tool_calls": calls, "silent_errors": silent},
    }


class AggregateTest(unittest.TestCase):
    def test_flaky_task_reports_mean_and_sd(self):
        a = stability.aggregate(
            [
                rec("A", True),
                rec("A", True),
                rec("B", True),
                rec("B", False),
            ]
        )
        self.assertEqual(a["runs"], 4)
        self.assertEqual(a["passed"], 3)
        rates = {t["task"]: t["pass_rate"] for t in a["tasks"]}
        self.assertEqual(rates, {"A": 1.0, "B": 0.5})
        self.assertAlmostEqual(a["mean_task_pass_rate"], 0.75)
        self.assertAlmostEqual(a["sd_task_pass_rate"], 0.353553, places=5)

    def test_missing_tokens_stay_unknown(self):
        a = stability.aggregate([rec("A", True), rec("A", False)])
        self.assertIsNone(a["tasks"][0]["mean_tokens"], "tokens 全缺失 ⇒ 未知，不得按 0")

    def test_empty_denominator_is_none(self):
        a = stability.aggregate([])
        self.assertIsNone(a["pass_rate"])
        self.assertEqual(a["wilson_95"], [None, None])

    def test_false_completion_and_silent_counted(self):
        a = stability.aggregate(
            [rec("A", False, fc=True, silent=2), rec("A", True, silent=0)]
        )
        self.assertEqual(a["false_completions"], 1)
        self.assertEqual(a["silent_errors"], 2)
        self.assertEqual(a["tool_calls"], 4)


if __name__ == "__main__":
    unittest.main()
