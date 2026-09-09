#!/usr/bin/env python3
"""v14 S1-S3 gate: upload changed sources -> touch -> fmt/clippy/test/wiring on VM.

Follows the v13 lessons:
- tar packing preserves mtime -> MUST touch all .rs after extract or cargo
  reuses stale target artifacts (fake pass/fail).
- run gates in the FOREGROUND via exec_command (blocking read), log to
  ~/gate_v14.log; no nohup races.
- paramiko SFTP does not expand ~ -> absolute /home/wutao paths.
"""
import os, sys, tarfile, tempfile, time
from pathlib import Path

import paramiko

VM_HOST = "192.168.220.131"
VM_USER = "wutao"
VM_PW = os.environ.get("CODEX_VM_PW", "")
REMOTE = "/home/wutao/codex_work"
LOCAL = Path(__file__).resolve().parent.parent  # workspace root

EXCLUDE_DIRS = {"target", ".git", ".workbuddy", "node_modules", "__pycache__"}


def should_skip(p: Path) -> bool:
    parts = set(p.parts)
    if parts & EXCLUDE_DIRS:
        return True
    if p.suffix in {".log", ".zip", ".pytmp"}:
        return True
    return False


def make_tar() -> Path:
    out = Path(tempfile.gettempdir()) / "codex_v14_gate.tgz"
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
    print(f"[tar] {out} = {out.stat().st_size/1e6:.1f} MB")
    return out


def main():
    c = paramiko.SSHClient()
    c.set_missing_host_key_policy(paramiko.AutoAddPolicy())
    c.connect(VM_HOST, username=VM_USER, password=VM_PW, timeout=30)

    tarball = make_tar()
    ftp = c.open_sftp()
    ftp.put(str(tarball), "/home/wutao/codex_v14_gate.tgz")
    ftp.close()
    print("[scp] uploaded")

    def run(cmd, timeout=7200):
        _, stdout, stderr = c.exec_command(cmd, timeout=timeout)
        out = stdout.read().decode(errors="replace")
        err = stderr.read().decode(errors="replace")
        rc = stdout.channel.recv_exit_status()
        return rc, out, err

    # extract (keep target/ for incremental build), then touch ALL .rs
    rc, out, err = run(
        f"cd {REMOTE} && tar xzf /home/wutao/codex_v14_gate.tgz && "
        f"find {REMOTE}/crates -name '*.rs' -exec touch {{}} + && echo EXTRACT_OK"
    )
    print(f"[extract] rc={rc} {out.strip()[-200:]} {err.strip()[-200:]}")
    if rc != 0:
        sys.exit(1)

    gate = (
        f"cd {REMOTE} && source ~/.cargo/env && ("
        f"echo '=== FMT ===' && cargo fmt --all --check; echo FMT_RC=$?; "
        f"echo '=== CLIPPY ===' && cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -5; "
        f"cargo clippy --workspace --all-targets -- -D warnings >/dev/null 2>&1; echo CLIPPY_RC=$?; "
        f"echo '=== TEST ===' && cargo test --all 2>&1 | grep -E '^test result|error\\[|^error' | head -40; "
        f"cargo test --all >/dev/null 2>&1; echo TEST_RC=$?; "
        f"echo '=== WIRING ===' && cargo run -q -p project-xray -- wiring 2>&1; echo WIRING_RC=$?"
        f") 2>&1 | tee /home/wutao/gate_v14.log"
    )
    t0 = time.time()
    rc, out, err = run(gate, timeout=7200)
    print(out[-3000:])
    print(f"[gate] wall={time.time()-t0:.0f}s ssh_rc={rc}")


if __name__ == "__main__":
    main()
