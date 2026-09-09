#!/usr/bin/env python3
"""v12.1 benchmark runner — batch serial execution with fixture upload + verify."""
import json, os, sys, time, urllib.request, urllib.error, tarfile, io, shutil, argparse, random
from pathlib import Path

SERVICE_BASE = os.environ.get("CODEX_SERVICE_URL", "http://192.168.220.131:3000")
VM_HOST = "192.168.220.131"
VM_USER = "wutao"
VM_PW = os.environ.get("CODEX_VM_PW", "")
RESULT_JSONL = Path(__file__).parent / "results" / "raw" / os.environ.get("BENCH_OUT", "baseline-v12.jsonl")
TASKS_DIR = Path(__file__).parent / "tasks"
BUDGET = int(os.environ.get("BENCH_BUDGET", "15"))
TIMEOUT_MIN = int(os.environ.get("BENCH_TIMEOUT_MIN", "30"))
RUNS = int(os.environ.get("BENCH_RUNS", "3"))
PROVIDER = os.environ.get("BENCH_PROVIDER", "doubao")

import paramiko

def ssh():
    c = paramiko.SSHClient()
    c.set_missing_host_key_policy(paramiko.AutoAddPolicy())
    c.connect(VM_HOST, username=VM_USER, password=VM_PW, timeout=30)
    return c

def api(method, path, data=None):
    url = f"{SERVICE_BASE}{path}"
    req = urllib.request.Request(url, method=method)
    req.add_header("Content-Type", "application/json")
    if data is not None:
        req.data = json.dumps(data).encode()
    resp = urllib.request.urlopen(req, timeout=10)
    raw = resp.read().decode()
    try:
        return json.loads(raw)
    except json.JSONDecodeError:
        return raw

def upload_fixture(task_name, vm_dir):
    fixture = TASKS_DIR / task_name / "fixture"
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
    # v12.4: exec_command is asynchronous — issuing `rm -rf` and `tar xzf` as two
    # separate calls let them race, occasionally wiping the freshly extracted
    # fixture. Run them as one command and block on the exit status.
    _, stdout, _ = c.exec_command(
        f"rm -rf {vm_dir} && mkdir -p {vm_dir} && "
        f"tar xzf ~/fixture.tar.gz -C {vm_dir} && echo FIXTURE_OK"
    )
    ok = "FIXTURE_OK" in stdout.read().decode(errors="replace")
    c.close()
    return ok

def run_one(task_name, run_idx):
    goal_file = TASKS_DIR / task_name / "goal.txt"
    verify_file = TASKS_DIR / task_name / "verify.sh"
    meta_file = TASKS_DIR / task_name / "meta.json"
    if not goal_file.exists():
        return {"task": task_name, "run": run_idx, "success": False, "error": "no goal.txt"}

    goal = goal_file.read_text().strip()
    meta = json.loads(meta_file.read_text()) if meta_file.exists() else {}

    # v14: per-task budget override — L4/L5 tasks were starving at the
    # global 15-step budget (v13: T14/T15/T19 all failed at steps=15).
    task_budget = int(meta.get("max_steps", BUDGET))

    # 1. Create session
    try:
        sess = api("POST", "/api/v1/sessions", {
            "provider": PROVIDER,
            "goal": goal,
            "budget": {"max_steps": task_budget},
        })
        sid = sess["session_id"]
    except Exception as e:
        return {"task": task_name, "run": run_idx, "success": False, "error": f"session create: {e}"}

    ws = f"/home/wutao/codex_work/sessions/{sid}"

    # 2. Upload fixture to session workspace
    upload_fixture(task_name, ws)

    # 3. Send message to start agent (SSE stream — fire-and-forget, timeout expected)
    try:
        api("POST", f"/api/v1/sessions/{sid}/messages", {"content": goal})
    except urllib.error.URLError:
        pass  # SSE stream close = expected
    except Exception as e:
        if "timed out" in str(e) or "Timeout" in type(e).__name__:
            pass  # Expected — SSE doesn't close until agent finishes
        else:
            return {"task": task_name, "run": run_idx, "success": False, "error": f"send_message: {e}"}

    # 4. Poll
    deadline = time.time() + TIMEOUT_MIN * 60
    t0 = time.time()
    phase = "created"
    steps = 0
    while time.time() < deadline:
        time.sleep(3)
        try:
            s = api("GET", f"/api/v1/sessions/{sid}")
            if isinstance(s, dict):
                phase = s.get("phase", phase)
                steps = s.get("steps", steps)
                if phase in ("done", "completed", "error", "cancelled"):
                    break
        except Exception:
            pass

    wall_s = round(time.time() - t0, 1)

    # 5. Verify
    success = False
    verify_out = ""
    if verify_file.exists():
        try:
            c = ssh()
            # v12.4: the verify script MUST live inside the session workspace,
            # because every verify.sh does `cd "$(dirname "$0")"`. Previously it
            # was written to /tmp, so the script cd'd into /tmp and every task
            # failed regardless of what the agent produced (0% pass rate bug).
            ftp = c.open_sftp()
            vm_verify = f"{ws}/__verify.sh"
            with ftp.file(vm_verify, "wb") as fh:
                fh.write(verify_file.read_bytes())
            ftp.close()
            _, stdout, _ = c.exec_command(
                f"cd {ws} && source ~/.cargo/env 2>/dev/null; bash __verify.sh 2>&1"
            )
            verify_out = stdout.read().decode(errors="replace")
            success = "VERIFY_PASS" in verify_out
            c.close()
        except Exception as e:
            verify_out = f"verify exec error: {e}"

    # P4 (v24-post): transcript 落盘——session 完成后导出完整事件流
    # （goal / tool_calls / results / reflection / verify 全在信封 JSONL 里）。
    # 供事后审计与失败根因分析（560-run 马拉松曾零 transcript 留存）。
    transcript_path = None
    try:
        events = api("GET", f"/api/v1/sessions/{sid}/events")
        if isinstance(events, str) and events.strip():
            transcripts_dir = Path(__file__).parent / "results" / "transcripts"
            transcripts_dir.mkdir(parents=True, exist_ok=True)
            transcript_path = transcripts_dir / f"{sid}.jsonl"
            transcript_path.write_text(events, encoding="utf-8")
    except Exception as e:
        # service 内存缓冲可能已被 TTL 清理——不阻断 run，只记录
        transcript_error = f"{e}"

    record = {
        "task": task_name,
        "run": run_idx,
        "success": success,
        "phase": phase,
        "steps": steps,
        "wall_s": wall_s,
        "provider": PROVIDER,
        "session_id": sid,
        "transcript": str(transcript_path) if transcript_path else "",
        "level": meta.get("level", "?"),
        "verify_tail": verify_out.strip()[-400:],
    }
    return record

