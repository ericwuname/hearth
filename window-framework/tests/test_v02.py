#!/usr/bin/env python3
"""窗口群框架 v0.2 测试 — 覆盖 S1-S3 新增功能（plan-v02 §v0.2.1 补充5 的 10 项）。
运行: python3 tests/test_framework.py（含 v0.1 回归）
"""
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

SRC = Path(__file__).resolve().parent.parent / "src" / "framework.py"
TMP = Path(tempfile.mkdtemp(prefix="wf-v02-"))
PROJECT = "test-app"

passed = 0
failed = 0


def run(args, env_extra=None):
    env = dict(os.environ)
    env["CODEX_PROJECTS_ROOT"] = str(TMP)
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


# ── 准备：项目 + 窗口（含 prompt）──────────────────────────
run(["project", "create", PROJECT, "--type", "software"])
run(["window", "create", PROJECT, "--name", "test-01", "--role", "测试角色",
     "--prompt", "你是测试窗口，任务：在 outputs/ 写 hello.txt 内容为 hi"])
P = TMP / PROJECT
W = P / "windows" / "win-test-01"
CONV = W / "conversation.jsonl"

print("=== S2: 对话存储 ===")
# 1. window create 含 prompt
wt = W / "window.toml"
check("prompt stored", 'prompt = "你是测试窗口' in wt.read_text())

print("=== S3: 快照/回滚 ===")
# 2. snapshot 创建
r = run(["window", "snapshot", PROJECT, "win-test-01"])
check("snapshot rc=0", r.returncode == 0, r.stderr)
snaps = list((P / ".snapshots").glob("win-test-01--*"))
check("snapshot dir created", len(snaps) == 1)
# 3. 写几行对话（模拟）
with open(CONV, "a", encoding="utf-8") as f:
    f.write('{"t": "2026-08-01T00:00:00Z", "role": "user", "content": "do it"}\n')
    f.write('{"t": "2026-08-01T00:00:01Z", "role": "assistant", "content": "ok"}\n')
# 4. 再次快照（应保留前一快照？v0.2 简单版：每次新建，不清理）
r = run(["window", "snapshot", PROJECT, "win-test-01"])
check("2nd snapshot", len(list((P / ".snapshots").glob("win-test-01--*"))) == 2)
# 5. 覆盖当前对话（模拟回滚前有未快照的历史）→ 回滚到【最新】快照（含对话的那个）
CONV.write_text('{"t": "2026-08-01T00:00:05Z", "role": "user", "content": "newer msg not in snapshot"}\n',
                encoding="utf-8")
ts = sorted(x.name for x in (P / ".snapshots").glob("win-test-01--*"))[-1].replace("win-test-01--", "")
r = run(["window", "rollback", PROJECT, "win-test-01", "--to", ts])
check("rollback rc=0", r.returncode == 0, r.stderr)
# 回滚后 conv = 快照版本（含 "do it"），当前历史移入 rolled-back
restored = CONV.read_text(encoding="utf-8")
check("conv restored from snapshot", "do it" in restored, restored[:80])
check("rolled-back preserved", len(list((P / ".snapshots" / "rolled-back").glob("*.jsonl"))) >= 1)

print("=== S2: export ===")
# 6. export markdown 可读
r = run(["window", "export", PROJECT, "win-test-01", "--format", "markdown"])
check("export rc=0", r.returncode == 0, r.stderr)
md = (P / "exports" / "win-test-01-conversation.md").read_text(encoding="utf-8")
check("markdown readable", "窗口 win-test-01 对话导出" in md and "**user**" in md, md[:100])
# 7. export json
r = run(["window", "export", PROJECT, "win-test-01", "--format", "json"])
check("export json rc=0", r.returncode == 0)
check("json file", (P / "exports" / "win-test-01-conversation.json").exists())

print("=== 补充2: 写路径沙箱（离线单测 Agent 校验）===")
sys.path.insert(0, str(SRC.parent))
import importlib.util
spec = importlib.util.spec_from_file_location("fw", SRC)
fw = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fw)
agent = fw.Agent(P, "win-test-01")
ok_write = agent._allowed_write(str(W / "outputs" / "hello.txt"))
denied_write = agent._allowed_write(str(P / "windows" / "win-other" / "x.txt"))
denied_toml = agent._allowed_write(str(W / "window.toml"))
check("write inside outputs OK", ok_write)
check("write other window DENIED", not denied_write)
check("write window.toml DENIED", not denied_toml)
check("read shared OK", agent._allowed_read(str(P / "shared" / "decisions.md")))

print("=== 补充1: 无 key 报错（不真调 API）===")
# 用子进程验证：无 DEEPSEEK_API_KEY → start 返回错误
run(["window", "create", PROJECT, "--name", "nokey-01", "--role", "r", "--prompt", "p"])
r = run(["window", "start", PROJECT, "win-nokey-01"], env_extra={"DEEPSEEK_API_KEY": ""})
check("no-key errors rc!=0", r.returncode != 0, f"rc={r.returncode}")
check("no-key message", "DEEPSEEK_API_KEY not set" in r.stdout + r.stderr)

print("=== 补充3: 无 prompt 窗口拒绝启动 ===")
run(["window", "create", PROJECT, "--name", "noprompt-01", "--role", "r"])
r = run(["window", "start", PROJECT, "win-noprompt-01"], env_extra={"DEEPSEEK_API_KEY": "x"})
check("no-prompt refused", r.returncode != 0, r.stdout)
check("no-prompt message", "no prompt" in r.stdout)

print("=== v0.1 回归 ===")
r = run(["framework", "check", PROJECT])
check("framework check green", r.returncode == 0, r.stdout)
check("5/5 PASS", "5/5" in r.stdout)

print(f"\n=== RESULT: {passed} passed, {failed} failed ===")
sys.exit(1 if failed else 0)
