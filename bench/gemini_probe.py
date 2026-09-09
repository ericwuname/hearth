#!/usr/bin/env python3
"""v15 S1b — gemini ceiling probe (runs after chain_v15's deepseek matrix).

Why: v14 concluded "T14/T19 are a model capability wall" from the fact that two
weak models both failed them. That only proves *those two* failed. If a stronger
model (gemini-3.6-flash) clears them, the wall is real and the capability
white-paper stands; if gemini fails too, the fixtures or the tool chain are
suspect and the white-paper must NOT be written.

Waits for chain_v15 to finish (so the service isn't shared), then:
  1. gemini smoke on T00-smoke (fail-fast if the VM cannot reach Google)
  2. T14/T19/T09 x2 ceiling probe

Log: bench/results/gemini-probe.log
"""
import json
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
LOG = HERE / "results" / "gemini-probe.log"
CHAIN_LOG = HERE / "results" / "chain-v15.log"
RAW = HERE / "results" / "raw"
CEILING_TASKS = "T14-add-serde,T19-merge-duplicate,T09-add-error-type"
MAX_WAIT_S = 3 * 3600


def log(msg):
    line = f"[{time.strftime('%H:%M:%S')}] {msg}"
    print(line, flush=True)
    LOG.parent.mkdir(parents=True, exist_ok=True)
    with open(LOG, "a", encoding="utf-8") as f:
        f.write(line + "\n")


def run(args, tag):
    t0 = time.time()
    p = subprocess.run([sys.executable, "-u", str(HERE / "matrix_v15.py")] + args,
                       cwd=str(HERE), stdout=subprocess.PIPE,
                       stderr=subprocess.STDOUT, text=True)
    for ln in (p.stdout or "").splitlines()[-40:]:
        log(f"  [{tag}] {ln}")
    log(f"--- {tag} rc={p.returncode} in {time.time() - t0:.0f}s ---")
    return p.returncode


def passed(fname):
    f = RAW / fname
    if not f.exists():
        return False
    for line in f.read_text(encoding="utf-8").splitlines():
        try:
            if json.loads(line).get("success"):
                return True
        except Exception:
            pass
    return False


def main():
    log("=== GEMINI PROBE: waiting for chain_v15 to finish ===")
    deadline = time.time() + MAX_WAIT_S
    while time.time() < deadline:
        if CHAIN_LOG.exists() and "chain v15 DONE" in CHAIN_LOG.read_text(encoding="utf-8"):
            log("chain_v15 finished — service is free")
            break
        time.sleep(60)
    else:
        log("!!! timed out waiting for chain_v15; proceeding anyway")

    time.sleep(20)  # grace

    log("step1: gemini smoke (T00-smoke x1)")
    run(["--providers", "gemini", "--runs", "1", "--tasks", "T00-smoke",
         "--suffix=-smoke", "--attempts", "1"], "gemini-smoke")

    if not passed("matrix-v15-gemini-smoke.jsonl"):
        log("SMOKE FAILED — gemini unusable from the VM; ceiling probe SKIPPED")
        log("=== gemini probe DONE (skipped) ===")
        return

    log(f"step2: ceiling probe {CEILING_TASKS} x2")
    run(["--providers", "gemini", "--runs", "2", "--tasks", CEILING_TASKS,
         "--suffix=-ceiling"], "gemini-ceiling")
    log("=== gemini probe DONE ===")


if __name__ == "__main__":
    main()
