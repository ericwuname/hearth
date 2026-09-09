#!/usr/bin/env python3
"""v15 line C (downgraded per R2) — value pilot: agent-side timing + human template.

The agent half is free: every matrix record already carries `wall_s`. The human
half cannot be produced by the agent itself, so this script emits a filled agent
table plus an empty, explicitly-specified human table for the operator.

Output: bench/results/value-pilot-v15.md
"""
import json
import statistics
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
RAW = HERE / "results" / "raw"
OUT = HERE / "results" / "value-pilot-v15.md"

SOURCES = [
    ("deepseek (v15 20x2)", "matrix-v15-deepseek.jsonl"),
    ("zhipu (v14 20x2)", "matrix-v14-zhipu.jsonl"),
]
# L2 / L3 / L4, one each — the human-comparison suite.
PILOT = ["T02-change-timeout", "T13-fix-index", "T09-add-error-type"]


def load(fname):
    p = RAW / fname
    recs = {}
    if not p.exists():
        return recs
    for line in p.read_text(encoding="utf-8").splitlines():
        try:
            r = json.loads(line)
        except Exception:
            continue
        recs[(r["task"], r["run"])] = r
    return recs


def level_of(task, recs):
    for (t, _), r in recs.items():
        if t == task:
            return r.get("level", "?")
    return "?"


def main():
    lines = ["# v15 价值雏形（线 C，降级版）", ""]
    lines += [
        "> **口径声明**：这是雏形不是完整 V 维度实验。完整实验需独立第三人、随机化任务顺序、",
        "> 排除学习效应。本轮只证明「数能算出来、方法能跑通」。agent 侧数据来自基准矩阵的",
        "> `wall_s`（端到端墙钟，含夹具上传与 verify 之外的 agent 执行时间）；人侧留空待填。",
        "",
    ]

    for label, fname in SOURCES:
        recs = load(fname)
        if not recs:
            continue
        lines += [f"## agent 侧耗时 — {label}", ""]
        by_level = defaultdict(list)
        for (t, _), r in recs.items():
            if r.get("wall_s"):
                by_level[r.get("level", "?")].append(r["wall_s"])
        lines += ["| 难度 | 样本 | 中位(s) | 均值(s) | 最快(s) | 最慢(s) |", "|---|---|---|---|---|---|"]
        for lv in sorted(by_level):
            v = sorted(by_level[lv])
            lines.append(
                f"| {lv} | {len(v)} | {statistics.median(v):.1f} | "
                f"{statistics.mean(v):.1f} | {min(v):.1f} | {max(v):.1f} |"
            )
        allv = [x for v in by_level.values() for x in v]
        if allv:
            lines.append(
                f"| **全体** | {len(allv)} | {statistics.median(allv):.1f} | "
                f"{statistics.mean(allv):.1f} | {min(allv):.1f} | {max(allv):.1f} |"
            )
        lines += ["", f"全套 {len(allv)} 次运行合计机器时间：**{sum(allv) / 60:.1f} 分钟**", ""]

    # human comparison suite
    ds = load("matrix-v15-deepseek.jsonl")
    lines += ["## 人机对照套件（3 题，人侧待填）", ""]
    lines += [
        "| 任务 | 难度 | agent 中位耗时(s) | 人耗时(s) | 效率比 |",
        "|---|---|---|---|---|",
    ]
    for t in PILOT:
        vals = [r["wall_s"] for (tt, _), r in ds.items() if tt == t and r.get("wall_s")]
        med = f"{statistics.median(vals):.1f}" if vals else "—"
        lines.append(f"| {t} | {level_of(t, ds)} | {med} | _待填_ | _待填_ |")

    lines += [
        "",
        "### 人侧计时规程（保证可比）",
        "",
        "1. 使用与 agent 相同的初始工作区：`bench/tasks/<task>/fixture/`（不要用改过的副本）。",
        "2. 只读 `goal.txt`，不看 `verify.sh`（否则等于提前拿到答案，效率比失真）。",
        "3. 计时从打开工作区开始，到 `bash verify.sh` 输出 `VERIFY_PASS` 为止。",
        "4. 中途查文档/搜索计入耗时；被打断的时间扣除。",
        "5. 三题按 L2 → L3 → L4 顺序做，中间不休息超过 10 分钟（控制状态漂移）。",
        "",
        "**已知偏差（如实记录，不要粉饰）**：同一人连做三题存在学习效应；单人样本无统计意义；",
        "人可以边做边理解需求而 agent 只有一次性 prompt。这些在 v16 完整实验中用随机化与",
        "多人样本消除。",
        "",
    ]

    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text("\n".join(lines), encoding="utf-8")
    print(f"wrote {OUT}")


if __name__ == "__main__":
    main()
