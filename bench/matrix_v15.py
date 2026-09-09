#!/usr/bin/env python3
"""v15 S1 driver: deepseek 20x2 stability (brain selection).

Same battle-tested loop as matrix_v14.py (serial providers, --resume retries).
Output: matrix-v15-<provider>.jsonl

Usage:
    python matrix_v15.py [--runs 2] [--providers deepseek] [--tasks A,B]
Log: bench/results/matrix-v15.log
"""
import argparse
import os
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
LOG = HERE / "results" / "matrix-v15.log"


def log(msg):
    line = f"[{time.strftime('%H:%M:%S')}] {msg}"
    print(line, flush=True)
    LOG.parent.mkdir(parents=True, exist_ok=True)
    with open(LOG, "a", encoding="utf-8") as f:
        f.write(line + "\n")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--runs", type=int, default=2)
    ap.add_argument("--providers", default="deepseek")
    ap.add_argument("--tasks", default=None)
    ap.add_argument("--suffix", default="", help="output filename suffix, e.g. -ceiling")
    ap.add_argument("--attempts", type=int, default=4)
    a = ap.parse_args()

    providers = [p.strip() for p in a.providers.split(",") if p.strip()]
    log(f"=== MATRIX v15 START: providers={providers} runs={a.runs} "
        f"tasks={a.tasks or 'ALL'} suffix={a.suffix or '-'} ===")

    for p in providers:
        env = dict(os.environ)
        env["BENCH_PROVIDER"] = p
        env["BENCH_OUT"] = f"matrix-v15-{p}{a.suffix}.jsonl"
        env.setdefault("BENCH_TIMEOUT_MIN", "15")

        cmd = [sys.executable, "-u", str(HERE / "runner.py"), "batch",
               "--runs", str(a.runs), "--resume"]
        if a.tasks:
            cmd += ["--tasks", a.tasks]

        for attempt in range(1, a.attempts + 1):
            log(f"--- provider {p} attempt {attempt}/{a.attempts} start ---")
            t0 = time.time()
            proc = subprocess.run(
                cmd, cwd=str(HERE), env=env,
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

    log("=== MATRIX v15 DONE ===")


if __name__ == "__main__":
    main()
