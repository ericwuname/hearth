#!/usr/bin/env python3
"""Baseline report from JSONL — funnel chart + level breakdown."""
import json, sys
from pathlib import Path
from collections import Counter

RESULT = Path(__file__).parent / "results" / "raw" / "baseline-v12.jsonl"

def load():
    if not RESULT.exists():
        print(f"No results at {RESULT}")
        return []
    records = []
    for line in RESULT.read_text().splitlines():
        if line.strip():
            records.append(json.loads(line))
    return records

def report(records):
    if not records:
        print("No data.")
        return
    
    total = len(records)
    ok = sum(1 for r in records if r["success"])
    pct = round(100 * ok / total, 1)
    
    # By level
    by_level = {}
    for r in records:
        lv = r.get("level", "?")
        by_level.setdefault(lv, []).append(r)
    
    # Phase/error distribution
    phases = Counter(r.get("phase", r.get("error", "???")) for r in records)
    wall_times = [r.get("wall_s", 0) for r in records]
    avg_wall = round(sum(wall_times) / len(wall_times), 1) if wall_times else 0
    
    print(f"# v12.1 Baseline Report\n")
    print(f"## Overview")
    print(f"| metric | value |")
    print(f"|--------|-------|")
    print(f"| runs | {total} |")
    print(f"| passed | {ok} ({pct}%) |")
    print(f"| avg wall | {avg_wall}s |")
    print(f"| provider | {records[0].get('provider','?')} |")
    print()
    
    print(f"## Phase Distribution")
    for phase, count in phases.most_common():
        bar = "█" * min(40, count * 40 // total)
        print(f"| {phase:15s} | {count:3d} | {bar} |")
    print()
    
    print(f"## By Difficulty Level")
    for lv in sorted(by_level.keys()):
        recs = by_level[lv]
        lv_ok = sum(1 for r in recs if r["success"])
        avg = round(sum(r.get("wall_s",0) for r in recs) / len(recs), 1)
        print(f"| {lv:5s} | {len(recs):3d} runs | {lv_ok:2d} pass | avg {avg}s |")
    print()
    
    print(f"## Task Detail")
    tasks = {}
    for r in records:
        t = r.get("task","?")
        tasks.setdefault(t, []).append(r)
    for t in sorted(tasks.keys()):
        tr = tasks[t]
        t_ok = sum(1 for r in tr if r["success"])
        avg = round(sum(r.get("wall_s",0) for r in tr) / len(tr), 1)
        ph = Counter(r.get("phase", "?") for r in tr).most_common(1)[0][0]
        print(f"| {t:30s} | {tr[0].get('level','?'):3s} | {len(tr):2d} | {t_ok:3d} | {avg:5.1f}s | {ph:10s} |")

    # Funnel
    print(f"\n## Failure Funnel")
    err_types = Counter()
    for r in records:
        if not r["success"]:
            ph = r.get("phase", "???")
            steps = r.get("steps", 0)
            if ph == "error" and steps == 0:
                err_types["plan-fail (0 steps)"] += 1
            elif ph == "error":
                err_types[f"agent-error ({steps} steps)"] += 1
            else:
                err_types[ph] += 1
    max_et = max(err_types.values()) if err_types else 1
    for et, cnt in err_types.most_common():
        bar = "█" * (cnt * 50 // max_et)
        print(f"  {cnt:3d}  {et:40s} {bar}")

if __name__ == "__main__":
    recs = load()
    report(recs)
