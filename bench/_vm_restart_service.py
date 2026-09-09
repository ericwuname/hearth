#!/usr/bin/env python3
"""Rebuild target/debug/service and restart it with the v15 replay fixtures.

Two traps this script exists to avoid:

1. **ETXTBSY** — cargo cannot relink `target/debug/service` while the binary is
   running, so the old process must die *before* the build, not after.
2. **self-kill** — `pkill -f 'target/debug/service'` also matches the bash that
   is running the pkill itself. `pkill -x service` matches the exact process
   name only and is safe.

Usage: CODEX_VM_PW=... python bench/_vm_restart_service.py
"""
import os
import sys
import time

import paramiko

VM_HOST = "192.168.220.131"
VM_USER = "wutao"
VM_PW = os.environ.get("CODEX_VM_PW", "")
REMOTE = "/home/wutao/codex_work"
FIXTURES = "/home/wutao/codex_work/bench/replay/fixtures"


def main():
    c = paramiko.SSHClient()
    c.set_missing_host_key_policy(paramiko.AutoAddPolicy())
    c.connect(VM_HOST, username=VM_USER, password=VM_PW, timeout=30)

    def run(cmd, timeout=3600):
        _, so, se = c.exec_command(cmd, timeout=timeout)
        o = so.read().decode(errors="replace")
        e = se.read().decode(errors="replace")
        return so.channel.recv_exit_status(), o, e

    print("[1/4] stop old service")
    run("pkill -x service ; sleep 2; pgrep -x service || echo STOPPED")

    print("[2/4] build service")
    t0 = time.time()
    rc, o, e = run(
        f"cd {REMOTE} && source ~/.cargo/env && "
        "cargo build -p service 2>&1 | tail -5; "
        "cargo build -p service >/dev/null 2>&1; echo BUILD_RC=$?"
    )
    print(o.strip()[-1500:], f"({time.time() - t0:.0f}s)")
    if "BUILD_RC=0" not in o:
        sys.exit(1)

    print("[3/4] start service with REPLAY_DIR")
    # The whole launcher is redirected again at the outer level: without it the
    # child keeps the SSH channel's stdout open and exec_command never sees EOF
    # (observed: the call hung for 18 min while the service was already up).
    run(
        f"( cd {REMOTE} && REPLAY_DIR={FIXTURES} setsid nohup "
        f"bash -c 'cd {REMOTE} && exec ./target/debug/service' "
        "> /home/wutao/service.log 2>&1 < /dev/null & disown ) "
        "> /dev/null 2>&1 < /dev/null; sleep 8; echo LAUNCHED"
    )

    print("[4/4] verify")
    rc, o, _ = run(
        "pgrep -x service && "
        "grep -Ei 'replay|deepseek-pro|zhipu-max|listening' /home/wutao/service.log "
        "| tail -8; "
        "curl -s -o /dev/null -w 'health=%{http_code}\\n' "
        "http://127.0.0.1:3000/health"
    )
    print(o.strip())


if __name__ == "__main__":
    main()
