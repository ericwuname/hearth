#!/usr/bin/env python3
"""v17 S1+S2: Self-evolution experiment — simplified.

Uses the auto-generated experiences from the first run (agent writes them
via do_reflect). No external LLM needed. Compares "no experience" vs
"with auto-generated experience" pass rates across two consecutive runs.

Flow:
  1. Clear experience store → run deepseek 20×2 baseline (writes auto-experiences)
  2. Keep the auto-experiences in store
  3. Run deepseek 20×2 again (agent finds experiences via search)
  4. Compare pass rates

NOTE: deepseek API key is 402 (no credits). Falls back to zhipu.
"""
import json, os, subprocess, sys, time
from pathlib import Path

HERE = Path(__file__).resolve().parent
RAW = HERE / "results" / "raw"
OUT = HERE / "results" / "self-evolve-v17.jsonl"
PROVIDER = os.environ.get("V17_PROVIDER", "zhipu")

# Use zhipu for both benchmark and condensing (deepseek is 402)
LLM_API = False  # Skip LLM condensing — use auto-experiences only

VM_HOST = "192.168.220.131"
VM_USER = "wutao"
VM_PW = os.environ.get("CODEX_VM_PW", "")
REMOTE = "/home/wutao/codex_work"


def log(msg):
    print(f"[{time.strftime('%H:%M:%S')}] {msg}", flush=True)


def ssh_clear_and_restart():
    """Clear experience store and restart service cleanly."""
    import paramiko
    c = paramiko.SSHClient()
    c.set_missing_host_key_policy(paramiko.AutoAddPolicy())
    c.connect(VM_HOST, username=VM_USER, password=VM_PW, timeout=30)
    _, so, _ = c.exec_command(
        f"rm -f {REMOTE}/memory/experience.jsonl && "
        f"pkill -x service; sleep 2; "
        f"( cd {REMOTE} && setsid nohup ./target/debug/service "
        f"> /home/wutao/service.log 2>&1 < /dev/null & disown ) "
        f"> /dev/null 2>&1; sleep 8; echo OK"
    )
    c.close()
    return so.read().decode(errors="replace").strip()


def get_metrics():
    """GET experience store metrics."""
    import urllib.request
    try:
        req = urllib.request.Request("http://192.168.220.131:3000/api/v1/experience/metrics")
        resp = urllib.request.urlopen(req, timeout=10)
        return json.loads(resp.read().decode())
    except Exception as e:
        return {"error": str(e)}


def run_benchmark(label: str) -> list[dict]:
    """Run 20×2 by calling runner.run_batch directly (no subprocess — avoids
    stdio pipe deadlock on long runs). Returns list of result dicts."""
    log(f"--- {label}: {PROVIDER} 20×2 (in-process) ---")
    
    # Set env BEFORE importing runner so RESULT_JSONL resolves correctly
    os.environ["BENCH_PROVIDER"] = PROVIDER
    os.environ["BENCH_OUT"] = f"matrix-v17-{label}.jsonl"
    os.environ.setdefault("BENCH_TIMEOUT_MIN", "15")
    
    import runner
    import importlib
    importlib.reload(runner)  # re-eval RESULT_JSONL with new BENCH_OUT
    
    all_tasks = sorted(d.name for d in runner.TASKS_DIR.iterdir()
                       if d.is_dir() and not d.name.startswith("_"))
    
    t0 = time.time()
    try:
        runner.run_batch(tasks=all_tasks, runs=2, resume=True)
    except KeyboardInterrupt:
        log("interrupted")
    except Exception as e:
        log(f"run_batch exception: {type(e).__name__}: {e}")
    elapsed = time.time() - t0
    
    # Parse results from JSONL
    results = []
    jsonl_path = RAW / f"matrix-v17-{label}.jsonl"
    if jsonl_path.exists():
        for line in jsonl_path.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if line:
                try:
                    results.append(json.loads(line))
                except json.JSONDecodeError:
                    pass
    
    ok = sum(1 for r in results if r.get("success"))
    log(f"  [{label}] {ok}/{len(results)} passed ({ok/max(len(results),1)*100:.1f}%) "
        f"in {elapsed:.0f}s")
    return results


