#!/usr/bin/env python3
"""bench/judge.py 的单元测试（M0 / D-222）。

RED 基线：`test_pass_requires_zero_rc` 断言「输出含 VERIFY_PASS 但退出码非 0 ⇒ 判失败」。
修复前 `judge_verify` 只看子串 ⇒ 该断言失败（这正是要把判据口径钉死的那条）。
"""
import pathlib
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))

from judge import (  # noqa: E402
    classify_run,
    judge_verify,
    split_rc,
    summarize_runs,
    summarize_tool_metrics,
)


class TestClassifyRun(unittest.TestCase):
    """假完成率的两块拼图：**agent 自报的结论** × 外部判据（M1 / D-229，口径 D-230 订正）。

    注意 `claim` 是 agent 自己的判断（`done.report.ok`），**不是** HTTP `phase`：
    `phase=="done"` 只表示循环终止，不表示它声称做成了。
    """

    def test_false_completion_is_claim_yes_plus_external_fail(self):
        c = classify_run(True, False)
        self.assertTrue(c["claim_ok"])
        self.assertFalse(c["external_ok"])
        self.assertTrue(c["false_completion"], "自称做成 + 外部判失败 = 假完成")

    def test_claim_yes_and_pass_is_not_false_completion(self):
        c = classify_run(True, True)
        self.assertFalse(c["false_completion"])
        self.assertFalse(c["verdict_conflict"])

    def test_claim_no_plus_pass_is_verdict_conflict(self):
        """D-230 的核心场景：agent 自报没做完（如预算耗尽），但外部判据说成了。

        这正是 31 条录制语料里的 30 条。旧口径（用 `phase`）会把它们**算进分母**
        并**吞掉**这条反向信息。
        """
        c = classify_run(False, True)
        self.assertFalse(c["claim_ok"])
        self.assertTrue(c["verdict_conflict"])
        self.assertFalse(c["false_completion"])

    def test_claim_no_and_fail_is_consistent(self):
        c = classify_run(False, False)
        self.assertFalse(c["false_completion"])
        self.assertFalse(c["verdict_conflict"])

    def test_unknown_claim_is_not_claimed(self):
        c = classify_run(None, True)
        self.assertFalse(c["claim_ok"])
        self.assertTrue(c["unknown_claim"])
        self.assertFalse(c["claim_known"])
        self.assertTrue(c["verdict_conflict"] is False, "未知不得当作'自称没做成'")


class TestSummarizeRuns(unittest.TestCase):
    def _rec(self, claim, ok):
        return dict(classify_run(claim, ok), **{"task": "T"})

    def test_rate_denominator_is_claimed_count(self):
        recs = [self._rec(True, False), self._rec(True, True), self._rec(False, True)]
        s = summarize_runs(recs)
        self.assertEqual(3, s["runs"])
        self.assertEqual(2, s["claim_ok"], "分母只数'自称做成'的会话")
        self.assertEqual(1, s["false_completion"])
        self.assertAlmostEqual(0.5, s["false_completion_rate"])
        self.assertEqual(1, s["verdict_conflict"])

    def test_unknown_rate_is_none_not_zero(self):
        """一条都没自称做成 ⇒ 比率是**未知**（None），不是 0.0（同 D-176/D-219 口径）。"""
        s = summarize_runs([self._rec(False, False)])
        self.assertEqual(0, s["claim_ok"])
        self.assertIsNone(s["false_completion_rate"], "未知不得按 0 冒充")

    def test_unknown_claim_counted(self):
        s = summarize_runs([self._rec(None, True)])
        self.assertEqual(1, s["unknown_claim"])
        self.assertEqual(0, s["claim_ok"])

    def test_empty_input(self):
        s = summarize_runs([])
        self.assertEqual(0, s["runs"])
        self.assertIsNone(s["false_completion_rate"])


