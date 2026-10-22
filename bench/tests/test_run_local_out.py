"""D-240：`run_local.py --out` 的批次写入语义。

事故现场：一次 `--tasks-dir` 填错的失败跑（8 条"任务目录不存在"）与紧随其后的
正确跑（8 条全 PASS）叠在同一份 jsonl 里，`scorecard` 读成 8/16 = 50%（真实 8/8）。
口径与 `stress_local.py` 对齐：**默认清空式**，`--append` 才追加。
"""
import io
import json
import sys
import pathlib
import tempfile
import unittest

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

import run_local  # noqa: E402


def rows(path):
    return [json.loads(x) for x in io.open(path, encoding="utf-8") if x.strip()]


class WriteResultsTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.out = pathlib.Path(self.tmp.name) / "raw" / "x.jsonl"

    def tearDown(self):
        self.tmp.cleanup()

    def test_default_truncates_previous_batch(self):
        run_local.write_results(self.out, [{"ok": False, "why": "找错目录"}])
        run_local.write_results(self.out, [{"ok": True}, {"ok": True}])
        self.assertEqual(len(rows(self.out)), 2, "默认必须清空上一批，否则读数重复计入")

    def test_append_opt_in_keeps_history(self):
        run_local.write_results(self.out, [{"ok": False}])
        run_local.write_results(self.out, [{"ok": True}], append=True)
        self.assertEqual(len(rows(self.out)), 2)


if __name__ == "__main__":
    unittest.main()
