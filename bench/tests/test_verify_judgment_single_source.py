#!/usr/bin/env python3
"""门禁：判据的「执行 + 判定」必须只有一份实现（M0 / D-224）。

背景（实测）：判定口径曾在两处各写一份——`runner.py` 与 `replay.py`。
D-222 只修了 `runner.py`：
  - `runner.py` 改为「上传判据库 + 回传退出码 + `judge_verify`」；
  - `replay.py` 仍留着旧口径 `success = "VERIFY_PASS" in verify_out`，**且不上传判据库**
    ⇒ 新版 `verify.sh`（`. "$(dirname "$0")/verify_lib.sh"`）在**回放路径**上会一律报
    「缺少 verify_lib.sh」并判失败 —— 也就是「同能力多处各写一份」直接造成了漏改。

本门禁两条判据（覆盖所有会执行 verify.sh 的入口）：
  1. 不得存在裸子串判定 `"VERIFY_PASS" in`（忽略退出码 ⇒ fail-open）；
  2. 必须走共享实现 `verify_exec`（送达判据 + 判据库 + 执行 + 判定，一份代码）。
"""
import pathlib
import unittest

BENCH = pathlib.Path(__file__).resolve().parents[1]

# 会执行任务判据的全部入口（新增入口时必须登记在此，否则门禁覆盖不到）。
JUDGMENT_ENTRYPOINTS = ("runner.py", "replay.py", "run_local.py")


def code_lines(name: str):
    """剥掉整行注释（`#` 开头）后的源码行——避免文档里复述旧口径被误报。"""
    text = (BENCH / name).read_text(encoding="utf-8")
    return [l for l in text.splitlines() if not l.strip().startswith("#")]


class TestJudgmentSingleSource(unittest.TestCase):
    def test_entrypoints_exist(self):
        for name in JUDGMENT_ENTRYPOINTS:
            self.assertTrue((BENCH / name).is_file(), f"找不到 {name}——门禁自身可能失效")

    def test_no_bare_substring_judgment(self):
        offenders = []
        for name in JUDGMENT_ENTRYPOINTS:
            for i, line in enumerate(code_lines(name), 1):
                if '"VERIFY_PASS" in' in line:
                    offenders.append(f"{name}:{i}: {line.strip()}")
        self.assertEqual(
            [], offenders,
            "存在裸子串判定（只看输出、忽略退出码 ⇒ fail-open）：\n" + "\n".join(offenders),
        )

    def test_uses_shared_verify_exec(self):
        missing = [n for n in JUDGMENT_ENTRYPOINTS if "verify_exec" not in "\n".join(code_lines(n))]
        self.assertEqual([], missing, f"以下入口未走共享判据执行实现 verify_exec：{missing}")


if __name__ == "__main__":
    unittest.main()
