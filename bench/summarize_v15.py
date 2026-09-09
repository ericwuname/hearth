#!/usr/bin/env python3
"""v15 S5: fold every v15 raw artifact into one markdown evidence table.

Inputs (all optional -- a missing one is reported as "not run" rather than
crashing, so this can be run mid-flight):

    results/raw/matrix-v15-deepseek.jsonl          S1  brain selection (20x2)
    results/raw/matrix-v15-gemini-ceiling.jsonl    S1b ceiling control
    results/raw/matrix-v14-zhipu.jsonl             v14 baseline for the delta
    results/raw/replay-v15.jsonl                   S2  deterministic replay
    results/raw/stress-v15.jsonl                   S2b stress field

Output: results/matrix-v15.md
"""
import json
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
RAW = HERE / "results" / "raw"
OUT = HERE / "results" / "matrix-v15.md"

CEILING_TASKS = ["T14-add-serde", "T19-merge-duplicate", "T09-add-error-type"]


def load(name):
    p = RAW / name
    if not p.exists():
        return None
    rows = []
    for line in p.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line:
            try:
                rows.append(json.loads(line))
            except json.JSONDecodeError:
                pass
    return rows


def dedup(rows, keys=("task", "run")):
    """--resume can append a retry of the same (task, run); last write wins."""
    seen = {}
    for r in rows:
        seen[tuple(r.get(k) for k in keys)] = r
    return list(seen.values())


def rate(rows, field="success"):
    if not rows:
        return 0, 0, 0.0
    ok = sum(1 for r in rows if r.get(field))
    return ok, len(rows), ok / len(rows) * 100


def by_level(rows):
    d = defaultdict(lambda: [0, 0])
    for r in rows:
        b = d[r.get("level", "?")]
        b[1] += 1
        b[0] += 1 if r.get("success") else 0
    return d


def by_task(rows):
    d = defaultdict(lambda: [0, 0])
    for r in rows:
        b = d[r.get("task", "?")]
        b[1] += 1
        b[0] += 1 if r.get("success") else 0
    return d


