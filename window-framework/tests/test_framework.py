#!/usr/bin/env python3
"""窗口群框架 v0.1 测试 — 覆盖 9 条命令 + 4 条框架断言（补丁7）。
运行: python3 tests/test_framework.py
"""
import os
import subprocess
import sys
import tempfile
from pathlib import Path

SRC = Path(__file__).resolve().parent.parent / "src" / "framework.py"
ROOT = Path(__file__).resolve().parent.parent

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


TMP = Path(tempfile.mkdtemp(prefix="wf-test-"))

print("=== 1. project create ===")
r = run(["project", "create", "test-app", "--type", "software"])
check("create rc=0", r.returncode == 0, r.stderr)
p = TMP / "test-app"
check("project.toml created", (p / "project.toml").exists())
check("skeleton dirs", all((p / d).exists() for d in
     ["windows", "shared", "shared/outputs", "shared/gates", ".trash", ".archive", ".snapshots"]))

print("=== 2. window create ===")
r = run(["window", "create", "test-app", "--name", "arch-01", "--role", "架构设计"])
check("create rc=0", r.returncode == 0, r.stderr)
wt = p / "windows" / "win-arch-01" / "window.toml"
check("window.toml created", wt.exists())
check("budget declared", "[budget]" in wt.read_text(), "§8.1")

print("=== 3. window list ===")
r = run(["window", "list", "test-app"])
check("list shows win-arch-01", "win-arch-01" in r.stdout, r.stdout)

print("=== 4. window start/stop（v0.2 语义：start 需 prompt+key）===")
# 旧窗口无 prompt → start 拒绝（v0.2 补充3 行为）
r = run(["window", "start", "test-app", "win-arch-01"])
check("start refused (no prompt)", r.returncode != 0)
# stop 仍直接改状态
r = run(["window", "stop", "test-app", "win-arch-01"])
check("stop rc=0", r.returncode == 0)
check("state blocked", 'state = "blocked"' in wt.read_text())

print("=== 5. window delete (soft) + restore ===")
r = run(["window", "delete", "test-app", "win-arch-01"])
check("delete rc=0", r.returncode == 0)
check("moved to .trash", (p / ".trash" / "win-arch-01").exists())
r = run(["window", "restore", "test-app", "win-arch-01"])
check("restore rc=0", r.returncode == 0)
check("back in windows", (p / "windows" / "win-arch-01").exists())

print("=== 6. duplicate window id auto-suffix ===")
run(["window", "create", "test-app", "--name", "arch-01"])
r = run(["window", "create", "test-app", "--name", "arch-01"])
check("auto -2", (p / "windows" / "win-arch-01-2").exists(), r.stdout)

print("=== 7. framework check (4 assertions) ===")
# 构造一个缺 budget 的窗口 → 断言2 应红
bad = p / "windows" / "win-bad-01"
bad.mkdir(parents=True)
bad_wt = """[window]
id = "win-bad-01"
name = "bad"
role = "x"
state = "pending"
created_by = "human"
created_at = "2026-08-01T00:00:00Z"
[context]
max_tokens = 32000
current_tokens = 0
compression_count = 0
[dependencies]
upstream = []
[outputs]
files = []
gate = ""
"""
(bad / "window.toml").write_text(bad_wt, encoding="utf-8")
r = run(["framework", "check", "test-app"])
check("check rc=1 (budget-declared red)", r.returncode == 1, r.stdout)
check("budget-declared FAIL shown", "budget-declared" in r.stdout and "FAIL" in r.stdout)
# 删掉坏窗口 → 全绿
import shutil
shutil.rmtree(bad)
r = run(["framework", "check", "test-app"])
check("check rc=0 (all green)", r.returncode == 0, r.stdout)
check("5/5 PASS", "5/5" in r.stdout, r.stdout)

print("=== 8. gate-script-exists assertion ===")
# 窗口声明 gate 但脚本不存在 → 红
gt = p / "windows" / "win-arch-01" / "window.toml"
text = gt.read_text().replace('gate = ""', 'gate = "shared/gates/missing.sh"')
gt.write_text(text, encoding="utf-8")
r = run(["framework", "check", "test-app"])
check("gate-script red", r.returncode == 1 and "gate-script-exists" in r.stdout, r.stdout)
# 补上脚本 → 绿
gate_sh = p / "shared" / "gates" / "missing.sh"
gate_sh.write_text("#!/bin/bash\necho VERIFY_PASS\n", encoding="utf-8")
r = run(["framework", "check", "test-app"])
check("gate-script green", r.returncode == 0, r.stdout)

print(f"\n=== RESULT: {passed} passed, {failed} failed ===")
sys.exit(1 if failed else 0)
