#!/usr/bin/env python3
"""评测报表入口（M1 退出判据里说的 `bench/report`）。

**实现只有一份**：口径与渲染全在 [`scorecard.py`]（通过率 / 假完成率 / 反向不一致 /
每解 tokens / 每解成本 / 静默失败率 / 步数 / 墙钟）。本文件只是命令名兼容层——
D-224 的教训是"同一能力多处各写一份"会导致口径分叉与静默错读数，这里不再复制任何逻辑。

用法：
  python bench/report.py bench/results/raw/live-agnes-full-v3.jsonl
  python bench/report.py 公开集=bench/results/raw/live-agnes-full-v3.jsonl \
                         held-out=bench/results/raw/live-agnes-heldout-v3.jsonl
"""
import argparse
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import scorecard  # noqa: E402


def main(argv=None):
    ap = argparse.ArgumentParser(description="评测报表（委托 scorecard）")
    ap.add_argument("inputs", nargs="+", help="`套件=路径`，或直接给路径（套件名取文件名）")
    ap.add_argument("--out", default=None, help="另存 markdown")
    a = ap.parse_args(argv)

    argv2 = []
    for spec in a.inputs:
        name = spec if "=" in spec else f"{pathlib.Path(spec).stem}={spec}"
        argv2 += ["--input", name]
    if a.out:
        argv2 += ["--out", a.out]
    scorecard.main(argv2)
    return 0


if __name__ == "__main__":
    sys.exit(main())
