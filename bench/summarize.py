#!/usr/bin/env python3
"""Summarise a benchmark batch log into a pass/fail table."""
import json
import re
import sys
from pathlib import Path

path = Path(sys.argv[1] if len(sys.argv) > 1 else "results/batch-v126.log")
txt = path.read_text(encoding="utf-8", errors="replace")
objs = re.findall(r"\{[^{}]*\"task\"[^{}]*\}", txt, re.S)

rows = []
for o in objs:
    try:
        rows.append(json.loads(o))
    except json.JSONDecodeError:
        pass

npass = 0
for d in rows:
    ok = d.get("success")
    npass += bool(ok)
    tail = (d.get("verify_tail") or "").replace("\n", " ")[:60]
    print(f"{'PASS' if ok else 'FAIL'}  {d['task']:<22} {d['wall_s']:>6}s  steps={d['steps']:<3} {tail}")

print(f"\n== {npass}/{len(rows)} passed ==")
