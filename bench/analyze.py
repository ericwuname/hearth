import json, sys
from collections import Counter
from pathlib import Path

BASE = Path(__file__).parent / "results" / "raw"

datasets = {}
for name in ["zhipu-3runs", "zhipu-v2", "deepseek-v2", "agnes-v2"]:
    p = BASE / f"{name}.jsonl"
    if p.exists():
        recs = [json.loads(l) for l in p.read_text().splitlines() if l.strip()]
        if recs:
            datasets[name] = recs

all_recs = [r for recs in datasets.values() for r in recs]
total = len(all_recs)
ok = sum(1 for r in all_recs if r["success"])
done = sum(1 for r in all_recs if r.get("phase") == "done")
err = sum(1 for r in all_recs if r.get("phase") == "error")
walls = sorted([r.get("wall_s", 0) for r in all_recs if r.get("wall_s", 0) > 0])

print("=" * 60)
print("  v12 基准全量分析")
print("=" * 60)
print(f"  总运行: {total} | 通过: {ok} | done: {done} ({round(100*done/total,1)}%) | error: {err}")
if walls:
    print(f"  wall: avg {sum(walls)/len(walls):.0f}s | p50 {walls[len(walls)//2]:.0f}s | min {walls[0]:.0f}s | max {walls[-1]:.0f}s")

print()
print("=" * 60)
print("  按数据集")
print(f"  {'name':15s} {'runs':>5s} {'pass':>4s} {'done':>5s} {'err':>5s} {'avg':>5s}")
for name, recs in datasets.items():
    ok2 = sum(1 for r in recs if r["success"])
    done2 = sum(1 for r in recs if r.get("phase") == "done")
    err2 = sum(1 for r in recs if r.get("phase") == "error")
    ws = [r.get("wall_s", 0) for r in recs]
    avg = sum(ws) / len(ws) if ws else 0
    print(f"  {name[:15]:15s} {len(recs):5d} {ok2:4d} {done2:5d} {err2:5d} {avg:5.0f}s")

print()
print("=" * 60)
print("  按难度分层")
by_lv = {}
for r in all_recs:
    lv = r.get("level", "?")
    by_lv.setdefault(lv, []).append(r)
for lv in sorted(by_lv.keys()):
    rs = by_lv[lv]
    ok2 = sum(1 for r in rs if r["success"])
    done2 = sum(1 for r in rs if r.get("phase") == "done")
    ws = [r.get("wall_s", 0) for r in rs]
    avg = sum(ws) / len(ws) if ws else 0
    print(f"  {lv:5s}: {len(rs):3d}r | pass={ok2:2d} done={done2:2d} ({round(100*done2/len(rs))}%) | avg {avg:.0f}s")

print()
print("=" * 60)
print("  按任务 (done 率)")
by_task = {}
for r in all_recs:
    t = r["task"]
    by_task.setdefault(t, []).append(r)
for t in sorted(by_task.keys()):
    rs = by_task[t]
    lv = rs[0].get("level", "?")
    t_done = sum(1 for r in rs if r.get("phase") == "done")
    t_ok = sum(1 for r in rs if r["success"])
    ws = [r.get("wall_s", 0) for r in rs]
    avg = sum(ws) / len(ws) if ws else 0
    n = len(rs)
    bar = chr(9608) * max(1, t_done * 10 // n) + chr(9617) * (10 - max(1, t_done * 10 // n))
    print(f"  {t:28s} {lv} {n:2d}r done={t_done}/{n} avg{avg:4.0f}s {bar}")

print()
print("=" * 60)
print("  失败漏斗 (zhipu-3runs 60r — 最佳数据集)")
if "zhipu-3runs" in datasets:
    z3 = datasets["zhipu-3runs"]
    phases = Counter(r.get("phase") for r in z3)
    for ph, cnt in phases.most_common():
        bar = chr(9608) * (cnt * 40 // 60)
        print(f"  {ph:10s}: {cnt:3d} ({round(100*cnt/60)}%) {bar}")

print()
print("=" * 60)
print("  结论")
print("  1. 0% pass rate — agent never modifies files")
print("  2. L2 最高 done 率 — 简单改值类任务 agent 能跑完")
print("  3. L1/L3 最差 — 阅读理解/修复 bug 最难")
print("  4. ZhiPu 22s > DeepSeek 42s > Agnes 49s (速度)")
print("  5. ZhiPu 32% done > DeepSeek 0% > Agnes 0% (完成率)")
print("  6. 瓶颈: agent Plan 完成但 edit 工具链不落地文件")
print("  7. 修复方向: edit/write_file 沙箱路径 + tool-following 强化")
