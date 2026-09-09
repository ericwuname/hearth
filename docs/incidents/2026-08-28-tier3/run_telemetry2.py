import paramiko, time, shlex
from collections import Counter

HOST = "192.168.220.131"; USER = "wutao"; PW = "123456"
AGNES_KEY = "cpk-f4UBH3NaHUUN2SAcW1LhZT0kyuqlu9TF5lTItbuXHSTP3w0c"
AGNES_URL = "https://api.agnes-ai.cn/v1"
MODEL = "agnes-2.5-flash"
GOAL = "用一句话回答：2加2等于几？只输出结果，不要解释。"
N = 15
H = "~/codex/target/release/hearth"
CFG = "~/.config/hearth/config.toml"
AGNES_CFG = f'''provider = "agnes"
url = "{AGNES_URL}"
api_key = "{AGNES_KEY}"
mode = "{MODEL}"
'''

ssh = paramiko.SSHClient()
ssh.set_missing_host_key_policy(paramiko.AutoAddPolicy())
ssh.connect(HOST, username=USER, password=PW, timeout=20)

# backup + install agnes config
ssh.exec_command(f"cp {CFG} {CFG}.bak")
ssh.exec_command(f"cat > {CFG} <<'EOF'\n{AGNES_CFG}EOF")
print("config backed up + agnes installed", flush=True)

results = []
try:
    for i in range(1, N + 1):
        cmd = f"timeout 150 {H} chat --budget 20 {shlex.quote(GOAL)} 2>&1"
        t0 = time.time()
        stdin, stdout, stderr = ssh.exec_command(cmd, timeout=170)
        out = stdout.read().decode(errors="replace")
        err = stderr.read().decode(errors="replace")
        rc = stdout.channel.recv_exit_status()
        dur = time.time() - t0
        blob = (out + err).lower()
        if "missing session" in blob or "执行失败" in out:
            cls = "BACKEND_ERR"
        elif rc == 124:
            cls = "HANG"
        elif rc != 0:
            cls = "CRASH"
        else:
            cls = "SUCCESS"
        tail = "\n".join(out.strip().splitlines()[-2:])
        results.append((i, cls, rc, round(dur, 1), tail))
        print(f"RUN {i:2d} [{cls}] rc={rc} {dur:.1f}s", flush=True)
        print("   tail:", repr(tail[:140]), flush=True)
finally:
    # always restore original config
    ssh.exec_command(f"cp {CFG}.bak {CFG} && rm -f {CFG}.bak")
    print("config restored", flush=True)

c = Counter(r[1] for r in results)
print("\n=== TALLY ===", flush=True)
for k in ["SUCCESS", "CRASH", "HANG", "BACKEND_ERR"]:
    print(f"{k}: {c.get(k, 0)}", flush=True)
print(f"TOTAL: {len(results)}", flush=True)

with open("C:/tmp/hearth_telemetry_result.txt", "w", encoding="utf-8") as f:
    for r in results:
        f.write(f"RUN {r[0]:2d} {r[1]} rc={r[2]} {r[3]}s\n  tail: {r[4][:200]}\n")
    f.write("\n=== TALLY ===\n")
    for k in ["SUCCESS", "CRASH", "HANG", "BACKEND_ERR"]:
        f.write(f"{k}: {c.get(k, 0)}\n")
    f.write(f"TOTAL: {len(results)}\n")
ssh.close()
print("=== DONE ===", flush=True)
