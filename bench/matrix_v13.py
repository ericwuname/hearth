#!/usr/bin/env python3
"""v13 S4 driver: cross-provider contrast matrix (zhipu / deepseek / agnes), 20 tasks x N runs.

Runs providers strictly serially so CPU contention on the VM cannot skew
`verify.sh` timings between providers. Every provider batch is invoked with
--resume so an interrupted matrix picks up exactly where it stopped instead of
re-burning tokens on already-completed tasks.

Usage:
    python matrix_v13.py [--runs 1] [--providers zhipu,deepseek,agnes]
Log: bench/results/matrix-v13.log   Results: bench/results/raw/matrix-v13-<provider>.jsonl
"""
import argparse
import os
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
LOG = HERE / "results" / "matrix-v13.log"


def log(msg):
    line = f"[{time.strftime('%H:%M:%S')}] {msg}"
    print(line, flush=True)
    LOG.parent.mkdir(parents=True, exist_ok=True)
    with open(LOG, "a", encoding="utf-8") as f:
        f.write(line + "\n")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--runs", type=int, default=1)
    ap.add_argument("--providers", default="zhipu,deepseek,agnes")
    ap.add_argument("--attempts", type=int, default=4,
                    help="max resume attempts per provider when the batch dies early")
    a = ap.parse_args()

    providers = [p.strip() for p in a.providers.split(",") if p.strip()]
    log(f"=== MATRIX v13 START: providers={providers} runs={a.runs} ===")

    for p in providers:
        env = dict(os.environ)
        env["BENCH_PROVIDER"] = p
        env["BENCH_OUT"] = f"matrix-v13-{p}.jsonl"
        env.setdefault("BENCH_TIMEOUT_MIN", "15")

        # The v13 matrix was killed twice mid-batch with STATUS_CONTROL_C_EXIT
        # (0xC000013A) from outside the process. Rather than lose the run, retry
        # with --resume: completed (task,run) pairs are skipped, so each attempt
        # only pays for what is genuinely missing. Stop as soon as one attempt
        # exits cleanly.
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

    log("=== MATRIX v13 DONE ===")


if __name__ == "__main__":
    main()
