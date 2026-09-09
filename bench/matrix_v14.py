#!/usr/bin/env python3
"""v14 S4 driver: zhipu 20x2 stability rerun after the bleed-stop fixes.

Same battle-tested loop as matrix_v13.py (serial providers, --resume retries),
only the output naming changes: results go to matrix-v14-<provider>.jsonl so
v13 raw data stays untouched for comparison.

Usage:
    python matrix_v14.py [--runs 2] [--providers zhipu]
Log: bench/results/matrix-v14.log   Results: bench/results/raw/matrix-v14-<provider>.jsonl
"""
import argparse
import os
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
LOG = HERE / "results" / "matrix-v14.log"


def log(msg):
    line = f"[{time.strftime('%H:%M:%S')}] {msg}"
    print(line, flush=True)
    LOG.parent.mkdir(parents=True, exist_ok=True)
    with open(LOG, "a", encoding="utf-8") as f:
        f.write(line + "\n")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--runs", type=int, default=2)
    ap.add_argument("--providers", default="zhipu")
    ap.add_argument("--attempts", type=int, default=4,
                    help="max resume attempts per provider when the batch dies early")
    a = ap.parse_args()

    providers = [p.strip() for p in a.providers.split(",") if p.strip()]
    log(f"=== MATRIX v14 START: providers={providers} runs={a.runs} ===")

    for p in providers:
        env = dict(os.environ)
        env["BENCH_PROVIDER"] = p
        env["BENCH_OUT"] = f"matrix-v14-{p}.jsonl"
        env.setdefault("BENCH_TIMEOUT_MIN", "15")

        for attempt in range(1, a.attempts + 1):
            log(f"--- provider {p} attempt {attempt}/{a.attempts} start ---")
            t0 = time.time()
            proc = subprocess.run(
                [sys.executable, "-u", str(HERE / "runner.py"), "batch",
                 "--runs", str(a.runs), "--resume"],
                cwd=str(HERE), env=env,
                stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
            )
            for ln in (proc.stdout or "").splitlines():
                log(f"  [{p}] {ln}")
            log(f"--- provider {p} attempt {attempt} rc={proc.returncode} "
                f"in {time.time() - t0:.0f}s ---")
            if proc.returncode == 0:
                break
            log(f"    [{p}] non-zero exit, resuming in 10s")
            time.sleep(10)
        else:
            log(f"!!! provider {p} exhausted {a.attempts} attempts, moving on")

    log("=== MATRIX v14 DONE ===")


if __name__ == "__main__":
    main()