class TestSummarizeToolMetrics(unittest.TestCase):
    """M1-3 静默失败率：失败发生了、但用户**看不到原因**（空错误 / 结果丢失）。

    口径订正记录：初版把"没有 error 事件"当静默，是错的——CLI 拿 `tool_result.is_error`
    画 ✗（D-155/W4），工具失败本来就有用户可见通道；现在的静默只数
    `empty_errors`（失败但无输出）+ `lost_results`（调用没有结果）两类。
    """

    def test_silent_is_empty_errors_plus_lost_results(self):
        s = summarize_tool_metrics(
            [
                {"tool_calls": 10, "tool_errors": 2, "error_events": 0,
                 "empty_errors": 2, "lost_results": 0, "silent_errors": 2},
                {"tool_calls": 10, "tool_errors": 3, "error_events": 1,
                 "empty_errors": 0, "lost_results": 1, "silent_errors": 1},
            ]
        )
        self.assertEqual(20, s["tool_calls"])
        self.assertEqual(5, s["tool_errors"])
        self.assertEqual(3, s["silent_errors"])
        self.assertAlmostEqual(0.15, s["silent_error_rate"], msg="3/20")

    def test_visible_tool_errors_are_not_silent(self):
        """工具失败但**有输出**（用户能看到原因）⇒ 不算静默。"""
        s = summarize_tool_metrics(
            [{"tool_calls": 5, "tool_errors": 5, "error_events": 0,
              "empty_errors": 0, "lost_results": 0, "silent_errors": 0}]
        )
        self.assertEqual(0, s["silent_errors"])
        self.assertEqual(0.0, s["silent_error_rate"])

    def test_no_tool_calls_is_unknown_not_zero(self):
        """没有任何工具调用 ⇒ 分母 0 ⇒ **未知**（None），不是 0.0（同 D-176/D-219）。"""
        s = summarize_tool_metrics(
            [{"tool_calls": 0, "tool_errors": 0, "error_events": 0,
              "empty_errors": 0, "lost_results": 0, "silent_errors": 0}]
        )
        self.assertIsNone(s["silent_error_rate"])

    def test_missing_metric_rows_are_excluded(self):
        """旧结果文件没有 `tool_metrics` 列 ⇒ 该条**不参与**，不得当 0 失败灌进分母。"""
        s = summarize_tool_metrics(
            [None, {},
             {"tool_calls": 4, "tool_errors": 1, "error_events": 0,
              "empty_errors": 1, "lost_results": 0, "silent_errors": 1}]
        )
        self.assertEqual(1, s["runs_with_metrics"])
        self.assertEqual(4, s["tool_calls"])
        self.assertAlmostEqual(0.25, s["silent_error_rate"])

    def test_all_missing_yields_none(self):
        s = summarize_tool_metrics([None, None])
        self.assertEqual(0, s["runs_with_metrics"])
        self.assertIsNone(s["silent_error_rate"])


class TestScanToolStats(unittest.TestCase):
    """`run_local.scan_tool_stats`：静默两类（空错误 / 结果丢失）的事件级计数。"""

    def _ev(self, et, **payload):
        return {"event_type": et, "payload": dict(payload, type=et)}

    def test_empty_error_and_lost_result(self):
        from run_local import scan_tool_stats

        msgs = [
            self._ev("tool_call", call_id="a", name="bash", args={}),
            self._ev("tool_result", call_id="a", is_error=True, output="   "),  # 失败但空白
            self._ev("tool_call", call_id="b", name="read", args={}),  # 永远没有结果
            self._ev("tool_call", call_id="c", name="read", args={}),
            self._ev("tool_result", call_id="c", is_error=False, output="ok"),
        ]
        s = scan_tool_stats(msgs)
        self.assertEqual(3, s["tool_calls"])
        self.assertEqual(1, s["tool_errors"])
        self.assertEqual(1, s["empty_errors"])
        self.assertEqual(1, s["lost_results"])
        self.assertEqual(2, s["silent_errors"], "空错误 + 结果丢失")

    def test_visible_error_is_not_silent(self):
        from run_local import scan_tool_stats

        s = scan_tool_stats(
            [
                self._ev("tool_call", call_id="a", name="bash", args={}),
                self._ev("tool_result", call_id="a", is_error=True, output="exit code 1: boom"),
            ]
        )
        self.assertEqual(1, s["tool_errors"])
        self.assertEqual(0, s["silent_errors"])


