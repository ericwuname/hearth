#!/usr/bin/env python3
"""v15 line B — deterministic replay driver.

Why this is NOT "run the benchmark again":
    The service must be started with REPLAY_DIR=<fixtures> so that the provider
    named `replay` serves the *recorded* assistant turns. The LLM is pinned;
    only the harness (tools, state machine, budget, approval, sandbox) actually
    runs. A drop in the pass rate therefore means a harness regression, not a
    bad dice roll from the model.

Per fixture we:
    1. create a session with provider=replay and the recorded goal, plus an
       explicit `[replay-fixture: <stem>]` marker (two runs of one task share a
       goal, so the marker is what pins the exact recording),
    2. upload the same task fixture workspace the original run had,
    3. drive the agent, then run the task's verify.sh,
    4. compare against the recording: it was a PASS, so replay must PASS too.

Output: bench/results/raw/replay-v15.jsonl + console summary.
"""
import json
import os
import sys
import time
import urllib.error
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import runner  # noqa: E402  (reuse api/ssh/upload_fixture/verify plumbing)

FIXTURES = HERE / "replay" / "fixtures"
OUT = HERE / "results" / "raw" / "replay-v15.jsonl"
TIMEOUT_MIN = int(os.environ.get("REPLAY_TIMEOUT_MIN", "6"))


def log(msg):
    print(f"[{time.strftime('%H:%M:%S')}] {msg}", flush=True)


def replay_one(fx_path: Path):
    stem = fx_path.stem
    d = json.loads(fx_path.read_text(encoding="utf-8"))
    meta = d.get("meta", {})
    task = meta.get("task", "?")
    goal = d.get("history", {}).get("goal", "").strip()
    if not goal:
        return {"fixture": stem, "task": task, "ok": False, "why": "no goal in fixture"}

    marked_goal = f"{goal}\n\n[replay-fixture: {stem}]"

    # 1. session with the replay provider
    try:
        sess = runner.api("POST", "/api/v1/sessions", {
            "provider": "replay",
            "goal": marked_goal,
            "budget": {"max_steps": max(int(meta.get("steps", 20)) + 5, 25)},
        })
        sid = sess["session_id"]
    except Exception as e:
        return {"fixture": stem, "task": task, "ok": False, "why": f"session create: {e}"}

    ws = f"/home/wutao/codex_work/sessions/{sid}"

    # 2. same starting workspace as the original run
    if not runner.upload_fixture(task, ws):
        return {"fixture": stem, "task": task, "ok": False, "why": "fixture upload failed",
                "session_id": sid}

    # 3. drive
    t0 = time.time()
    try:
        runner.api("POST", f"/api/v1/sessions/{sid}/messages", {"content": marked_goal})
    except urllib.error.URLError:
        pass
    except Exception as e:
        if "timed out" not in str(e) and "Timeout" not in type(e).__name__:
            return {"fixture": stem, "task": task, "ok": False, "why": f"send: {e}",
                    "session_id": sid}

    deadline = time.time() + TIMEOUT_MIN * 60
    phase, steps = "created", 0
    while time.time() < deadline:
        time.sleep(2)
        try:
            s = runner.api("GET", f"/api/v1/sessions/{sid}")
            if isinstance(s, dict):
                phase = s.get("phase", phase)
                steps = s.get("steps", steps)
                if phase in ("done", "completed", "error", "cancelled"):
                    break
        except Exception:
            pass
    wall_s = round(time.time() - t0, 1)

    # 4. verify with the task's own script
    verify_file = runner.TASKS_DIR / task / "verify.sh"
    success, verify_out = False, ""
    if verify_file.exists():
        try:
            c = runner.ssh()
            ftp = c.open_sftp()
            with ftp.file(f"{ws}/__verify.sh", "wb") as fh:
                fh.write(verify_file.read_bytes())
            ftp.close()
            _, stdout, _ = c.exec_command(
                f"cd {ws} && source ~/.cargo/env 2>/dev/null; bash __verify.sh 2>&1")
            verify_out = stdout.read().decode(errors="replace")
            success = "VERIFY_PASS" in verify_out
            c.close()
        except Exception as e:
            verify_out = f"verify exec error: {e}"

    return {
        "fixture": stem,
        "task": task,
        "ok": success,                       # recording was a PASS -> replay must PASS
        "phase": phase,
        "steps": steps,
        "recorded_steps": meta.get("steps"),
        "wall_s": wall_s,
        "recorded_wall_s": meta.get("wall_s"),
        "session_id": sid,
        "verify_tail": verify_out.strip()[-300:],
    }


def main():
    only = sys.argv[1] if len(sys.argv) > 1 else None
    fixtures = sorted(FIXTURES.glob("*.json"))
    if only:
        fixtures = [f for f in fixtures if only in f.stem]
    if not fixtures:
        log("no fixtures found")
        return 1

    log(f"=== REPLAY v15: {len(fixtures)} fixtures, provider=replay ===")
    OUT.parent.mkdir(parents=True, exist_ok=True)
    done = set()
    if OUT.exists():
        for line in OUT.read_text(encoding="utf-8").splitlines():
            try:
                done.add(json.loads(line)["fixture"])
            except Exception:
                pass

    results = []
    for i, fx in enumerate(fixtures, 1):
        if fx.stem in done:
            log(f"[{i}/{len(fixtures)}] {fx.stem} SKIP (done)")
            continue
        log(f"[{i}/{len(fixtures)}] {fx.stem} ...")
        rec = replay_one(fx)
        results.append(rec)
        with open(OUT, "a", encoding="utf-8") as f:
            f.write(json.dumps(rec) + "\n")
        log(f"    -> {'PASS' if rec.get('ok') else 'FAIL'} "
            f"steps={rec.get('steps')} (rec {rec.get('recorded_steps')}) "
            f"{rec.get('wall_s')}s {rec.get('why', '')}")

    ok = sum(1 for r in results if r.get("ok"))
    log(f"=== REPLAY DONE: {ok}/{len(results)} passed ===")
    return 0


if __name__ == "__main__":
    sys.exit(main())
