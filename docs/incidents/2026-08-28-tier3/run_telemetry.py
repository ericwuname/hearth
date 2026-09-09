import paramiko, time, shlex
from collections import Counter

HOST = "192.168.220.131"
USER = "wutao"
PW = "123456"
AGNES_KEY = "cpk-f4UBH3NaHUUN2SAcW1LhZT0kyuqlu9TF5lTItbuXHSTP3w0c"
AGNES_URL = "https://api.agnes-ai.cn/v1"
MODEL = "agnes-2.5-flash"
GOAL = "用一句话回答：2加2等于几？只输出结果，不要解释。"
N = 15
HEARTH = "~/codex/target/release/hearth"

ssh = paramiko.SSHClient()
ssh.set_missing_host_key_policy(paramiko.AutoAddPolicy())
ssh.connect(HOST, username=USER, password=PW, timeout=20)

results = []
for i in range(1, N + 1):
    cmd = (
        f"timeout 150 {HEARTH} chat {shlex.quote(GOAL)} "
        f"--provider agnes --url {AGNES_URL} --api-key {AGNES_KEY} "
        f"--model {MODEL} --budget 20 2>&1"
    )
    t0 = time.time()
    stdin, stdout, stderr = ssh.exec_command(cmd, timeout=170)
    out = stdout.read().decode(errors="replace")
    err = stderr.read().decode(errors="replace")
    rc = stdout.channel.recv_exit_status()
    dur = time.time() - t0
    blob = (out + err).lower()
    if rc == 124:
        cls = "HANG"
    elif rc != 0:
        if any(k in blob for k in ["401", "403", "unauthorized", "api key", "invalid", "authentication"]):
            cls = "API_ERR"
        else:
            cls = "CRASH"
    else:
        cls = "SUCCESS"
    tail = "\n".join((out + err).strip().splitlines()[-3:])
    results.append((i, cls, rc, round(dur, 1), tail))
    print(f"RUN {i:2d} [{cls}] rc={rc} {dur:.1f}s", flush=True)
    print("   tail:", repr(tail[:160]), flush=True)

c = Counter(r[1] for r in results)
print("\n=== TALLY ===", flush=True)
for k in ["SUCCESS", "CRASH", "HANG", "API_ERR"]:
    print(f"{k}: {c.get(k, 0)}", flush=True)
print(f"TOTAL: {len(results)}", flush=True)

# persist locally
with open("/tmp/hearth_telemetry_result.txt", "w", encoding="utf-8") as f:
    for r in results:
        f.write(f"RUN {r[0]:2d} {r[1]} rc={r[2]} {r[3]}s\n  tail: {r[4][:200]}\n")
    f.write("\n=== TALLY ===\n")
    for k in ["SUCCESS", "CRASH", "HANG", "API_ERR"]:
        f.write(f"{k}: {c.get(k, 0)}\n")
    f.write(f"TOTAL: {len(results)}\n")
ssh.close()
print("=== DONE ===", flush=True)
