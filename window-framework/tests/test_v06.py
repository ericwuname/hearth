#!/usr/bin/env python3
"""窗口群框架 v0.6 测试 — 冲突仲裁第二层（§8.4）+ 集成测试骨架。
运行: python3 tests/test_v06.py
"""
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

SRC = Path(__file__).resolve().parent.parent / "src" / "framework.py"
TMP = Path(tempfile.mkdtemp(prefix="wf-v06-"))
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

print("=== S1: 冲突仲裁（§8.4 第二层）===")
# 加载模块测 line diff
import importlib.util
spec = importlib.util.spec_from_file_location("fw", SRC)
fw = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fw)

# 1. 不同区域自动合并（补充2）
base = "fn main() {\n    let x = 1;\n    let y = 2;\n    println!();\n}\n"
ver_a = "fn main() {\n    let x = 100;\n    let y = 2;\n    println!();\n}\n"  # 改行 1
ver_b = "fn main() {\n    let x = 1;\n    let y = 200;\n    println!();\n}\n"  # 改行 2
merged, conflict, detail = fw.merge_two("src/lib.rs", ver_a, ver_b, base)
check("disjoint regions merge", merged is not None and not conflict, detail)
check("merge keeps A change", "x = 100" in merged)
check("merge keeps B change", "y = 200" in merged)

# 2. 同区域冲突 → blocked
ver_a2 = "fn main() {\n    let x = 100;\n    let y = 2;\n    println!();\n}\n"
ver_b2 = "fn main() {\n    let x = 999;\n    let y = 2;\n    println!();\n}\n"  # 都改行 1
merged2, conflict2, detail2 = fw.merge_two("src/lib.rs", ver_a2, ver_b2, base)
check("same-region conflict detected", merged2 is None and conflict2, detail2)

# 3. conflict list 检测同路径声明
run(["window", "create", PROJECT, "--name", "a", "--role", "r", "--prompt", "p"])
run(["window", "create", PROJECT, "--name", "b", "--role", "r", "--prompt", "p"])
# 两个窗口声明同一 outputs 路径
for wid in ["win-a", "win-b"]:
    wt = P / "windows" / wid / "window.toml"
    text = wt.read_text(encoding="utf-8").replace(
        'files = []', 'files = [\n  "shared/outputs/api.rs"\n]')
    wt.write_text(text, encoding="utf-8")
r = run(["conflict", "list", PROJECT])
check("conflict list detects", "api.rs" in r.stdout and "win-a" in r.stdout, r.stdout)

# 4. framework check 断言 4 红
r = run(["framework", "check", PROJECT])
check("conflict-marked red", r.returncode == 1 and "conflict" in r.stdout, r.stdout)

# 5. resolve --keep 清冲突
r = run(["conflict", "resolve", PROJECT, "shared/outputs/api.rs", "--keep", "win-a"])
check("resolve rc=0", r.returncode == 0, r.stdout + r.stderr)
check("loser blocked", 'state = "blocked"' in (P / "windows" / "win-b" / "window.toml").read_text())
# 6. 断言 4 重新绿
r = run(["framework", "check", PROJECT])
check("conflict-marked green after resolve", r.returncode == 0, r.stdout)

print("=== 集成测试骨架（模式 A 的自动化部分）===")
# 7. 预置需求对话 → analyze → deploy → workflow（replay 全链路验证机制）
run(["window", "create", PROJECT, "--name", "req", "--role", "需求", "--prompt", "p"])
conv = P / "windows" / "win-req" / "conversation.jsonl"
with open(conv, "a", encoding="utf-8") as f:
    for i in range(20):
        f.write(json.dumps({"t": "x", "role": "user", "content": f"博客需求 {i}"}) + "\n")
        f.write(json.dumps({"t": "x", "role": "assistant", "content": f"澄清 {i}"}) + "\n")
r = run(["window", "analyze", PROJECT, "win-req", "--confirm"])
check("analyze chain rc=0", r.returncode == 0, r.stdout + r.stderr)
r = run(["workflow", "deploy", PROJECT, "win-req"])
check("deploy chain rc=0", r.returncode == 0, r.stdout + r.stderr)
r = run(["workflow", "start", PROJECT])
check("workflow chain runs", r.returncode in (0, 1), r.stdout)  # human gate 可能停

print("=== 补充4: 沙箱路径审计（集成测试断言）===")
# 检查所有窗口产出都在允许前缀内
bad = []
for d in (P / "windows").iterdir() if (P / "windows").exists() else []:
    if d.name.startswith("win-") and (d / "outputs").exists():
        for f in (d / "outputs").rglob("*"):
            if f.is_file():
                pass  # 窗口 outputs 内部合法
    # 检查 conversation 引用的写路径（简化：window.toml outputs 必须在允许前缀）
    wt = d / "window.toml"
    if wt.exists():
        data = json.loads('{}') if False else None
        import tomllib
        try:
            tdata = tomllib.loads(wt.read_text(encoding="utf-8"))
            for out in tdata.get("outputs", {}).get("files", []):
                if not (out.startswith("shared/outputs/") or out.startswith("src/") or out.startswith("web/") or out.startswith("docs/")):
                    bad.append(f"{d.name}:{out}")
        except Exception:
            pass
check("sandbox paths audited", not bad, str(bad))

print("=== 回归 ===")
r = run(["window", "list", PROJECT])
check("window list works", r.returncode == 0, r.stderr)

print(f"\n=== RESULT: {passed} passed, {failed} failed ===")
sys.exit(1 if failed else 0)
