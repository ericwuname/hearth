#!/usr/bin/env python3
"""VM self-proof for the v13 wiring gate (forge-v13 red line #2).

On the VM (~/codex_work):
  1. break the real call site `self.record_tool_exchange();` in agent-core/loop.rs
  2. run `codex-xray wiring` -> MUST exit 1 (RED BREAK)
  3. revert loop.rs
  4. run `codex-xray wiring` -> MUST exit 0 (green)
codex-xray reads .rs as TEXT (grep), so no full rebuild needed for the edit test;
`cargo run -p project-xray -- wiring` builds the tiny binary once then greps source.
"""
import os, time, paramiko

VM_HOST = "192.168.220.131"
VM_USER = "wutao"
VM_PW = os.environ.get("CODEX_VM_PW", "")
REMOTE = "~/codex_work"


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
    # baseline green first
    print("[1] baseline green check")
    rc, out, err = run(c,
        f"cd {REMOTE} && source ~/.cargo/env && cargo run -q -p project-xray -- wiring 2>&1 | tail -20")
    print(out[-1500:])
    print(f"    baseline rc={rc}  (expect 0)")
    green_before = (rc == 0)

    # backup + break the call site
    print("[2] break real call site self.record_tool_exchange();")
    run(c, f"cd {REMOTE} && cp crates/agent-core/src/loop.rs /tmp/loop.rs.bak")
    # comment ONLY the call (not the fn def): replace '    self.record_tool_exchange();'
    rc2, o2, e2 = run(c,
        r"cd ~/codex_work && sed -i 's/^\( *\)self\.record_tool_exchange();/\1// self.record_tool_exchange();  # BREAK_FOR_SELFPROOF/' crates/agent-core/src/loop.rs")
    rcg, outg, errg = run(c,
        f"cd {REMOTE} && grep -n 'BREAK_FOR_SELFPROOF' crates/agent-core/src/loop.rs")
    print("    break marker present:" , bool(rcg == 0 and outg.strip()))

    print("[3] wiring must turn RED (exit 1)")
    rc3, out3, err3 = run(c,
        f"cd {REMOTE} && source ~/.cargo/env && cargo run -q -p project-xray -- wiring 2>&1 | tail -20")
    print(out3[-1500:])
    print(f"    after-break rc={rc3}  (expect !=0 -> RED)")
    red_after = (rc3 != 0)

    # revert
    print("[4] revert loop.rs")
    run(c, f"cd {REMOTE} && cp /tmp/loop.rs.bak crates/agent-core/src/loop.rs && rm /tmp/loop.rs.bak")
    rc4, out4, err4 = run(c, f"cd {REMOTE} && grep -c BREAK_FOR_SELFPROOF crates/agent-core/src/loop.rs")
    print("    break marker remaining (expect 0):", rc4)

    print("[5] wiring must be GREEN again (exit 0)")
    rc5, out5, err5 = run(c,
        f"cd {REMOTE} && source ~/.cargo/env && cargo run -q -p project-xray -- wiring 2>&1 | tail -20")
    print(out5[-1500:])
    print(f"    reverted rc={rc5}  (expect 0)")
    green_after = (rc5 == 0)

    c.close()
    print("\n=== SELF-PROOF VERDICT ===")
    print(f"  baseline green     : {green_before}")
    print(f"  break -> red (rc!=0): {red_after}")
    print(f"  revert -> green     : {green_after}")
    ok = green_before and red_after and green_after
    print(f"  SELF-PROOF {'PASS' if ok else 'FAIL'}")


if __name__ == "__main__":
    main()
