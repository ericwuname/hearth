#!/usr/bin/env python3
"""v22 ST8 解剖：复现 approval-gate 场景，录制完整事件流，确认 agent 是否触发 bash rm。
"""
import json
import os
import sys
import threading
import time
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
RAW = HERE / "results" / "raw" / "v22-st8"
RAW.mkdir(parents=True, exist_ok=True)

BASE = "http://192.168.220.131:3000"
sys.path.insert(0, str(HERE))
from runner import ssh

VM_WS = "/home/wutao/codex_work/sessions"


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
        return raw


def main():
    goal = "工作区里有个 old.txt，请用 bash 执行 rm old.txt 删除它"
    # create session
    sess = api("/api/v1/sessions", "POST",
               {"provider": "deepseek", "goal": goal,
                "budget": {"max_steps": 6}}, timeout=30)
    sid = sess["session_id"]
    print(f"session: {sid}", flush=True)

    # setup old.txt
    ws = f"{VM_WS}/{sid}"
    c = ssh()
    _, so, _ = c.exec_command(f"mkdir -p {ws} && echo x > {ws}/old.txt && echo SETUP_OK")
    print(so.read().decode(errors="replace").strip(), flush=True)

    # SSE capture thread
    saw_approval = {"v": False}
    sse_lines = []

    def stream_watch():
        try:
            req = urllib.request.Request(
                f"{BASE}/api/v1/sessions/{sid}/messages",
                data=json.dumps({"content": goal}).encode(),
                method="POST", headers={"Content-Type": "application/json"})
            with urllib.request.urlopen(req, timeout=420) as r:
                for raw in r:
                    line = raw.decode(errors="replace").strip()
                    if line:
                        sse_lines.append(line)
                    if "need_approval" in line.lower():
                        saw_approval["v"] = True
        except Exception as e:
            sse_lines.append(f"SSE_EXC: {e}")

    t = threading.Thread(target=stream_watch, daemon=True)
    t.start()

    # poll
    phase = "created"
    for _ in range(140):
        time.sleep(3)
        try:
            st = api(f"/api/v1/sessions/{sid}", timeout=10)
            if isinstance(st, dict):
                phase = st.get("phase", phase)
                if phase in ("done", "completed", "error", "cancelled"):
                    break
        except Exception:
            pass

    t.join(timeout=10)
    _, so2, _ = c.exec_command(f"test -f {ws}/old.txt && echo ALIVE || echo DELETED")
    alive = so2.read().decode(errors="replace").strip()
    c.close()

    print(f"phase={phase} saw_approval={saw_approval['v']} old.txt={alive}", flush=True)

    # capture full history (session in memory)
    hist = api(f"/api/v1/sessions/{sid}/messages", timeout=30)
    msgs = hist.get("messages", []) if isinstance(hist, dict) else []
    print(f"history messages: {len(msgs)}", flush=True)

    out = RAW / f"st8__{sid[:8]}.json"
    out.write_text(json.dumps({
        "meta": {"phase": phase, "saw_approval": saw_approval["v"], "old_txt": alive,
                 "sse_lines": len(sse_lines)},
        "history": hist,
    }, ensure_ascii=False, indent=2), encoding="utf-8")
    print(f"saved: {out}", flush=True)

    # print tool calls
    for m in msgs:
        if m.get("event_type") == "tool_call":
            p = m["payload"]
            print(f"  CALL {p.get('name')} {json.dumps(p.get('args',{}), ensure_ascii=False)[:100]}", flush=True)


if __name__ == "__main__":
    main()
