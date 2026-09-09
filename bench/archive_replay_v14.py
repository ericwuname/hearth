#!/usr/bin/env python3
"""v14 line C (scoped by plan v2 R1): archive PASS session transcripts as replay fixtures.

Sessions live in service memory only — this MUST run after S4 finishes and
BEFORE the service is restarted (the stress field may restart it).

For every success=true record in matrix-v14-zhipu.jsonl:
  GET /api/v1/sessions/{sid}/messages  (B1 history replay route)
  -> bench/replay/fixtures/<task>__run<k>__<sid8>.json
     { meta: {task, run, provider, steps, wall_s}, history: <full transcript> }

replay.py + ReplayProvider are v15 scope. These fixtures are the recording.
"""
import json
import os
import urllib.request
from pathlib import Path

BASE = os.environ.get("CODEX_SERVICE_URL", "http://192.168.220.131:3000")
HERE = Path(__file__).resolve().parent
RAW = HERE / "results" / "raw" / "matrix-v14-zhipu.jsonl"
OUT = HERE / "replay" / "fixtures"


def api_get(path, timeout=30):
    with urllib.request.urlopen(f"{BASE}{path}", timeout=timeout) as r:
        return json.loads(r.read().decode() or "{}")


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    if not RAW.exists():
        raise SystemExit(f"no raw results at {RAW}")

    # dedupe by (task, run) keeping the last record (resume reruns append)
    latest = {}
    for line in RAW.read_text(encoding="utf-8").splitlines():
        try:
            r = json.loads(line)
            latest[(r["task"], r["run"])] = r
        except Exception:
            continue

    okc, failc = 0, 0
    for (task, run), r in sorted(latest.items()):
        if not r.get("success"):
            continue
        sid = r.get("session_id")
        if not sid:
            continue
        name = f"{task}__run{run}__{sid[:8]}.json"
        dest = OUT / name
        if dest.exists():
            print(f"[skip] {name}")
            okc += 1
            continue
        try:
            history = api_get(f"/api/v1/sessions/{sid}/messages")
            fixture = {
                "meta": {
                    "task": task, "run": run,
                    "provider": r.get("provider"), "steps": r.get("steps"),
                    "wall_s": r.get("wall_s"), "session_id": sid,
                    "source": "matrix-v14-zhipu", "fixture_schema": 1,
                },
                "history": history,
            }
            dest.write_text(json.dumps(fixture, ensure_ascii=False, indent=1),
                            encoding="utf-8")
            n_msgs = len(history.get("messages", history if isinstance(history, list) else []))
            print(f"[ok] {name} msgs={n_msgs}")
            okc += 1
        except Exception as e:
            print(f"[FAIL] {name}: {e}")
            failc += 1
    print(f"archived={okc} failed={failc}")


if __name__ == "__main__":
    main()
