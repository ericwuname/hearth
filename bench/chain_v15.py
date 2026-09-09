#!/usr/bin/env python3
"""v15 chain driver (S1 + S1b).

Order matters:
  0. gemini smoke (1 task x1)  -- fail fast if the VM cannot reach Google
  1. deepseek 20x2 full matrix -- brain selection (S1)
  2. gemini ceiling probe      -- T14/T19/T09 x2 (S1b), only if smoke passed

Everything runs against the CURRENT service binary. Do NOT rebuild/restart the
service while this chain is alive (the replay provider work must wait).

Log: bench/results/chain-v15.log
"""
import os
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
LOG = HERE / "results" / "chain-v15.log"
RAW = HERE / "results" / "raw"

CEILING_TASKS = "T14-add-serde,T19-merge-duplicate,T09-add-error-type"


def log(msg):
    line = f"[{time.strftime('%H:%M:%S')}] {msg}"
    print(line, flush=True)
    LOG.parent.mkdir(parents=True, exist_ok=True)
    with open(LOG, "a", encoding="utf-8") as f:
        f.write(line + "\n")


def run(cmd, env_extra=None, tag=""):
    env = dict(os.environ)
    if env_extra:
        env.update(env_extra)
    t0 = time.time()
    proc = subprocess.run(cmd, cwd=str(HERE), env=env,
                          stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    for ln in (proc.stdout or "").splitlines()[-60:]:
        log(f"  [{tag}] {ln}")
    log(f"--- {tag} rc={proc.returncode} in {time.time() - t0:.0f}s ---")
    return proc.returncode


def main():
    log("=== CHAIN v15 START ===")

    # ---- step 0: gemini smoke ----
    log("step0: gemini smoke (T00-smoke x1)")
    rc = run([sys.executable, "-u", str(HERE / "matrix_v15.py"),
              "--providers", "gemini", "--runs", "1",
              "--tasks", "T00-smoke", "--suffix=-smoke", "--attempts", "1"],
             tag="gemini-smoke")
    smoke_ok = False
    smoke_file = RAW / "matrix-v15-gemini-smoke.jsonl"
    if smoke_file.exists():
        import json
        for line in smoke_file.read_text(encoding="utf-8").splitlines():
            try:
                if json.loads(line).get("success"):
                    smoke_ok = True
            except Exception:
                pass
    log(f"step0 result: gemini reachable/usable = {smoke_ok}")

    # ---- step 1: deepseek 20x2 (the long one) ----
    log("step1: deepseek 20x2 full matrix")
    run([sys.executable, "-u", str(HERE / "matrix_v15.py"),
         "--providers", "deepseek", "--runs", "2"], tag="deepseek")

    # ---- step 2: gemini ceiling probe ----
    if smoke_ok:
        log(f"step2: gemini ceiling probe on {CEILING_TASKS} x2")
        run([sys.executable, "-u", str(HERE / "matrix_v15.py"),
             "--providers", "gemini", "--runs", "2",
             "--tasks", CEILING_TASKS, "--suffix=-ceiling"], tag="gemini-ceiling")
    else:
        log("step2 SKIPPED: gemini smoke failed (VM likely cannot reach Google)")

    log("=== chain v15 DONE ===")


if __name__ == "__main__":
    main()