def main():
    L = []
    L.append("# v15 证据表（S1 择脑 / S1b 天花板 / S2 确定性回放 / S2b 应力场）\n")

    # ---------- S1 ----------
    ds = load("matrix-v15-deepseek.jsonl")
    zp = load("matrix-v14-zhipu.jsonl")
    L.append("## S1 择脑：deepseek 20x2 全矩阵\n")
    if not ds:
        L.append("_未产出 matrix-v15-deepseek.jsonl_\n")
    else:
        ds = dedup(ds)
        ok, n, pct = rate(ds)
        L.append(f"- **总通过率：{ok}/{n} = {pct:.1f}%**（红线 ≥90% → "
                 f"{'**达标**' if pct >= 90 else '**未达标**'}）")
        if zp:
            zp = dedup(zp)
            zok, zn, zpct = rate(zp)
            L.append(f"- v14 zhipu 对照：{zok}/{zn} = {zpct:.1f}%"
                     f"（delta **{pct - zpct:+.1f} pt**）")
        wall = [r.get("wall_s", 0) for r in ds if r.get("wall_s")]
        if wall:
            wall.sort()
            L.append(f"- 单任务耗时：中位 {wall[len(wall) // 2]:.1f}s，"
                     f"p90 {wall[int(len(wall) * 0.9)]:.1f}s，最长 {wall[-1]:.1f}s")
        L.append("\n| 难度 | deepseek v15 | zhipu v14 |")
        L.append("|---|---|---|")
        dl, zl = by_level(ds), by_level(zp) if zp else {}
        for lv in sorted(dl):
            a = dl[lv]
            b = zl.get(lv)
            L.append(f"| {lv} | {a[0]}/{a[1]} ({a[0] / a[1] * 100:.0f}%) | "
                     + (f"{b[0]}/{b[1]} ({b[0] / b[1] * 100:.0f}%) |" if b else "- |"))
        fails = [r for r in ds if not r.get("success")]
        L.append(f"\n失败明细（{len(fails)} 条）：\n")
        if fails:
            L.append("| task | run | phase | steps | verify tail |")
            L.append("|---|---|---|---|---|")
            for r in sorted(fails, key=lambda x: (x.get("task", ""), x.get("run", 0))):
                tail = (r.get("verify_tail") or r.get("error") or "").replace("\n", " ")[-90:]
                L.append(f"| {r.get('task')} | {r.get('run')} | {r.get('phase')} | "
                         f"{r.get('steps')} | `{tail}` |")
        else:
            L.append("_无失败_")
    L.append("")

    # ---------- S1b ----------
    L.append("## S1b 天花板对照：同通道换大模型\n")
    L.append("原计划用 gemini 做对照，实测被网关缺陷阻断（Gemini 3.x 多轮 function "
             "calling 要求回传 `thought_signature`，OpenAI 兼容层未透传 → 第 2 步 HTTP 400）。"
             "换通道会同时改变网关行为，无法把失败归因到模型强度，"
             "因此改为在**同一条已验证健康的通道内只替换模型规模**"
             "（deepseek `v4-flash → v4-pro`、zhipu `glm-4.5-air → glm-4.7`），"
             "使 S1b 的唯一自变量就是模型能力。\n")
    L.append("判据：强模型也挂 → 是**夹具/工具链**问题，能力白皮书作废；"
             "强模型过而现役小模型挂 → 确证**模型能力墙**，白皮书成立。\n")
    ceil = {}
    for label, fn in [("deepseek-v4-pro", "matrix-v15-deepseek-pro-ceiling.jsonl"),
                      ("glm-4.7", "matrix-v15-zhipu-max-ceiling.jsonl")]:
        r = load(fn)
        if r:
            ceil[label] = by_task(dedup(r))
    if not ceil:
        L.append("_未产出天花板对照数据_\n")
    else:
        dt = by_task(ds) if ds else {}
        zt = by_task(zp) if zp else {}

        def cell(d, t):
            b = d.get(t)
            return f"{b[0]}/{b[1]}" if b else "-"

        head = "| task | " + " | ".join(ceil) + \
               " | deepseek-v4-flash (v15) | glm-4.5-air (v14) | 结论 |"
        L.append(head)
        L.append("|---" * (len(ceil) + 4) + "|")
        for t in CEILING_TASKS:
            strong = [ceil[k].get(t) for k in ceil]
            strong_pass = any(b and b[0] == b[1] for b in strong)
            strong_any = any(b for b in strong)
            weak_fail = ((dt.get(t) and dt[t][0] < dt[t][1])
                         or (zt.get(t) and zt[t][0] < zt[t][1]))
            if not strong_any:
                verdict = "-"
            elif strong_pass and weak_fail:
                verdict = "**模型能力墙**（白皮书成立）"
            elif strong_pass:
                verdict = "无人失败，非难点"
            else:
                verdict = "**强模型也挂 → 查夹具/工具链**"
            L.append(f"| {t} | " + " | ".join(cell(ceil[k], t) for k in ceil) +
                     f" | {cell(dt, t)} | {cell(zt, t)} | {verdict} |")
    L.append("")

    # ---------- S2 ----------
    rp = load("replay-v15.jsonl")
    L.append("## S2 确定性回放（ReplayProvider，LLM 打桩 / 工具真跑）\n")
    if not rp:
        L.append("_未产出 replay-v15.jsonl_\n")
    else:
        rp = dedup(rp, keys=("fixture",))
        ok, n, pct = rate(rp, "ok")
        L.append(f"- **回放通过率：{ok}/{n} = {pct:.1f}%**"
                 f"（录制全为 PASS，红线 = 100%，"
                 f"{'**达标**' if ok == n else '**未达标**'}）")
        wall = [r.get("wall_s", 0) for r in rp if r.get("wall_s")]
        rec = [r.get("recorded_wall_s", 0) for r in rp if r.get("recorded_wall_s")]
        if wall and rec:
            L.append(f"- 耗时：回放合计 {sum(wall):.0f}s vs 原始录制合计 {sum(rec):.0f}s"
                     f"（省 {100 - sum(wall) / sum(rec) * 100:.0f}%，且零 token）")
        bad = [r for r in rp if not r.get("ok")]
        if bad:
            L.append("\n| fixture | task | phase | steps(rec) | why |")
            L.append("|---|---|---|---|---|")
            for r in bad:
                why = (r.get("why") or r.get("verify_tail") or "").replace("\n", " ")[-90:]
                L.append(f"| {r.get('fixture')} | {r.get('task')} | {r.get('phase')} | "
                         f"{r.get('steps')}({r.get('recorded_steps')}) | `{why}` |")
    L.append("")

    # ---------- S2b ----------
    st = load("stress-v15.jsonl")
    L.append("## S2b 应力场 v15（ST7 判据拆分后）\n")
    if not st:
        L.append("_未产出 stress-v15.jsonl_\n")
    else:
        ok, n, pct = rate(st, "ok")
        L.append(f"- 通过：{ok}/{n} = {pct:.1f}%；panic = "
                 f"{sum(1 for r in st if not r.get('no_panic', True))}\n")
        L.append("| scenario | 结果 | 关键信息 |")
        L.append("|---|---|---|")
        from itertools import groupby
        st_sorted = sorted(st, key=lambda r: r.get("scenario", ""))
        for sc, group in groupby(st_sorted, key=lambda r: r.get("scenario", "")):
            grp = list(group)
            runs_ok = sum(1 for r in grp if r.get("ok"))
            # Pick the richest detail string (prefer longer, more informative one)
            dets = sorted(set((r.get("detail") or "").replace("\n", " ") for r in grp))
            det = dets[0] if dets else ""
            # For ST7, a short summary is more useful
            if "ST7" in sc:
                det = det[:120]
            else:
                det = det[:90]
            L.append(f"| {sc} | {runs_ok}/{len(grp)} PASS | {det} |")
    L.append("")

    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text("\n".join(L), encoding="utf-8")
    print(f"wrote {OUT}")
    print("\n".join(L[:40]))


if __name__ == "__main__":
    main()