def _completed_keys():
    """v13: read already-written results so an interrupted batch can resume.

    The v13 matrix run got killed mid-flight by the host shell being reclaimed;
    without resume the only options were 'lose 15 completed runs' or 'append
    duplicates'. Keyed by (task, run) so a partial provider file resumes exactly.
    """
    done = set()
    if RESULT_JSONL.exists():
        with open(RESULT_JSONL, encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    rec = json.loads(line)
                except json.JSONDecodeError:
                    continue
                done.add((rec.get("task"), rec.get("run")))
    return done


def run_batch(tasks=None, runs=RUNS, resume=False, shuffle=False):
    records = []
    all_tasks = sorted([d.name for d in TASKS_DIR.iterdir() if d.is_dir() and (d / "goal.txt").exists()])
    if tasks:
        all_tasks = [t for t in all_tasks if t in tasks]
    if shuffle:
        random.shuffle(all_tasks)
        print(f"SHUFFLED task order: {all_tasks}")

    skip = _completed_keys() if resume else set()
    total = len(all_tasks) * runs
    done = 0
    print(f"BENCH: {len(all_tasks)} tasks x {runs} runs = {total} total, provider={PROVIDER}")
    if skip:
        print(f"RESUME: {len(skip)} (task,run) pairs already in {RESULT_JSONL.name}, skipping them")

    for task in all_tasks:
        for r in range(runs):
            done += 1
            if (task, r) in skip:
                print(f"[{done}/{total}] {task} run={r} ... SKIP (already done)", flush=True)
                continue
            print(f"[{done}/{total}] {task} run={r} ...", end=" ", flush=True)
            rec = run_one(task, r)
            records.append(rec)
            status = "PASS" if rec["success"] else f"FAIL({rec.get('phase','?')})"
            print(f"{status} {rec.get('wall_s','?')}s")
            RESULT_JSONL.parent.mkdir(parents=True, exist_ok=True)
            with open(RESULT_JSONL, "a") as f:
                f.write(json.dumps(rec) + "\n")

    # Summary
    ok = sum(1 for r in records if r["success"])
    print(f"\nSUMMARY: {ok}/{total} passed ({round(100*ok/total,1) if total else 0}%)")
    return records

def preflight():
    print("=== PREFLIGHT ===")
    print(f"  Check service: {SERVICE_BASE}")
    try:
        r = api("GET", "/readyz")
        print(f"  service OK: {str(r)[:40]}")
    except Exception as e:
        print(f"  service DOWN: {e}")
        return False

    print(f"  Tasks found: {len([d for d in TASKS_DIR.iterdir() if d.is_dir() and (d/'goal.txt').exists()])}")
    print(f"  Provider: {PROVIDER}")
    print(f"  Budget: {BUDGET} steps, Timeout: {TIMEOUT_MIN}min")
    return True

if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("cmd", nargs="?", default="run", choices=["run", "batch", "preflight", "single"])
    ap.add_argument("--tasks", default=None, help="comma-separated task names")
    ap.add_argument("--runs", type=int, default=RUNS, help="runs per task")
    ap.add_argument("--resume", action="store_true",
                    help="skip (task,run) pairs already present in the output jsonl")
    ap.add_argument("--shuffle", action="store_true",
                    help="v18: randomize task order (eliminate order effects)")
    ap.add_argument("task", nargs="?", default=None)
    a = ap.parse_args()

    if a.cmd == "preflight":
        sys.exit(0 if preflight() else 1)
    elif a.cmd == "single" or a.cmd == "run":
        t = a.task or "T00-smoke"
        rec = run_one(t, 0)
        RESULT_JSONL.parent.mkdir(parents=True, exist_ok=True)
        with open(RESULT_JSONL, "a") as f:
            f.write(json.dumps(rec) + "\n")
        print(json.dumps(rec, indent=2))
    elif a.cmd == "batch":
        tl = a.tasks.split(",") if a.tasks else None
        run_batch(tasks=tl, runs=a.runs, resume=a.resume, shuffle=a.shuffle)
