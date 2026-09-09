#!/usr/bin/env python3
"""VM wiring self-proof (robust): SFTP edit, not shell sed.

Break the REAL call site string `self.record_tool_exchange();` in
agent-core/src/loop.rs by removing that exact substring, run codex-xray
wiring (must exit 1), revert, run again (must exit 0).
"""
import os, paramiko, tempfile
from pathlib import Path

VM_HOST = "192.168.220.131"
VM_USER = "wutao"
VM_PW = os.environ.get("CODEX_VM_PW", "")
REMOTE = "/home/wutao/codex_work"  # absolute; paramiko SFTP does NOT expand ~
TARGET = "crates/agent-core/src/loop.rs"
CALL = "self.record_tool_exchange();"   # exact substring = the assertion's `any` pattern


def ssh():
    c = paramiko.SSHClient()
    c.set_missing_host_key_policy(paramiko.AutoAddPolicy())
    c.connect(VM_HOST, username=VM_USER, password=VM_PW, timeout=30)
    return c


def run(c, cmd, timeout=600):
    _, stdout, stderr = c.exec_command(cmd, timeout=timeout)
    out = stdout.read().decode(errors="replace")
    err = stderr.read().decode(errors="replace")
    rc = stdout.channel.recv_exit_status()
    return rc, out, err


def main():
    c = ssh()
    tmp = Path(tempfile.gettempdir()) / "loop_rs_bak.pytmp"
    ftp = c.open_sftp()
    ftp.get(f"{REMOTE}/{TARGET}", str(tmp))
    original = tmp.read_text()
    # sanity: the call appears exactly once (the def is `fn record_tool_exchange`, not CALL)
    assert original.count(CALL) == 1, f"unexpected CALL count={original.count(CALL)}"
    print(f"[ok] downloaded loop.rs, CALL occurrences={original.count(CALL)}")

    # baseline green
    rc, out, _ = run(c, f"cd {REMOTE} && source ~/.cargo/env && cargo run -q -p project-xray -- wiring 2>&1")
    green_before = (rc == 0)
    print(f"[1] baseline green rc={rc} (expect 0)")

    # break: remove the exact substring
    broken = original.replace(CALL, "/* SELFPROOF_BREAK */")
    assert CALL not in broken, "break failed to remove CALL"
    tmp.write_text(broken)
    ftp.put(str(tmp), f"{REMOTE}/{TARGET}")
    ftp.close()
    print("[2] broke call site on VM (removed exact substring)")

    rc, out, _ = run(c, f"cd {REMOTE} && source ~/.cargo/env && cargo run -q -p project-xray -- wiring 2>&1")
    red_after = (rc != 0)
    print(out[-1200:])
    print(f"[3] after-break rc={rc} (expect !=0 -> RED)")
    tool_exchange_line = [l for l in out.splitlines() if "tool-exchange-wired" in l]
    if tool_exchange_line:
        print("    ", tool_exchange_line[0].strip())

    # revert
    tmp.write_text(original)
    ftp = c.open_sftp()
    ftp.put(str(tmp), f"{REMOTE}/{TARGET}")
    ftp.close()
    rc_chk, out_chk, _ = run(c, f"cd {REMOTE} && grep -c SELFPROOF_BREAK {TARGET}")
    print(f"[4] reverted; leftover marker (expect 0 matches -> grep rc=1): rc={rc_chk}")

    rc, out, _ = run(c, f"cd {REMOTE} && source ~/.cargo/env && cargo run -q -p project-xray -- wiring 2>&1")
    green_after = (rc == 0)
    print(f"[5] reverted rc={rc} (expect 0)")

    c.close()
    print("\n=== SELF-PROOF VERDICT ===")
    print(f"  baseline green       : {green_before}")
    print(f"  break -> RED (rc!=0) : {red_after}")
    print(f"  revert  -> GREEN     : {green_after}")
    print(f"  SELF-PROOF {'PASS' if (green_before and red_after and green_after) else 'FAIL'}")


if __name__ == "__main__":
    main()