def main():
    log(f"=== SELF-EVOLVE v17 START (provider={PROVIDER}) ===")
    RAW.mkdir(parents=True, exist_ok=True)
    
    mode = sys.argv[1] if len(sys.argv) > 1 else "auto"
    
    if mode == "resume-baseline":
        # Only complete the baseline (existing 35/40 + resume remaining)
        log("\n┌─ RESUME baseline ─┐")
        baseline = run_benchmark("baseline")
        m = get_metrics()
        log(f"experience store: {m}")
        ok = sum(1 for r in baseline if r.get("success"))
        log(f"baseline total: {ok}/{len(baseline)}")
        log("=== RESUME DONE ===")
        return
    
    # ── Phase 1: Clear store + baseline ──
    log("\n┌─ Phase 1: clean start + baseline ─┐")
    log(f"ssh clear: {ssh_clear_and_restart()}")
    m = get_metrics()
    log(f"experience store before baseline: {m}")
    
    baseline = run_benchmark("baseline")
    
    # Check auto-experiences written
    m = get_metrics()
    log(f"experience store after baseline: {m}")
    
    if not baseline:
        log("ERROR: baseline returned no results. Aborting.")
        sys.exit(1)
    
    fails = [r for r in baseline if not r.get("success")]
    log(f"baseline fails: {len(fails)}/{len(baseline)}")
    
    # ── Phase 2: Restart service (to ensure clean state) ──
    # KEEP the auto-generated experiences — don't clear
    log("\n┌─ Phase 2: restart service (keep experiences) ─┐")
    import paramiko
    c = paramiko.SSHClient()
    c.set_missing_host_key_policy(paramiko.AutoAddPolicy())
    c.connect(VM_HOST, username=VM_USER, password=VM_PW, timeout=30)
    _, so, _ = c.exec_command(
        f"pkill -x service; sleep 2; "
        f"( cd {REMOTE} && setsid nohup ./target/debug/service "
        f"> /home/wutao/service.log 2>&1 < /dev/null & disown ) "
        f"> /dev/null 2>&1; sleep 8; echo OK"
    )
    log(f"restart: {so.read().decode(errors='replace').strip()}")
    c.close()
    
    m = get_metrics()
    log(f"experience store after restart: {m}")
    
    # ── Phase 3: Injection run ──
    log("\n┌─ Phase 3: injection run (with experiences) ─┐")
    injection = run_benchmark("injection")
    
    m = get_metrics()
    log(f"experience store after injection run: {m}")
    
    # ── Phase 4: Compare ──
    log("\n┌─ Phase 4: comparison ─┐")
    base_ok = sum(1 for r in baseline if r.get("success"))
    base_n = len(baseline)
    inj_ok = sum(1 for r in injection if r.get("success"))
    inj_n = len(injection)
    base_pct = base_ok / base_n * 100 if base_n else 0
    inj_pct = inj_ok / inj_n * 100 if inj_n else 0
    delta = inj_pct - base_pct
    
    log(f"baseline:    {base_ok}/{base_n} = {base_pct:.1f}%")
    log(f"injection:   {inj_ok}/{inj_n} = {inj_pct:.1f}%")
    log(f"delta:       {delta:+.1f} pt")
    
    # Per-task comparison
    by_task = {}
    for r in baseline:
        by_task[(r.get("task"), r.get("run"))] = {"baseline": r.get("success", False)}
    for r in injection:
        k = (r.get("task"), r.get("run"))
        if k in by_task:
            by_task[k]["injection"] = r.get("success", False)
        else:
            by_task[k] = {"injection": r.get("success", False)}
    
    improved = sum(1 for v in by_task.values()
                   if v.get("baseline") is False and v.get("injection") is True)
    regressed = sum(1 for v in by_task.values()
                    if v.get("baseline") is True and v.get("injection") is False)
    
    log(f"improved:  {improved} tasks")
    log(f"regressed: {regressed} tasks")
    
    final = {
        "provider": PROVIDER,
        "baseline_ok": base_ok, "baseline_n": base_n, "baseline_pct": round(base_pct, 1),
        "injection_ok": inj_ok, "injection_n": inj_n, "injection_pct": round(inj_pct, 1),
        "delta_pt": round(delta, 1),
        "improved": improved, "regressed": regressed,
        "experiences_after_baseline": get_metrics().get("total_experiences", 0),
    }
    with open(OUT, "w", encoding="utf-8") as f:
        f.write(json.dumps(final, ensure_ascii=False) + "\n")
    
    # Detailed per-task table
    log("\n┌─ Per-task changes ─┐")
    log(f"{'task':20s} {'run':3s} {'baseline':10s} {'injection':10s} {'change':10s}")
    log("-" * 55)
    for (t, r), v in sorted(by_task.items()):
        b = "PASS" if v.get("baseline") else "FAIL"
        i = "PASS" if v.get("injection") else "FAIL"
        ch = "✅ improved" if (v.get("baseline") is False and v.get("injection") is True) else \
              "🔻 regressed" if (v.get("baseline") is True and v.get("injection") is False) else \
              "—"
        if ch != "—":
            log(f"{t:20s} {r:<3} {b:10s} {i:10s} {ch}")
    
    log(f"\nwrote {OUT}")
    log("=== SELF-EVOLVE v17 DONE ===")


if __name__ == "__main__":
    main()
