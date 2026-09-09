#!/usr/bin/env python3
"""S6 finalize: wait for run=1=20, dedupe, compute zhipu stability stats."""
import json, time
from pathlib import Path
from collections import defaultdict

HERE = Path(__file__).resolve().parent
RES = HERE / "results" / "raw" / "matrix-v13-zhipu.jsonl"


def load():
    recs = []
    if RES.exists():
        for line in open(RES, encoding="utf-8"):
            line = line.strip()
            if not line:
                continue
            try:
                recs.append(json.loads(line))
            except Exception:
                continue
    return recs


def run1_count(recs):
    return len({(d.get("task"), d.get("run")) for d in recs if d.get("run") == 1})


print(f"[{time.strftime('%H:%M:%S')}] waiting for zhipu run=1 >= 20 ...")
for _ in range(28):  # up to ~7 min
    recs = load()
    n = run1_count(recs)
    if n >= 20:
        print(f"[{time.strftime('%H:%M:%S')}] run=1 reached {n}")
        break
    print(f"  [{time.strftime('%H:%M:%S')}] run=1={n}/20", flush=True)
    time.sleep(15)
else:
    print(f"[{time.strftime('%H:%M:%S')}] TIMEOUT run=1={run1_count(load())}/20")

# Dedupe: keep last record per (task,run)
recs = load()
best = {}
for d in recs:
    best[(d.get("task"), d.get("run"))] = d  # later overrides
dedup = list(best.values())

# write deduped back
with open(RES, "w", encoding="utf-8") as f:
    for d in dedup:
        f.write(json.dumps(d) + "\n")

print(f"\n=== ZHIPU S6 STABILITY (deduped) ===")
by_run = defaultdict(dict)
for d in dedup:
    by_run[d.get("run")][d.get("task")] = d.get("success")

tasks = sorted({d.get("task") for d in dedup})
for r in (0, 1):
    rs = by_run.get(r, {})
    ok = sum(1 for v in rs.values() if v)
    tot = len(rs)
    print(f"run={r}: {ok}/{tot} = {round(100*ok/tot,1) if tot else 0}%")

agg_ok = sum(1 for d in dedup if d.get("success"))
agg_tot = len(dedup)
print(f"AGGREGATE run0+run1: {agg_ok}/{agg_tot} = {round(100*agg_ok/agg_tot,1)}%")

print("\n--- per-task consistency (0=fail both, 1=pass one, 2=pass both) ---")
both = []
either = []
none = []
for t in tasks:
    s0 = by_run.get(0, {}).get(t)
    s1 = by_run.get(1, {}).get(t)
    c = (1 if s0 else 0) + (1 if s1 else 0)
    tag = {0: "FAIL-BOTH", 1: "FLAKY", 2: "PASS-BOTH"}[c]
    print(f"  {t:24s} run0={s0} run1={s1} -> {tag}")
    if c == 2:
        both.append(t)
    elif c == 1:
        either.append(t)
    else:
        none.append(t)
print(f"\nPASS-BOTH={len(both)} FLAKY={len(either)} FAIL-BOTH={len(none)}")
if none:
    print("  FAIL-BOTH (consistent failures):", none)
if either:
    print("  FLAKY (pass in one run only):", either)
