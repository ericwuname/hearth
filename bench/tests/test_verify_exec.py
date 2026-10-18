#!/usr/bin/env python3
"""`verify_exec` 契约测试（M0 / D-224）。

不需要 SSH / bash：用**假传输对象**把三条契约钉死——
  ① `stage` 必须**同时**送达判据与判据库（缺库会把「环境缺件」误报成「任务失败」）；
  ② 判定以**退出码**为准（不看输出子串）；
  ③ 取不到退出码 ⇒ 判失败（未知不得当成功）。

另有一条专门复现旧缺陷的用例：**只送判据、不送判据库** ⇒ 判失败——这正是修复前
`replay.py` 在 D-222 换了新 verify.sh 之后会出现的现象（回放路径全判失败）。
"""
import pathlib
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))

import judge  # noqa: E402
import verify_exec  # noqa: E402

BENCH = pathlib.Path(__file__).resolve().parents[1]
T03_VERIFY = BENCH / "tasks" / "T03-fix-off-by-one" / "verify.sh"


class FakeTransport:
    """模拟远端工作区：`put` 落进 files；`exec_` 按 files 现状回话。"""

    def __init__(self, rc=0, emit_marker=True):
        self.files = {}
        self.commands = []
        self.rc = rc
        self.emit_marker = emit_marker

    def put(self, name, data):
        self.files[name] = data

    def exec_(self, cmd):
        self.commands.append(cmd)
        if verify_exec.VERIFY_LIB_NAME not in self.files:
            # 实测行为（本地已复现）：新 verify.sh 找不到判据库时 fail-loud。
            return "VERIFY_FAIL: 缺少 verify_lib.sh（判据库）\n__HEARTH_RC=1\n"
        marker = "VERIFY_PASS: T99\n" if self.emit_marker else ""
        return f"test result: ok\n{marker}__HEARTH_RC={self.rc}\n"


class TestVerifyExecContract(unittest.TestCase):
    def test_stages_both_script_and_lib(self):
        t = FakeTransport()
        verify_exec.verify_task(t.put, t.exec_, T03_VERIFY)
        self.assertIn(verify_exec.VERIFY_SCRIPT_NAME, t.files)
        self.assertIn(verify_exec.VERIFY_LIB_NAME, t.files)
        self.assertTrue(t.files[verify_exec.VERIFY_LIB_NAME], "判据库内容不应为空")

    def test_command_carries_rc_marker(self):
        t = FakeTransport()
        verify_exec.verify_task(t.put, t.exec_, T03_VERIFY)
        self.assertEqual(1, len(t.commands))
        self.assertIn(judge.RC_MARKER, t.commands[0])

    def test_success_requires_zero_rc_and_marker(self):
        ok = FakeTransport(rc=0, emit_marker=True)
        self.assertTrue(verify_exec.verify_task(ok.put, ok.exec_, T03_VERIFY)[0])

        nonzero = FakeTransport(rc=101, emit_marker=True)
        self.assertFalse(
            verify_exec.verify_task(nonzero.put, nonzero.exec_, T03_VERIFY)[0],
            "退出码非 0 却因输出含 VERIFY_PASS 而判通过 —— 这是 fail-open",
        )

        no_marker = FakeTransport(rc=0, emit_marker=False)
        self.assertFalse(
            verify_exec.verify_task(no_marker.put, no_marker.exec_, T03_VERIFY)[0],
            "退出码 0 但无独立 VERIFY_PASS 标记 ⇒ 必须判失败",
        )

    def test_missing_lib_is_a_failure(self):
        """复现旧缺陷：只送判据不送判据库 ⇒ 判据 fail-loud，任务被判失败。"""
        t = FakeTransport()
        t.put(verify_exec.VERIFY_SCRIPT_NAME, T03_VERIFY.read_bytes())
        body, rc = verify_exec.run(t.exec_)
        self.assertFalse(judge.judge_verify(body, rc))
        self.assertIn("verify_lib.sh", body)


if __name__ == "__main__":
    unittest.main()
