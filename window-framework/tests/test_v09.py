#!/usr/bin/env python3
"""窗口群框架 v0.9 测试 — UX 呈现/容错/polish（plan-ux-v09 §v0.9.1 补充5 的 12 项）。
运行: python3 tests/test_v09.py
"""
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

SRC = Path(__file__).resolve().parent.parent / "src" / "framework.py"
TMP = Path(tempfile.mkdtemp(prefix="wf-v09-"))
PROJECT = "app"

passed = 0
failed = 0


def run(args, env_extra=None):
    env = dict(os.environ)
    env["CODEX_PROJECTS_ROOT"] = str(TMP)
    env["AGENT_MODE"] = "replay"
    if env_extra:
        env.update(env_extra)
    return subprocess.run([sys.executable, str(SRC)] + args,
                          capture_output=True, text=True, env=env)


def check(name, cond, detail=""):
    global passed, failed
    if cond:
        passed += 1
        print(f"  [PASS] {name}")
    else:
        failed += 1
        print(f"  [FAIL] {name} — {detail}")


P = TMP / PROJECT
run(["project", "create", PROJECT, "--type", "software"])
run(["window", "create", PROJECT, "--name", "dev", "--role", "开发", "--prompt", "p"])
W = P / "windows" / "win-dev"
CONV = W / "conversation.jsonl"

print("=== S1: window run --verbose ===")
# 1. replay demo 输出含步骤（红线 🔴）
r = run(["window", "run", PROJECT, "win-dev", "--verbose", "--max-turns", "8"])
check("run verbose rc=0", r.returncode == 0, r.stdout + r.stderr)
check("verbose has steps", any(k in r.stdout for k in
      ["Plan", "Read", "Write", "Bash", "DONE"]), r.stdout[:300])
# 2. 非 verbose 等同 start（补充2）
r2 = run(["window", "run", PROJECT, "win-dev", "--max-turns", "4"])
check("run no-verbose ok", r2.returncode == 0, r2.stderr)
# 3. step_log 持久化（补充1）
entries = [json.loads(l) for l in CONV.read_text(encoding="utf-8").splitlines() if l.strip()]
sl = [e for e in entries if e.get("meta", {}).get("step_log")]
check("step_log persisted", len(sl) >= 4, f"{len(sl)} logs")
check("step_log fields", all(k in sl[0]["meta"]["step_log"]
      for k in ("turn", "phase", "tool", "summary", "tokens", "wall_ms")), str(sl[0])[:150])

print("=== S2: resume 断点续传 ===")
# 5. resume 从 turn+1（红线 🔴）——replay 模式每次跑 max_turns 轮
r = run(["window", "run", PROJECT, "win-dev", "--max-turns", "6"])
before = [e for e in [json.loads(l) for l in CONV.read_text().splitlines() if l.strip()]
          if e.get("meta", {}).get("step_log")]
last_turn = max(e["meta"]["step_log"]["turn"] for e in before)
r = run(["window", "resume", PROJECT, "win-dev", "--max-turns", "3"])
check("resume rc=0", r.returncode == 0, r.stdout + r.stderr)
after = [e for e in [json.loads(l) for l in CONV.read_text().splitlines() if l.strip()]
         if e.get("meta", {}).get("step_log")]
new_turns = [e["meta"]["step_log"]["turn"] for e in after]
check("resume continues (no restart)", max(new_turns) > last_turn, f"last={last_turn} new={max(new_turns)}")
# 6. resume 跳过不完整残留（补充4）——追加空 assistant 再 resume
with open(CONV, "a", encoding="utf-8") as f:
    f.write(json.dumps({"t": "x", "role": "assistant", "content": None}) + "\n")
r = run(["window", "resume", PROJECT, "win-dev", "--max-turns", "2"])
check("resume skips incomplete", r.returncode == 0, r.stderr)
after2 = [json.loads(l) for l in CONV.read_text().splitlines() if l.strip()]
check("incomplete dropped", not (after2[-1].get("role") == "assistant" and not after2[-1].get("content")))

print("=== S2: deploy --last ===")
# 8. analyze → deploy --last
run(["window", "create", PROJECT, "--name", "req", "--role", "需求", "--prompt", "p"])
conv2 = P / "windows" / "win-req" / "conversation.jsonl"
with open(conv2, "a", encoding="utf-8") as f:
    for i in range(4):
        f.write(json.dumps({"t": "x", "role": "user", "content": f"需求 {i}"}) + "\n")
r = run(["window", "analyze", PROJECT, "win-req"])  # 不 --confirm 也写 last_analyze
check("analyze rc=0", r.returncode == 0, r.stdout)
check("last_analyze written", (P / ".snapshots" / "last_analyze.json").exists())
r = run(["workflow", "deploy", PROJECT, "--last"])
check("deploy --last rc=0", r.returncode == 0, r.stdout + r.stderr)
check("window from last", (P / "windows" / "win-dev-01").exists())
# 9. invalid last → 拒绝
(P / ".snapshots" / "last_analyze.json").write_text(
    json.dumps({"valid": False, "error": "test"}), encoding="utf-8")
r = run(["workflow", "deploy", PROJECT, "--last"])
check("deploy --last invalid rejected", r.returncode != 0, r.stdout)

print("=== S2: workflow retry ===")
# 10. retry 重置 stage 窗口为 pending
run(["window", "create", PROJECT, "--name", "w1", "--role", "r", "--prompt", "p"])
wt = P / "windows" / "win-w1" / "window.toml"
wt.write_text(wt.read_text().replace('state = "pending"', 'state = "blocked"'), encoding="utf-8")
# 写 workflow stage 引用 win-w1
pt = P / "project.toml"
pt.write_text(pt.read_text() + "\n[[workflow.stages]]\nid = \"s1\"\ntrigger = \"project_start\"\nwindows = [\"win-w1\"]\ngate = \"human:x\"\n", encoding="utf-8")
r = run(["workflow", "retry", PROJECT, "s1"])
check("retry rc=0", r.returncode == 0, r.stdout + r.stderr)
check("window reset pending", 'state = "pending"' in wt.read_text())

print("=== S3: codex status 聚合 ===")
# 11. status 与实际 window.toml 一致（红线 🔴）
r = run(["status"])
check("status rc=0", r.returncode == 0, r.stdout + r.stderr)
check("status shows project", PROJECT in r.stdout, r.stdout[:200])
check("status shows windows", "win-dev" in r.stdout and "win-w1" in r.stdout, r.stdout[:300])

print("=== 12. framework check ===")
r = run(["framework", "check", PROJECT])
check("framework check green", r.returncode == 0, r.stdout)

print("=== 回归 ===")
r = run(["window", "list", PROJECT])
check("window list works", r.returncode == 0, r.stderr)

print(f"\n=== RESULT: {passed} passed, {failed} failed ===")
sys.exit(1 if failed else 0)
