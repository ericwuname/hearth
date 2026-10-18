#!/usr/bin/env python3
"""`bench/gen_heldout.py` 的契约测试（M0 / D-227）。

钉住四件事：
  1. 产物齐备：每个任务都有 fixture/ + goal.txt + verify.sh + meta.json，且有配套参考解；
  2. 元数据合规：`suite == "held-out"`、`acceptance` 非空、`heldout_family` 是已知家族；
  3. **可复现**：同 seed 生成两次 ⇒ 逐字节一致（否则"held-out"就不可审计）；
  4. **随 seed 变化**：换 seed ⇒ 至少一个文件不同（否则 seed 是摆设）。

注：测试一律写到临时目录，**不碰仓库内的生成物目录**。
"""
import pathlib
import sys
import tempfile
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))

import gen_heldout  # noqa: E402


def gen(seed, count, root):
    out = pathlib.Path(root) / "tasks-heldout"
    sol = pathlib.Path(root) / "solutions-heldout"
    tasks = gen_heldout.generate(seed, count, out, sol)
    return out, sol, tasks


def read_all(root):
    root = pathlib.Path(root)
    return {str(p.relative_to(root)): p.read_bytes() for p in root.rglob("*") if p.is_file()}


class TestGenHeldout(unittest.TestCase):
    def test_products_are_complete_and_schema_compliant(self):
        with tempfile.TemporaryDirectory() as tmp:
            out, sol, tasks = gen(1234, 8, tmp)
            self.assertEqual(8, len(tasks))
            for rec in tasks:
                tid = rec["task"]
                d = out / tid
                self.assertTrue((d / "fixture" / "Cargo.toml").is_file(), f"{tid} 缺 Cargo.toml")
                self.assertTrue((d / "fixture" / "src" / "lib.rs").is_file(), f"{tid} 缺 lib.rs")
                self.assertTrue((d / "goal.txt").is_file(), f"{tid} 缺 goal.txt")
                self.assertTrue((d / "verify.sh").is_file(), f"{tid} 缺 verify.sh")
                self.assertTrue((sol / f"{tid}.sh").is_file(), f"{tid} 缺参考解")
                meta = __import__("json").loads((d / "meta.json").read_text(encoding="utf-8"))
                self.assertEqual("held-out", meta["suite"], f"{tid} 的 suite 必须为 held-out")
                self.assertTrue(str(meta["acceptance"]).strip(), f"{tid} 的 acceptance 不能为空")
                self.assertIn(meta["heldout_family"], gen_heldout.FAMILIES)
                self.assertEqual(1234, meta["heldout_seed"])

    def test_reproducible_for_same_seed(self):
        with tempfile.TemporaryDirectory() as a, tempfile.TemporaryDirectory() as b:
            out_a, sol_a, _ = gen(777, 6, a)
            out_b, sol_b, _ = gen(777, 6, b)
            self.assertEqual(read_all(out_a), read_all(out_b), "同 seed 必须逐字节一致")
            self.assertEqual(read_all(sol_a), read_all(sol_b), "参考解也必须一致")

    def test_varies_with_seed(self):
        with tempfile.TemporaryDirectory() as a, tempfile.TemporaryDirectory() as b:
            out_a, _, _ = gen(1, 6, a)
            out_b, _, _ = gen(2, 6, b)
            self.assertNotEqual(read_all(out_a), read_all(out_b), "换 seed 应至少有一处不同")

    def test_families_are_spread(self):
        with tempfile.TemporaryDirectory() as tmp:
            _, _, tasks = gen(99, len(gen_heldout.FAMILIES) * 2, tmp)
            seen = {t["family"] for t in tasks}
            self.assertEqual(set(gen_heldout.FAMILIES), seen, "每个家族都应被覆盖到")

    def test_genesis_records_provenance(self):
        with tempfile.TemporaryDirectory() as tmp:
            out, _, _ = gen(4242, 5, tmp)
            g = __import__("json").loads((out / "GENESIS.json").read_text(encoding="utf-8"))
            self.assertEqual(4242, g["seed"])
            self.assertEqual(5, g["count"])
            self.assertEqual(5, len(g["tasks"]))


if __name__ == "__main__":
    unittest.main()
