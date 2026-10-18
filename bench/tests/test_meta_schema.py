#!/usr/bin/env python3
"""任务元数据 schema 门禁（M0 / D-223）。

**为什么要有它**：`meta.json` 是评测台的**输入契约**——runner 按它决定预算/超时，
报表按它分档；而 `suite` 决定一个任务属于「开发可见集」还是「held-out 判定集」
（M0-4 抗 reward hacking 的关键）。缺字段不会报错，只会**静默少一层信息**：
例如没有 `suite` 就无法证明 held-out 任务没被混进开发集。

判据（每个含 goal.txt 的任务目录）：
  1. `meta.json` 存在且可解析；
  2. 必填键齐备：level / expect_files_changed / timeout_min / suite / acceptance；
  3. `level` 形如 `L<数字>`；`expect_files_changed` / `timeout_min` 为正整数；
  4. `suite` ∈ {public, held-out}；
  5. `acceptance` 是非空字符串（一句话说明"什么算完成"——判据的可读契约）。
"""
import json
import pathlib
import re
import unittest

BENCH = pathlib.Path(__file__).resolve().parents[1]
TASK_ROOTS = [BENCH / "tasks", BENCH / "tasks-heldout"]

REQUIRED = ("level", "expect_files_changed", "timeout_min", "suite", "acceptance")
SUITES = {"public", "held-out"}
LEVEL_RE = re.compile(r"^L\d+$")


def task_dirs():
    out = []
    for root in TASK_ROOTS:
        if not root.is_dir():
            continue
        for d in sorted(root.iterdir()):
            if d.is_dir() and (d / "goal.txt").is_file():
                out.append(d)
    return out


class TestMetaSchema(unittest.TestCase):
    def test_at_least_one_task_discovered(self):
        self.assertTrue(task_dirs(), "未发现任何任务目录——门禁自身可能失效")

    def test_required_fields_present_and_typed(self):
        problems = []
        for d in task_dirs():
            name = d.name
            meta_path = d / "meta.json"
            if not meta_path.is_file():
                problems.append(f"{name}: 缺 meta.json")
                continue
            try:
                meta = json.loads(meta_path.read_text(encoding="utf-8"))
            except json.JSONDecodeError as e:
                problems.append(f"{name}: meta.json 解析失败（{e}）")
                continue
            for k in REQUIRED:
                if k not in meta:
                    problems.append(f"{name}: 缺字段 `{k}`")
            level = meta.get("level")
            if level is not None and not (isinstance(level, str) and LEVEL_RE.match(level)):
                problems.append(f"{name}: level 应是 `L<数字>`，实得 {level!r}")
            for k in ("expect_files_changed", "timeout_min"):
                v = meta.get(k)
                if v is not None and not (isinstance(v, int) and v > 0):
                    problems.append(f"{name}: {k} 应为正整数，实得 {v!r}")
            suite = meta.get("suite")
            if suite is not None and suite not in SUITES:
                problems.append(f"{name}: suite 应为 {sorted(SUITES)} 之一，实得 {suite!r}")
            acc = meta.get("acceptance")
            if acc is not None and not (isinstance(acc, str) and acc.strip()):
                problems.append(f"{name}: acceptance 应为非空字符串")
        self.assertEqual([], problems, "meta.json schema 不合规：\n" + "\n".join(problems))


if __name__ == "__main__":
    unittest.main()
