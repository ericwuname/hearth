#!/usr/bin/env python3
"""v18 S4: E0-E3 experiment driver.

  E0: baseline (empty store, shuffled)
  E1: keyword + rule-constructed exp + shuffled
  E2: keyword + LLM-refined exp + shuffled
  E3: embedding + LLM-refined exp + shuffled

Service restart per phase with explicit EMBEDDING_ENABLED + store state.
Run:  V18_PHASE=E0 python bench/self-evolve-v18.py
"""
import json
import os
import random
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
RAW = HERE / "results" / "raw"
PROVIDER = os.environ.get("V17_PROVIDER", "zhipu")

VM_HOST = "192.168.220.131"
VM_USER = "wutao"
VM_PW = os.environ.get("CODEX_VM_PW", "")
REMOTE = "/home/wutao/codex_work"

# phase -> (embedding_enabled, store_action)
# store_action: "clear" | "inject-rule" | "inject-refined" | "keep"
PHASES = {
    "E0": (False, "clear"),
    "E1": (False, "inject-rule"),
    "E2": (False, "inject-refined"),
    "E3": (True, "inject-refined"),
}


def log(msg):
    print(f"[{time.strftime('%H:%M:%S')}] {msg}", flush=True)


def ssh(cmd, timeout=120):
    import paramiko
    c = paramiko.SSHClient()
    c.set_missing_host_key_policy(paramiko.AutoAddPolicy())
    c.connect(VM_HOST, username=VM_USER, password=VM_PW, timeout=30)
    _, so, se = c.exec_command(cmd, timeout=timeout)
    out = so.read().decode(errors="replace") + se.read().decode(errors="replace")
    c.close()
    return out


def restart_service(embedding: bool, store_action: str, exp_file: str | None = None):
    """Restart service with explicit embedding flag and store state."""
    # 1. prepare store file
    store_cmd = f"rm -f {REMOTE}/memory/experience.jsonl"
    if store_action.startswith("inject"):
        local = RAW / (exp_file or "v18-rule-experiences.jsonl")
        import paramiko
        c = paramiko.SSHClient()
        c.set_missing_host_key_policy(paramiko.AutoAddPolicy())
        c.connect(VM_HOST, username=VM_USER, password=VM_PW, timeout=30)
        ftp = c.open_sftp()
        with ftp.file(f"{REMOTE}/memory/experience.jsonl", "w") as f:
            f.write(local.read_bytes().decode("utf-8"))
        ftp.close()
        c.close()
        log(f"  injected store from {local.name}")
    elif store_action == "clear":
        ssh(store_cmd)
        log("  store cleared")

    # 2. restart with embedding flag
    emb = "1" if embedding else "0"
    out = ssh(
        f"pkill -x service; sleep 2; "
        f"( cd {REMOTE} && EMBEDDING_ENABLED={emb} setsid nohup "
        f"./target/debug/service > /home/wutao/service.log 2>&1 < /dev/null & disown ) "
        f"> /dev/null 2>&1; sleep 12; echo OK"
    )
    log(f"  service restarted EMBEDDING={emb}")

    # 3. verify store count
    try:
        import urllib.request
        req = urllib.request.Request("http://192.168.220.131:3000/api/v1/experience/metrics")
        resp = urllib.request.urlopen(req, timeout=10)
        m = json.loads(resp.read().decode())
        log(f"  store metrics: {m}")
    except Exception as e:
        log(f"  metrics failed: {e}")


def run_benchmark(phase: str):
    """Run 20x2 shuffled with runner.py."""
    log(f"--- {phase}: {PROVIDER} 20x2 shuffled ---")
    os.environ["BENCH_PROVIDER"] = PROVIDER
    os.environ["BENCH_OUT"] = f"matrix-v18-{phase.lower()}.jsonl"
    os.environ.setdefault("BENCH_TIMEOUT_MIN", "15")

    import runner
    import importlib
    importlib.reload(runner)

    all_tasks = sorted(d.name for d in runner.TASKS_DIR.iterdir()
                       if d.is_dir() and not d.name.startswith("_"))
    random.shuffle(all_tasks)
    log(f"  shuffled order: {all_tasks}")

    t0 = time.time()
    try:
        runner.run_batch(tasks=all_tasks, runs=2, resume=False, shuffle=False)
    except Exception as e:
        log(f"  run_batch exception: {type(e).__name__}: {e}")
    elapsed = time.time() - t0

    results = []
    jl = RAW / f"matrix-v18-{phase.lower()}.jsonl"
    if jl.exists():
        for line in jl.read_text(encoding="utf-8").splitlines():
            if line.strip():
                try:
                    results.append(json.loads(line))
                except Exception:
                    pass
    ok = sum(1 for r in results if r.get("success"))
    log(f"  [{phase}] {ok}/{len(results)} = {ok/max(len(results),1)*100:.1f}% in {elapsed:.0f}s")
    return results


def main():
    phase = os.environ.get("V18_PHASE", "E0").upper()
    if phase not in PHASES:
        log(f"unknown phase {phase}")
        sys.exit(1)
    embedding, store_action = PHASES[phase]
    exp_file = None
    if store_action == "inject-rule":
        exp_file = "v18-rule-experiences.jsonl"
    elif store_action == "inject-refined":
        exp_file = "v18-refined-experiences.jsonl"

    log(f"=== v18 {phase} START (provider={PROVIDER}) ===")
    RAW.mkdir(parents=True, exist_ok=True)

    restart_service(embedding, store_action, exp_file)
    time.sleep(3)
    results = run_benchmark(phase)
    log(f"=== v18 {phase} DONE ===")
    return 0


if __name__ == "__main__":
    sys.exit(main())
