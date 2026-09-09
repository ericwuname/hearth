#!/usr/bin/env python3
"""P2 Node 05/06: collect A/B paired experiment results."""
import json, re, os, sys

EXPECTED = ["1500","3500","5500","7500","9500","11500","13500","15500","17500","19500","21500","23500"]
RUNS = [f"{c}{i}" for i in range(1,6) for c in ("A","B")]  # A1,B1,A2,B2,... interleaved execution order

def read(p):
    try:
        with open(p, encoding="utf-8", errors="replace") as f:
            return f.read()
    except Exception:
        return None

def parse_run(name):
    log = read(f"/home/wutao/fa/ab_{name}.log") or ""
    result = read(f"/tmp/p2_ab_{name}/RESULT.txt")
    # timestamps
    ts = re.findall(r"(\d{4}-\d{2}-\d{2}T[0-9:.+\-]+)", log)
    start, end = (ts[0], ts[-1]) if len(ts) >= 2 else (None, None)
    # terminal
    m = re.findall(r"Task (completed|failed)", log)
    terminal = m[-1] if m else "unknown"
    reason = "completed" if terminal == "completed" else None
    if terminal != "completed":
        for pat in ["budget_exhausted", "deadline exceeded", "stalled", "give_up", "verify_failed"]:
            if pat in log:
                reason = pat
                break
        if not reason:
            reason = "failed(unknown)"
    m = re.findall(r"（(\d+) 步）", log)
    steps = int(m[-1]) if m else -1
    m = re.findall(r"tokens: ↑(\d+) ↓(\d+) \(calls=(\d+)\)", log)
    tokens = int(m[-1][0]) + int(m[-1][1]) if m else -1
    calls = int(m[-1][2]) if m else -1
    # seq re-run check: count distinct bash seq invocations
    seq_calls = len(re.findall(r"cmd: seq \d+ \d+", log))
    # RESULT correctness
    if result is None:
        correctness = "missing"
    else:
        got = [l.strip() for l in result.strip().splitlines() if l.strip()]
        correctness = "correct" if got == EXPECTED else f"wrong({len(got)})"
    return {
        "run": name, "start": start, "end": end, "terminal": terminal,
        "reason": reason, "steps": steps, "tokens": tokens, "llm_calls": calls,
        "seq_calls": seq_calls, "result_lines": result,
        "correctness": correctness,
    }

out = [parse_run(r) for r in RUNS]

# compaction attribution: parse compacted.jsonl, count turns whose first message
# created_at falls inside each run window
arch = read("/home/wutao/.config/hearth/archive/compacted.jsonl") or ""
archived_turns = []
for line in arch.splitlines():
    try:
        t = json.loads(line)
        msgs = t.get("messages", [])
        ca = msgs[0]["created_at"] if msgs else None
        n = len(msgs)
        archived_turns.append((ca, n))
    except Exception:
        pass

from datetime import datetime, timezone
def to_utc(ts):
    try:
        ts = ts.strip()
        if ts.endswith("Z"):
            ts = ts[:-1] + "+00:00"
        return datetime.fromisoformat(ts).astimezone(timezone.utc)
    except Exception:
        return None
arch_utc = [(to_utc(ca), n) for ca, n in archived_turns]
for r in out:
    s_utc, e_utc = to_utc(r["start"]), to_utc(r["end"])
    if s_utc and e_utc:
        r["compacted_turns"] = sum(1 for ca, _ in arch_utc if ca and s_utc <= ca <= e_utc)
    else:
        r["compacted_turns"] = -1

# summary
print(json.dumps(out, ensure_ascii=False, indent=1))
a = [r for r in out if r["run"].startswith("A")]
b = [r for r in out if r["run"].startswith("B")]
def stats(rs, field):
    vals = [r[field] for r in rs if isinstance(r[field], (int, float)) and r[field] >= 0]
    if not vals:
        return "n/a"
    return f"mean={sum(vals)/len(vals):.1f} min={min(vals)} max={max(vals)}"
print("\n=== SUMMARY ===")
print("A terminal:", [f"{r['run']}:{r['terminal']}({r['reason']})" for r in a])
print("B terminal:", [f"{r['run']}:{r['terminal']}({r['reason']})" for r in b])
print("A correctness:", [f"{r['run']}:{r['correctness']}" for r in a])
print("B correctness:", [f"{r['run']}:{r['correctness']}" for r in b])
print("A steps:", stats(a, "steps"), "| B steps:", stats(b, "steps"))
print("A tokens:", stats(a, "tokens"), "| B tokens:", stats(b, "tokens"))
print("A compacted_turns:", [r["compacted_turns"] for r in a])
print("B compacted_turns:", [r["compacted_turns"] for r in b])
