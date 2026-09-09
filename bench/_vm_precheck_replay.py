#!/usr/bin/env python3
"""Early compile-check of the new llm-replay crate ONLY.

Runs while the v15 benchmark chain is still using the live service, so it must
NOT relink target/debug/service (that would ETXTBSY / kill the running bench).
`cargo test -p llm-replay` only builds agent-types + llm-gateway + llm-replay.
"""
import os
import sys
import tarfile
import tempfile
import time
from pathlib import Path

import paramiko

VM_HOST = "192.168.220.131"
VM_USER = "wutao"
VM_PW = os.environ.get("CODEX_VM_PW", "")
REMOTE = "/home/wutao/codex_work"
LOCAL = Path(__file__).resolve().parent.parent
EXCLUDE_DIRS = {"target", ".git", ".workbuddy", "node_modules", "__pycache__"}


def should_skip(p: Path) -> bool:
    if set(p.parts) & EXCLUDE_DIRS:
        return True
    return p.suffix in {".log", ".zip", ".pytmp"}


def main():
    out = Path(tempfile.gettempdir()) / "codex_v15_pre.tgz"
    with tarfile.open(out, "w:gz") as tf:
        for sub in ["crates", "docs", "bench", "Cargo.toml", "Cargo.lock"]:
            src = LOCAL / sub
            if not src.exists():
                continue
            if src.is_file():
                tf.add(src, arcname=sub)
                continue
            for f in src.rglob("*"):
                if f.is_file() and not should_skip(f.relative_to(LOCAL)):
                    tf.add(f, arcname=str(f.relative_to(LOCAL)))
    print(f"[tar] {out.stat().st_size / 1e6:.1f} MB")

    c = paramiko.SSHClient()
    c.set_missing_host_key_policy(paramiko.AutoAddPolicy())
    c.connect(VM_HOST, username=VM_USER, password=VM_PW, timeout=30)
    ftp = c.open_sftp()
    ftp.put(str(out), "/home/wutao/codex_v15_pre.tgz")
    ftp.close()
    print("[scp] uploaded")

    def run(cmd, timeout=3600):
        _, so, se = c.exec_command(cmd, timeout=timeout)
        o = so.read().decode(errors="replace")
        e = se.read().decode(errors="replace")
        return so.channel.recv_exit_status(), o, e

    rc, o, e = run(
        f"cd {REMOTE} && tar xzf /home/wutao/codex_v15_pre.tgz && "
        f"find {REMOTE}/crates -name '*.rs' -exec touch {{}} + && echo EXTRACT_OK"
    )
    print("[extract]", rc, o.strip()[-200:], e.strip()[-200:])
    if rc != 0:
        sys.exit(1)

    t0 = time.time()
    cmd = (
        f"cd {REMOTE} && source ~/.cargo/env && ("
        "echo '=== BUILD llm-replay ===' && "
        "cargo test -p llm-replay 2>&1 | grep -E '^error|^warning: unused|^test |^test result|error\\[' | head -40; "
        "cargo test -p llm-replay >/dev/null 2>&1; echo REPLAY_TEST_RC=$?; "
        "echo '=== CLIPPY llm-replay ===' && "
        "cargo clippy -p llm-replay --all-targets -- -D warnings 2>&1 | tail -20; "
        "cargo clippy -p llm-replay --all-targets -- -D warnings >/dev/null 2>&1; echo REPLAY_CLIPPY_RC=$?"
        ") 2>&1 | tee /home/wutao/pre_v15.log"
    )
    rc, o, e = run(cmd)
    print(o[-6000:])
    print(f"[pre] wall={time.time() - t0:.0f}s rc={rc}")


if __name__ == "__main__":
    main()
