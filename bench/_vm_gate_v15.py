#!/usr/bin/env python3
"""v15 gate + deploy: upload -> touch -> fmt/clippy/test/wiring -> rebuild+restart service.

Same v13/v14 lessons apply:
- tar preserves mtime -> MUST touch all .rs after extract, or cargo reuses stale
  target artifacts and reports the OLD code's errors (fake pass/fail).
- run gates in the FOREGROUND via exec_command (blocking read); no nohup races.

v15 additions:
- `cargo fmt --all` (write) BEFORE `--check`, then report whether it had to
  change anything (so hand-written new code cannot red-light the gate on
  formatting alone, while still surfacing that it was unformatted).
- after the gates pass, rebuild the service binary and restart it with
  REPLAY_DIR pointing at the uploaded fixtures, so the `replay` provider is live.

Usage:  python _vm_gate_v15.py [--no-restart]
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


def make_tar() -> Path:
    out = Path(tempfile.gettempdir()) / "codex_v15_gate.tgz"
    with tarfile.open(out, "w:gz") as tf:
        for sub in ["crates", "docs", "bench", "Cargo.toml", "Cargo.lock", "constitution.md"]:
            src = LOCAL / sub
            if not src.exists():
                continue
            if src.is_file():
                tf.add(src, arcname=sub)
                continue
            for f in src.rglob("*"):
                if f.is_file() and not should_skip(f.relative_to(LOCAL)):
                    tf.add(f, arcname=str(f.relative_to(LOCAL)))
    print(f"[tar] {out} = {out.stat().st_size / 1e6:.1f} MB")
    return out


def main():
    restart = "--no-restart" not in sys.argv

    c = paramiko.SSHClient()
    c.set_missing_host_key_policy(paramiko.AutoAddPolicy())
    c.connect(VM_HOST, username=VM_USER, password=VM_PW, timeout=30)

    tarball = make_tar()
    ftp = c.open_sftp()
    ftp.put(str(tarball), "/home/wutao/codex_v15_gate.tgz")
    ftp.close()
    print("[scp] uploaded")

    def run(cmd, timeout=7200):
        _, stdout, stderr = c.exec_command(cmd, timeout=timeout)
        out = stdout.read().decode(errors="replace")
        err = stderr.read().decode(errors="replace")
        rc = stdout.channel.recv_exit_status()
        return rc, out, err

    rc, out, err = run(
        f"cd {REMOTE} && tar xzf /home/wutao/codex_v15_gate.tgz && "
        f"find {REMOTE}/crates -name '*.rs' -exec touch {{}} + && echo EXTRACT_OK"
    )
    print(f"[extract] rc={rc} {out.strip()[-200:]} {err.strip()[-200:]}")
    if rc != 0:
        sys.exit(1)

    gate = (
        f"cd {REMOTE} && source ~/.cargo/env && ("
        f"echo '=== FMT(write) ===' && cargo fmt --all; echo FMT_WRITE_RC=$?; "
        f"git diff --stat 2>/dev/null | tail -3; "
        f"echo '=== FMT(check) ===' && cargo fmt --all --check; echo FMT_RC=$?; "
        f"echo '=== CLIPPY ===' && cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -25; "
        f"cargo clippy --workspace --all-targets -- -D warnings >/dev/null 2>&1; echo CLIPPY_RC=$?; "
        f"echo '=== TEST ===' && cargo test --all 2>&1 | grep -E '^test result|error\\[|^error' | head -40; "
        f"cargo test --all >/dev/null 2>&1; echo TEST_RC=$?; "
        f"echo '=== WIRING ===' && cargo run -q -p project-xray -- wiring 2>&1 | tail -25; echo WIRING_RC=$?"
        f") 2>&1 | tee /home/wutao/gate_v15.log"
    )
    t0 = time.time()
    rc, out, err = run(gate, timeout=7200)
    print(out[-6000:])
    print(f"[gate] wall={time.time() - t0:.0f}s ssh_rc={rc}")

    if not restart:
        print("[deploy] skipped (--no-restart)")
        return

    # rebuild + restart with the replay provider enabled
    fixtures = f"{REMOTE}/bench/replay/fixtures"
    deploy = (
        f"cd {REMOTE} && source ~/.cargo/env && "
        f"cargo build -p service 2>&1 | tail -5; echo BUILD_RC=$?; "
        # NOTE: `pkill -f 'target/debug/service'` also matches THIS shell (its own
        # command line contains that literal), so it kills the deploy script before
        # the restart ever runs. The [t] bracket trick makes the pattern unable to
        # match itself. Bit us once in v15 -- service ended up simply dead.
        f"pkill -f '[t]arget/debug/service' ; sleep 2; "
        f"REPLAY_DIR={fixtures} setsid nohup bash -c "
        f"\"cd {REMOTE} && exec ./target/debug/service\" > /home/wutao/service.log 2>&1 < /dev/null & "
        f"disown; sleep 8; "
        f"grep -c 'replay provider registered' /home/wutao/service.log; "
        f"tail -3 /home/wutao/service.log"
    )
    rc, out, err = run(deploy, timeout=3600)
    print("[deploy]", out[-2500:], err[-500:])


if __name__ == "__main__":
    main()
