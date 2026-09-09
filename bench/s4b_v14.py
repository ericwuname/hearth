#!/usr/bin/env python3
"""v14 S4b budget retest (detached).

1. Wait for stress_v14 to finish (24 records in stress-v14.jsonl) — shares the
   service with the bench, must not overlap.
2. Rerun the budget-truncated failures (steps==15 exactly) with max_steps=20
   now present in meta.json: T09, T10, T13 (2 runs each).
   Output: matrix-v14b-zhipu.jsonl (separate table; main table untouched).

Log: bench/results/s4b-v14.log
"""
import json
import os
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
STRESS = HERE / "results" / "raw" / "stress-v14.jsonl"
LOG = HERE / "results" / "s4b-v14.log"
EXPECT_STRESS = 24

TASKS = "T09-add-error-type,T10-extract-config,T13-fix-index"


def log(msg):
    line = f"[{time.strftime('%H:%M:%S')}] {msg}"
    print(line, flush=True)
    with open(LOG, "a", encoding="utf-8") as f:
        f.write(line + "\n")


def stress_count():
    if not STRESS.exists():
        return 0
    n = 0
    for line in STRESS.read_text(encoding="utf-8").splitlines():
        try:
            json.loads(line)
            n += 1
        except Exception:
            pass
    return n


def main():
    log(f"s4b start: waiting for stress field ({EXPECT_STRESS} records)")
    idle = 0
    while True:
        n = stress_count()
        if n >= EXPECT_STRESS:
            break
        log(f"stress progress {n}/{EXPECT_STRESS}")
        time.sleep(120)
        idle += 1
        if idle > 45:  # 90 min hard stop
            log("TIMEOUT waiting for stress; proceeding anyway (service may be busy)")
            break
    log("stress done; grace 30s, then budget retest")
    time.sleep(30)

    env = dict(os.environ)
    env.update({
        "BENCH_PROVIDER": "zhipu",
        "BENCH_OUT": "matrix-v14b-zhipu.jsonl",
        "BENCH_RUNS": "2",
    })
    log(f"=== S4b retest start: {TASKS} x2 (max_steps=20 via meta.json) ===")
    proc = subprocess.run(
        [sys.executable, "-u", "runner.py", "batch", "--tasks", TASKS,
         "--runs", "2", "--resume"],
        cwd=str(HERE), env=env,
        stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    for ln in (proc.stdout or "").splitlines():
        log(f"  [s4b] {ln}")
    log(f"=== S4b rc={proc.returncode} ===")
    log("s4b DONE")


if __name__ == "__main__":
    main()
