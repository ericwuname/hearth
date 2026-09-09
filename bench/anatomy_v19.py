#!/usr/bin/env python3
"""v19 S1/S2: Anatomy runner for T13/T19 — run a single task then IMMEDIATELY
capture the full session transcript (GET /messages while the session is still
in memory; after service restart only meta+done remains).

Usage:
    python bench/anatomy_v19.py T13-fix-index T19-merge-duplicate
"""
import json
import io
import os
import sys
import tarfile
import time
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
RAW = HERE / "results" / "raw" / "v19-anatomy"
RAW.mkdir(parents=True, exist_ok=True)

BASE = "http://192.168.220.131:3000"
PROVIDER = os.environ.get("BENCH_PROVIDER", "deepseek")

sys.path.insert(0, str(HERE))
from runner import ssh  # reuse the VM ssh helper


def upload_fixture(task: str, vm_dir: str) -> bool:
    fixture = HERE / "tasks" / task / "fixture"
    if not fixture.is_dir():
        return False
    buf = io.BytesIO()
    tf = tarfile.open(fileobj=buf, mode="w:gz")
    for f in fixture.rglob("*"):
        if f.is_file():
            tf.add(str(f), str(f.relative_to(fixture)))
    tf.close()
    c = ssh()
    ftp = c.open_sftp()
    ftp.putfo(io.BytesIO(buf.getvalue()), "fixture.tar.gz")
    ftp.close()
    _, stdout, _ = c.exec_command(
        f"rm -rf {vm_dir} && mkdir -p {vm_dir} && "
        f"tar xzf ~/fixture.tar.gz -C {vm_dir} && echo FIXTURE_OK"
    )
    ok = "FIXTURE_OK" in stdout.read().decode(errors="replace")
    c.close()
    return ok


def api(path, method="GET", payload=None, timeout=10):
    data = json.dumps(payload).encode() if payload else None
    req = urllib.request.Request(
        f"{BASE}{path}", data=data, method=method,
        headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=timeout) as r:
        raw = r.read().decode() or "{}"
    try:
        return json.loads(raw)
    except json.JSONDecodeError:
        return raw  # SSE stream or plain text — caller handles


def run_and_capture(task: str):
    print(f"\n=== {task} ({PROVIDER}) ===", flush=True)
    goal_file = HERE / "tasks" / task / "goal.txt"
    goal = goal_file.read_text(encoding="utf-8").strip()

    # 1. create session
    sess = api("/api/v1/sessions", "POST",
               {"provider": PROVIDER, "goal": goal,
                "budget": {"max_steps": 20}}, timeout=30)
    if not isinstance(sess, dict) or "session_id" not in sess:
        print(f"  ERROR creating session: {sess}", flush=True)
        return None
    sid = sess["session_id"]
    print(f"  session: {sid}", flush=True)

    # 1b. upload fixture to session workspace (same as runner.py)
    ws = f"/home/wutao/codex_work/sessions/{sid}"
    upload_fixture(task, ws)
    print(f"  fixture uploaded to {ws}", flush=True)

    # 2. send message — SSE stream: fire-and-forget, expect timeout/raw
    try:
        raw = api(f"/api/v1/sessions/{sid}/messages", "POST", {"content": goal}, timeout=5)
        print(f"  POST returned: {str(raw)[:80]}", flush=True)
    except Exception as e:
        print(f"  POST (expected for SSE): {type(e).__name__}", flush=True)

    # 3. poll status until done
    phase, steps = "?", 0
    for _ in range(240):  # up to 40 min
        try:
            st = api(f"/api/v1/sessions/{sid}", timeout=10)
            if isinstance(st, dict):
                phase = st.get("phase", "?")
                steps = st.get("steps", 0)
                if phase in ("done", "error"):
                    print(f"  finished: phase={phase} steps={steps}", flush=True)
                    break
        except Exception as e:
            pass
        time.sleep(10)

    # 4. IMMEDIATELY capture full history (session still in memory)
    hist = api(f"/api/v1/sessions/{sid}/messages", timeout=30)
    msgs = hist.get("messages", []) if isinstance(hist, dict) else []
    print(f"  captured {len(msgs)} messages", flush=True)
    out = RAW / f"{task}__{PROVIDER}__{sid[:8]}.json"
    out.write_text(json.dumps({
        "meta": {"task": task, "provider": PROVIDER, "session_id": sid,
                 "phase": phase, "steps": steps},
        "history": hist,
    }, ensure_ascii=False, indent=2), encoding="utf-8")
    print(f"  saved: {out}", flush=True)
    return out


def main():
    tasks = sys.argv[1:] or ["T13-fix-index", "T19-merge-duplicate"]
    for t in tasks:
        try:
            run_and_capture(t)
        except Exception as e:
            print(f"  ERROR {t}: {e}", flush=True)
    print("\n=== ANATOMY DONE ===")


if __name__ == "__main__":
    main()
