#!/usr/bin/env python3
"""Launch a long benchmark run as a process fully detached from the calling shell.

Why: the v13 matrix run died at task 15/20 because the host shell that owned the
background job was reclaimed, taking the whole process tree with it. DETACHED_PROCESS
+ CREATE_NEW_PROCESS_GROUP (+ CREATE_BREAKAWAY_FROM_JOB when the job object allows)
gives the benchmark its own lifetime; progress is observable via the log file.

Usage: python launch_detached.py <script.py> [args...]
"""
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
OUT = HERE / "results" / "matrix-v13.stdout.log"

DETACHED_PROCESS = 0x00000008
CREATE_NEW_PROCESS_GROUP = 0x00000200
CREATE_BREAKAWAY_FROM_JOB = 0x01000000

if len(sys.argv) < 2:
    sys.exit("usage: launch_detached.py <script.py> [args...]")

OUT.parent.mkdir(parents=True, exist_ok=True)
cmd = [sys.executable, "-u"] + sys.argv[1:]

flags_try = [
    DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_BREAKAWAY_FROM_JOB,
    DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP,
]

last_err = None
for flags in flags_try:
    try:
        with open(OUT, "ab") as fh:
            p = subprocess.Popen(cmd, stdout=fh, stderr=subprocess.STDOUT,
                                 stdin=subprocess.DEVNULL, cwd=str(HERE),
                                 creationflags=flags, close_fds=True)
        print(f"LAUNCHED pid={p.pid} flags=0x{flags:08x}")
        print(f"LOG={OUT}")
        sys.exit(0)
    except OSError as e:  # breakaway refused by the job object
        last_err = e

sys.exit(f"failed to launch detached: {last_err}")
