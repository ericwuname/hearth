#!/usr/bin/env python3
"""稳定产出基线（M1 诚实性补强）：同一批任务跑 N≥5 轮，报均值 / 方差 / 区间。

口径（未知 ≠ 0）：
- 分母 = 实际参与的 run 数（记录里没有 `ok` 的视为无效，不进分母）；
- tokens 缺失（`usage.total_tokens` 全为 null）⇒ 该列 `None`，不按 0 拉低均值；
- 假完成后验（`false_completion`）与静默失败（`tool_metrics.silent_errors`）
  逐条计数；分母为 0 时比率 = `None`。

用法：
  python bench/stability.py --input bench/results/raw/stability-v4-rep*.jsonl
  python bench/stability.py --input rep1.jsonl --input rep2.jsonl --out stability.md
"""
import argparse
import glob
import json
import math
import pathlib
import statistics
import sys


def _num(v):
    return v if isinstance(v, (int, float)) else None


def _tokens(rec):
    u = rec.get("usage") or {}
    return _num(u.get("total_tokens"))


def wilson_interval(k, n, z=1.96):
    """二项比例的 Wilson 95% 区间；n=0 → (None, None)。"""
    if n <= 0:
        return (None, None)
    p = k / n
    denom = 1 + z * z / n
    centre = (p + z * z / (2 * n)) / denom
    half = z * math.sqrt((p * (1 - p) + z * z / (4 * n)) / n) / denom
    return (max(0.0, centre - half), min(1.0, centre + half))


def aggregate(records):
    """纯函数：吃多轮 jsonl 记录，出按 task 汇总 + 总览。"""
    valid = [r for r in records if isinstance(r.get("ok"), bool)]
    by_task = {}
    for r in valid:
        by_task.setdefault(r.get("task") or r.get("fixture", "?"), []).append(r)

    tasks = []
    for name in sorted(by_task):
        rs = by_task[name]
        n = len(rs)
        passed = sum(1 for r in rs if r["ok"])
        tokens = [t for t in (_tokens(r) for r in rs) if t is not None]
        steps = [s for s in (_num(r.get("steps")) for r in rs) if s is not None]
        walls = [w for w in (_num(r.get("wall_s")) for r in rs) if w is not None]
        tasks.append(
            {
                "task": name,
                "n": n,
                "passed": passed,
                "pass_rate": passed / n if n else None,
                "false": sum(1 for r in rs if r.get("false_completion")),
                "mean_steps": statistics.fmean(steps) if steps else None,
                "mean_wall_s": statistics.fmean(walls) if walls else None,
                "mean_tokens": statistics.fmean(tokens) if tokens else None,
            }
        )

    rates = [t["pass_rate"] for t in tasks if t["pass_rate"] is not None]
    n_runs = len(valid)
    passed = sum(1 for r in valid if r["ok"])
    silent = sum(int((r.get("tool_metrics") or {}).get("silent_errors") or 0) for r in valid)
    tool_calls = sum(int((r.get("tool_metrics") or {}).get("tool_calls") or 0) for r in valid)
    lo, hi = wilson_interval(passed, n_runs)
    return {
        "runs": n_runs,
        "passed": passed,
        "pass_rate": passed / n_runs if n_runs else None,
        "wilson_95": [lo, hi],
        "false_completions": sum(1 for r in valid if r.get("false_completion")),
        "claims": sum(1 for r in valid if r.get("claim") is True),
        "silent_errors": silent,
        "tool_calls": tool_calls,
        "mean_task_pass_rate": statistics.fmean(rates) if rates else None,
        "sd_task_pass_rate": statistics.stdev(rates) if len(rates) >= 2 else None,
        "min_task_pass_rate": min(rates) if rates else None,
        "max_task_pass_rate": max(rates) if rates else None,
        "tasks": tasks,
    }


def render_markdown(a, reps):
    def pct(v):
        return "—" if v is None else f"{v * 100:.1f}%"

    def num(v, nd=1):
        return "—" if v is None else f"{v:.{nd}f}"

    n_tasks = len(a["tasks"]) or 1
    batches = len(reps) / n_tasks
    lines = [
        "## 稳定产出基线（N≥5/任务）",
        "",
        f"- 参与 run：**{a['runs']}**（{len(a['tasks'])} 个任务 × {batches:.0f} 轮）",
        f"- 总通过率：**{a['passed']}/{a['runs']} = {pct(a['pass_rate'])}**，Wilson 95% 区间 "
        f"{pct(a['wilson_95'][0])} ~ {pct(a['wilson_95'][1])}",
        f"- 假完成：**{a['false_completions']}**；静默失败：**{a['silent_errors']}/{a['tool_calls']}**",
        f"- 任务通过率：均值 **{pct(a['mean_task_pass_rate'])}**，标准差 "
        f"{pct(a['sd_task_pass_rate']) if a['sd_task_pass_rate'] is not None else '—'}，"
        f"范围 {pct(a['min_task_pass_rate'])} ~ {pct(a['max_task_pass_rate'])}",
        "",
        "| 任务 | n | 通过 | 任务通过率 | 假完成 | 均步数 | 均墙钟/s | 每解 tokens |",
        "|---|---|---|---|---|---|---|---|",
    ]
    for t in a["tasks"]:
        lines.append(
            f"| {t['task']} | {t['n']} | {t['passed']}/{t['n']} | {pct(t['pass_rate'])} | "
            f"{t['false']} | {num(t['mean_steps'])} | {num(t['mean_wall_s'])} | {num(t['mean_tokens'], 0)} |"
        )
    lines += [
        "",
        "> **样本语义**：这是**子集**（10 任务 × 5 轮）的稳定度读数，不外推为全量 31 条的稳定率；"
        "单轮全量读数见 `live-agnes-full-v4`（31/31）。区间按 Wilson 二项区间计算。",
    ]
    return "\n".join(lines)


def main(argv=None):
    ap = argparse.ArgumentParser(description="稳定产出基线（N≥5/任务）")
    ap.add_argument("--input", action="append", required=True, help="一轮一个 jsonl（可多次/可 glob）")
    ap.add_argument("--out", default=None, help="另存 markdown")
    a = ap.parse_args(argv)

    paths = []
    for spec in a.input:
        paths.extend(sorted(glob.glob(spec)) or [spec])
    reps = []
    for p in paths:
        for line in pathlib.Path(p).read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if line:
                reps.append(json.loads(line))
    agg = aggregate(reps)
    md = render_markdown(agg, reps)
    print(md)
    if a.out:
        pathlib.Path(a.out).write_text(md + "\n", encoding="utf-8")
        print(f"\n→ {a.out}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
