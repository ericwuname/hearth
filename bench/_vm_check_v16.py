#!/usr/bin/env python3
"""Quick v16 precheck: compile + clippy on the 4 affected crates.
Uploads only source & Cargo files (skips target/.git/.workbuddy).
"""
import os, io, tarfile, paramiko, sys

VM_HOST = "192.168.220.131"
VM_USER = "wutao"
VM_PW = os.environ.get("CODEX_VM_PW", "")
REMOTE = "/home/wutao/codex"
LOCAL = r"C:\Users\87465\Desktop\codex-rust-v1.0-final"

def main():
    # 1. tar source & Cargo files (exclude target/.git/.workbuddy/sessions)
    buf = io.BytesIO()
    tf = tarfile.open(fileobj=buf, mode="w:gz")
    project = os.path.basename(LOCAL)
    parent = os.path.dirname(LOCAL)
    os.chdir(parent)
    for root, dirs, files in os.walk(project):
        # Skip heavy dirs
        parts = root.replace("\\", "/").split("/")
        if any(skip in parts for skip in ("target", ".git", ".workbuddy", "sessions",
                                          "__pycache__", "node_modules")):
            continue
        # Only include changed areas + Cargo files
        rroot = root.replace("\\", "/")
        keep = any(k in rroot for k in (
            "crates/experience", "crates/agent-core", "crates/nervous-system",
            "crates/subconscious", "crates/service",
            "docs/xray",
            "Cargo.toml", "Cargo.lock",
        ))
        if not keep and files:
            continue
        for f in files:
            if f.endswith((".rs", ".toml", ".lock")):
                fpath = os.path.join(root, f)
                tf.add(fpath, os.path.relpath(fpath, parent))
    tf.close()
    os.chdir(LOCAL)

    # 2. upload + extract
    c = paramiko.SSHClient()
    c.set_missing_host_key_policy(paramiko.AutoAddPolicy())
    c.connect(VM_HOST, username=VM_USER, password=VM_PW, timeout=30)

    ftp = c.open_sftp()
    ftp.putfo(io.BytesIO(buf.getvalue()), "v16_src.tar.gz")
    ftp.close()

    _, stdout, stderr = c.exec_command(
        f"cd {REMOTE} && tar xzf ~/v16_src.tar.gz --overwrite 2>&1 && echo EXTRACT_OK"
    )
    out = stdout.read().decode(errors="replace") + stderr.read().decode(errors="replace")
    if "EXTRACT_OK" not in out:
        print("EXTRACT FAILED:", out)
        sys.exit(1)

    # 3. touch .rs files to force rebuild
    _, _, _ = c.exec_command(f"find {REMOTE}/crates -name '*.rs' -exec touch {{}} +")

    # 4. compile affected crates
    print("[check] cargo check -p experience -p agent-core -p nervous-system -p service")
    _, stdout, stderr = c.exec_command(
        f"cd {REMOTE} && source ~/.cargo/env && "
        f"cargo check -p experience -p agent-core -p nervous-system -p service "
        f"2>&1; echo BUILD_RC=$?"
    )
    import time
    t0 = time.time()
    all_out = ""
    while time.time() - t0 < 600:
        try:
            chunk = stdout.channel.recv(4096).decode(errors="replace")
            if not chunk:
                break
            all_out += chunk
            print(chunk, end="", flush=True)
        except:
            break
    # Read remaining
    try:
        more = stdout.read().decode(errors="replace")
        all_out += more
        print(more, end="", flush=True)
    except:
        pass
    err = stderr.read().decode(errors="replace")
    if err.strip():
        print("STDERR:", err[:500])

    rc = 0 if "BUILD_RC=0" in all_out else 1
    if rc == 0:
        print("\n=== BUILD OK ===")
    else:
        print(f"\n=== BUILD FAILED (rc={rc}) ===")
    sys.exit(rc)

if __name__ == "__main__":
    main()
