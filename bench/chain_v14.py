#!/usr/bin/env python3
"""v14 S4->S7->S5/S6 chain driver (detached).

1. Wait for matrix_v14 (zhipu 20x2) to finish: 40 deduped (task,run) records
   in matrix-v14-zhipu.jsonl AND no live matrix_v14.py process is required —
   we simply wait for record count, then give a 60s grace period.
2. Immediately archive PASS session transcripts (sessions are in-memory; must
   happen before any service restart).
3. Run the stress field (deepseek) — it may restart the service on crash,
   which is safe only after step 2.

Log: bench/results/chain-v14.log
"""
import json
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
RAW = HERE / "results" / "raw" / "matrix-v14-zhipu.jsonl"
LOG = HERE / "results" / "chain-v14.log"
EXPECT = 40  # 20 tasks x 2 runs


def log(msg):
    line = f"[{time.strftime('%H:%M:%S')}] {msg}"
    print(line, flush=True)
    with open(LOG, "a", encoding="utf-8") as f:
        f.write(line + "\n")


def dedup_count():
    if not RAW.exists():
        return 0
    keys = set()
    for line in RAW.read_text(encoding="utf-8").splitlines():
        try:
            r = json.loads(line)
            keys.add((r["task"], r["run"]))
        except Exception:
            pass
    return len(keys)


def run_step(name, args):
    log(f"=== {name} start ===")
    proc = subprocess.run([sys.executable, "-u"] + args, cwd=str(HERE),
                          stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    for ln in (proc.stdout or "").splitlines():
        log(f"  [{name}] {ln}")
    log(f"=== {name} rc={proc.returncode} ===")
    return proc.returncode


def main():
    log(f"chain start: waiting for {EXPECT} matrix records")
    while True:
        n = dedup_count()
        if n >= EXPECT:
            break
        log(f"matrix progress {n}/{EXPECT}")
        time.sleep(120)
    log(f"matrix complete ({dedup_count()}/{EXPECT}); grace 60s for final writes")
    time.sleep(60)

    run_step("S7-archive", ["archive_replay_v14.py"])
    run_step("S5S6-stress", ["stress_v14.py"])
    log("chain DONE")


if __name__ == "__main__":
    main()
