#!/usr/bin/env python3
"""bench/judge.py 的单元测试（M0 / D-222）。

RED 基线：`test_pass_requires_zero_rc` 断言「输出含 VERIFY_PASS 但退出码非 0 ⇒ 判失败」。
修复前 `judge_verify` 只看子串 ⇒ 该断言失败（这正是要把判据口径钉死的那条）。
"""
import pathlib
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))

from judge import judge_verify, split_rc  # noqa: E402


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