class TestSplitRc(unittest.TestCase):
    def test_extracts_rc_and_removes_line(self):
        out = "test result: ok\nVERIFY_PASS: T03\n__HEARTH_RC=0\n"
        body, rc = split_rc(out)
        self.assertEqual(rc, 0)
        self.assertNotIn("__HEARTH_RC", body)
        self.assertIn("VERIFY_PASS: T03", body)

    def test_nonzero_rc(self):
        body, rc = split_rc("VERIFY_FAIL: x\n__HEARTH_RC=101\n")
        self.assertEqual(rc, 101)
        self.assertFalse(judge_verify(body, rc))

    def test_missing_rc_marker_is_none(self):
        body, rc = split_rc("VERIFY_PASS: T03\n")
        self.assertIsNone(rc)
        self.assertFalse(judge_verify(body, rc))

    def test_unparsable_rc_is_none(self):
        body, rc = split_rc("__HEARTH_RC=not-a-number\n")
        self.assertIsNone(rc)

    def test_last_occurrence_wins(self):
        body, rc = split_rc("__HEARTH_RC=1\n中间输出\n__HEARTH_RC=0\n")
        self.assertEqual(rc, 0)

    def test_end_to_end_red_case_now_fails(self):
        """T03 修复前的原始输出：cargo test 失败、却打印 VERIFY_PASS。
        经 split_rc + judge_verify 后必须判失败（这正是 D-222 要堵的 fail-open）。"""
        raw = (
            "test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out\n\n"
            "error: test failed, to rerun pass `--lib`\n"
            "VERIFY_PASS\n"
            "__HEARTH_RC=0\n"
        )
        body, rc = split_rc(raw)
        # 注意：这是「旧脚本」的输出——它自己退出 0。rc=0 + 有标记 ⇒ judge 会判通过。
        # 因此 D-222 的另一半修复是**脚本本身**（新 verify_lib.sh 的 vh_require_cargo_test
        # 会让 rc 非 0）。此测试固定该事实，防止有人把退出码口径再改回去。
        self.assertTrue(judge_verify(body, rc))
        self.assertFalse(judge_verify(body, 101))


class TestJudgeVerify(unittest.TestCase):
    def test_pass_requires_zero_rc(self):
        """输出含 VERIFY_PASS 但退出码非 0 ⇒ 必须判失败（修复前判通过）。"""
        out = "test result: FAILED. 0 passed; 2 failed\n\nVERIFY_PASS\n"
        self.assertFalse(judge_verify(out, 101))
        self.assertFalse(judge_verify(out, 1))

    def test_pass_when_rc_zero_and_marker_present(self):
        out = "test result: ok. 2 passed; 0 failed\nVERIFY_PASS: T03\n"
        self.assertTrue(judge_verify(out, 0))

    def test_no_marker_is_fail_even_with_zero_rc(self):
        """退出码 0 但无标记 ⇒ 判失败（防「判据什么都没做也退出 0」）。"""
        self.assertFalse(judge_verify("test result: ok\n", 0))

    def test_rc_unknown_is_fail_closed(self):
        """取不到退出码 ⇒ 判失败（未知不得当成功）。"""
        self.assertFalse(judge_verify("VERIFY_PASS: T03\n", None))

    def test_marker_must_be_a_standalone_line(self):
        """标记必须独立成行，散落在说明文字里的子串不算。"""
        out = "VERIFY_FAIL: 提示文本里提到 VERIFY_PASS 这个字样\n"
        self.assertFalse(judge_verify(out, 1))
        self.assertFalse(judge_verify(out, 0))

    def test_failure_output_is_fail(self):
        out = "VERIFY_FAIL: cargo test 退出码非 0（实得 101）\n"
        self.assertFalse(judge_verify(out, 1))


if __name__ == "__main__":
    unittest.main()
