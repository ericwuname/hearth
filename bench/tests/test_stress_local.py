#!/usr/bin/env python3
"""bench/stress_local.py 的读数单测（M1-4）。

守的不变式：
  · **跳过 ≠ 通过**：`skipped` 的轮次不进分母，且全部跳过时比率是 `None`（未知 ≠ 0）；
  · **三类故障分开计数**（服务死亡 / 挂起 / panic）——合并成一个数会掩盖处置方式；
  · 任一故障都让"不可用率"上升（量具必须能报坏消息）。
"""
import pathlib
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))

import stress_local  # noqa: E402


def rec(**kw):
    base = {
        "scenario": "ST-x",
        "run": 0,
        "ok": True,
        "skipped": False,
        "no_panic": True,
        "service_alive": True,
        "detail": "phase=done",
    }
    base.update(kw)
    return base


class TestStressSummary(unittest.TestCase):
    def test_healthy_run_is_zero_unavailable(self):
        s = stress_local.summarize([rec(), rec(), rec()])
        self.assertEqual(3, s["runs"])
        self.assertEqual(0.0, s["unavailable_rate"])
        self.assertEqual(0, s["crashed"] + s["hung"] + s["panicked"])

    def test_crash_raises_rate(self):
        s = stress_local.summarize([rec(), rec(service_alive=False)])
        self.assertEqual(1, s["crashed"])
        self.assertAlmostEqual(0.5, s["unavailable_rate"])

    def test_hang_raises_rate(self):
        s = stress_local.summarize([rec(), rec(detail="TIMEOUT(last=working)")])
        self.assertEqual(1, s["hung"])
        self.assertAlmostEqual(0.5, s["unavailable_rate"])

    def test_panic_raises_rate(self):
        s = stress_local.summarize([rec(no_panic=False)])
        self.assertEqual(1, s["panicked"])
        self.assertEqual(1.0, s["unavailable_rate"])

    def test_one_record_counted_once_even_if_multiple_faults(self):
        """同一轮同时"死了 + 有 panic"只算一次不可用（否则比率会超过 100%）。"""
        s = stress_local.summarize([rec(service_alive=False, no_panic=False)])
        self.assertEqual(1, s["runs"])
        self.assertEqual(1.0, s["unavailable_rate"])

    def test_skipped_is_excluded_from_denominator(self):
        s = stress_local.summarize([rec(skipped=True, ok=None, detail="skipped: 无 tmpfs"), rec()])
        self.assertEqual(1, s["runs"])
        self.assertEqual(1, s["skipped"])
        self.assertEqual(0.0, s["unavailable_rate"])

    def test_all_skipped_is_unknown_not_zero(self):
        s = stress_local.summarize([rec(skipped=True, ok=None), rec(skipped=True, ok=None)])
        self.assertEqual(0, s["runs"])
        self.assertIsNone(s["unavailable_rate"], "没有任何轮次参与 ⇒ 未知，不得按 0 冒充")

    def test_behavior_failure_alone_does_not_count_as_unavailable(self):
        """行为判失败（如审批没触发）≠ 崩溃/挂起——两者分开报，便于归因。"""
        s = stress_local.summarize([rec(ok=False)])
        self.assertEqual(1, s["behavior_failed"])
        self.assertEqual(1, s["runs"])
        self.assertEqual(0.0, s["unavailable_rate"])


if __name__ == "__main__":
    unittest.main()
