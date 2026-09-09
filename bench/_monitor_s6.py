#!/usr/bin/env python3
"""S6 watchdog (observe-only): wait for zhipu run=1 to reach 20 distinct records.

matrix_v13.py (pid alive) self-heals via --attempts 4 + runner.py --resume; each
task is checkpointed to disk immediately. This monitor ONLY observes progress so
it never spawns a second runner against the same service. If the target is not
met when this exits, a human/agent should check whether matrix_v13.py is still
alive and relaunch if needed.
"""
import json, time
from pathlib import Path
from collections import defaultdict

HERE = Path(__file__).resolve().parent
RES = HERE / "results" / "raw" / "matrix-v13-zhipu.jsonl"


def count_run1():
    keys = set()
    if RES.exists():
        for line in open(RES, encoding="utf-8"):
            line = line.strip()
            if not line:
                continue
            try:
                d = json.loads(line)
            except Exception:
                continue
            if d.get("run") == 1:
                keys.add((d.get("task"), d.get("run")))
    return len(keys)


print(f"[{time.strftime('%H:%M:%S')}] S6 watchdog (observe-only) start: run=1={count_run1()}/20")
for i in range(30):  # up to ~10 min
    n = count_run1()
    print(f"  [{time.strftime('%H:%M:%S')}] run=1={n}/20", flush=True)
    if n >= 20:
        print(f"[{time.strftime('%H:%M:%S')}] TARGET REACHED: run=1=20/20")
        break
    time.sleep(20)
else:
    print(f"[{time.strftime('%H:%M:%S')}] TIMEOUT: run=1={count_run1()}/20 (matrix_v13.py still self-healing?)")
