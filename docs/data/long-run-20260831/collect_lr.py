#!/usr/bin/env python3
"""P2-LR Node 14: Reliability Matrix collector."""
import json, re, os
from datetime import datetime, timezone

LOGS = {
    "n06_c_probe": "lr_n06.log",
    "n08a_compact": "lr_n08a.log",
    "n08b_resume": "lr_n08b.log",
    "n09_taskA": "lr_n09.log",
    "n10_taskB": "lr_n10.log",
    "n11_qa": "lr_n11.log",
    "n12r1": "lr_n12r1.log",
    "n12r2": "lr_n12r2.log",
    "n13a_stress1": "lr_n13a.log",
    "n13b_stress2": "lr_n13b.log",
}
BASE = "/home/wutao/fa"

def read(p):
    try:
        with open(p, encoding="utf-8", errors="replace") as f:
            return f.read()
    except Exception:
        return None

def to_utc(ts):
    try:
        ts = ts.strip()
        if ts.endswith("Z"):
            ts = ts[:-1] + "+00:00"
        return datetime.fromisoformat(ts).astimezone(timezone.utc)
    except Exception:
        return None

out = {}
for name, fn in LOGS.items():
    log = read(os.path.join(BASE, fn)) or ""
    r = {"log": fn, "exists": bool(log)}
    m = re.findall(r"Task (completed|failed)", log)
    r["terminal"] = m[-1] if m else None
    m = re.findall(r"（(\d+) 步）", log)
    r["steps"] = int(m[-1]) if m else None
    m = re.findall(r"tokens: ↑(\d+) ↓(\d+) \(calls=(\d+)\)", log)
    r["tokens"] = int(m[-1][0]) + int(m[-1][1]) if m else None
    r["give_up_routed"] = len(re.findall(r"GIVE_UP_ROUTED_TO_DONE", log))
    r["give_up_intercepted"] = len(re.findall(r"GIVE_UP_INTERCEPTED", log))
    r["override"] = len(re.findall(r"GIVE_UP_OVERRIDDEN", log))
    r["stalled"] = len(re.findall(r"stalled", log))
    r["exit_code_kind"] = len(re.findall(r"ExitNonZero|ExitSignal|ToolTimeout", log))
    r["rc47_form"] = "criteria 空" in log and "GIVE_UP" in log
    # timestamps
    ts = re.findall(r"(\d{4}-\d{2}-\d{2}T[0-9:.+\-]+)", log)
    r["start"], r["end"] = (ts[0], ts[-1]) if len(ts) >= 2 else (None, None)
    out[name] = r

print(json.dumps(out, ensure_ascii=False, indent=1))
with open("/home/wutao/fa/lr_matrix.json", "w") as f:
    json.dump(out, f, ensure_ascii=False, indent=1)

# archive compaction attribution
arch = read("/home/wutao/.config/hearth/archive/") or ""
import subprocess
lines = subprocess.run(["wc", "-l", "/home/wutao/.config/hearth/archive/compacted.jsonl"],
                       capture_output=True, text=True).stdout.strip()
print("archive compacted.jsonl:", lines)
