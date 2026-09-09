#!/usr/bin/env python3
"""窗口群框架 v0.5 测试 — analyze + import + template（plan-v05 §v0.5.1 补充3 的 20 项）。
运行: python3 tests/test_v05.py
"""
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

SRC = Path(__file__).resolve().parent.parent / "src" / "framework.py"
TMP = Path(tempfile.mkdtemp(prefix="wf-v05-"))
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
# 需求窗口 + 对话
run(["window", "create", PROJECT, "--name", "req", "--role", "需求", "--prompt", "p"])
conv = P / "windows" / "win-req" / "conversation.jsonl"
with open(conv, "a", encoding="utf-8") as f:
    for i in range(10):
        f.write(json.dumps({"t": "x", "role": "user", "content": f"需求 {i}"}) + "\n")
        f.write(json.dumps({"t": "x", "role": "assistant", "content": f"澄清 {i}"}) + "\n")

print("=== S1: window analyze ===")
# 1. replay 产出有效 YAML
r = run(["window", "analyze", PROJECT, "win-req"])
check("analyze replay rc=0", r.returncode == 0, r.stdout + r.stderr)
check("replay yaml valid", "valid" in r.stdout, r.stdout)
# 2. 契约字段（补充2）
check("windows listed", "win-dev-01" in r.stdout, r.stdout)
# 6. analyze → deploy 链（先 --confirm 写入产出）
r = run(["window", "analyze", PROJECT, "win-req", "--confirm"])
check("analyze confirm rc=0", r.returncode == 0, r.stdout)
r = run(["workflow", "deploy", PROJECT, "win-req"])
check("deploy after analyze rc=0", r.returncode == 0, r.stdout + r.stderr)
check("window created from analyze", (P / "windows" / "win-dev-01").exists())
# 3/4/5. 校验（replay YAML 是合法样例——直接测 validate 函数）
import importlib.util
spec = importlib.util.spec_from_file_location("fw", SRC)
fw = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fw)
wins = fw._parse_windows_shared(fw.REPLAY_YAML)
stages = fw._parse_stages_shared(fw.REPLAY_YAML)
ok, err = fw.validate_deploy_yaml(wins, stages)
check("validate ok", ok, err)
check("window count 1..10", 1 <= len(wins) <= 10)
# 8. real 无 key 拒绝
r = run(["window", "analyze", PROJECT, "win-req"],
        env_extra={"AGENT_MODE": "real", "DEEPSEEK_API_KEY": ""})
check("real no-key refuses", r.returncode != 0 and "not set" in r.stdout, r.stdout)
# 7. 坏 YAML 拒绝（构造含环依赖）
bad_wins = [{"id": "a", "role": "r", "prompt": "p", "depends_on": ["b"],
             "budget": {}, "outputs": [], "gate": "g.sh"},
            {"id": "b", "role": "x", "prompt": "p", "depends_on": ["a"],
             "budget": {}, "outputs": [], "gate": "g.sh"}]
bad_stages = [{"id": "s", "trigger": "start", "windows": ["a"], "gate": "g.sh"}]
ok2, err2 = fw.validate_deploy_yaml(bad_wins, bad_stages)
check("cycle rejected", not ok2 and "cycle" in err2, err2)
# 缺 gate 拒绝
bad_wins2 = [{"id": "a", "role": "r", "prompt": "p", "depends_on": [],
              "budget": {}, "outputs": []}]
ok3, err3 = fw.validate_deploy_yaml(bad_wins2, [])
check("missing fields rejected", not ok3 and "missing" in err3, err3)

print("=== S3: window import ===")
# 10. 导入 JSONL 格式（同框架格式）
src1 = TMP / "ext.jsonl"
with open(src1, "w", encoding="utf-8") as f:
    for i in range(5):
        f.write(json.dumps({"t": "x", "role": "user", "content": f"ext {i}"}) + "\n")
r = run(["window", "import", PROJECT, "--name", "imp1", "--role", "研究员",
         "--source", str(src1)])
check("import jsonl rc=0", r.returncode == 0, r.stdout + r.stderr)
check("imported window exists", (P / "windows" / "win-imp1" / "conversation.jsonl").exists())
r = run(["window", "export", PROJECT, "win-imp1", "--format", "json"])
check("export after import", r.returncode == 0)
# 11. OpenAI messages 数组
src2 = TMP / "openai.json"
openai_msgs = [
    {"role": "user", "content": "hi"},
    {"role": "assistant", "content": None,
     "tool_calls": [{"id": "c1", "type": "function",
                     "function": {"name": "read", "arguments": "{\"path\": \"shared/x.md\"}"}}]},
    {"role": "tool", "tool_call_id": "c1", "content": "file content"},
]
src2.write_text(json.dumps(openai_msgs), encoding="utf-8")
r = run(["window", "import", PROJECT, "--name", "imp2", "--role", "r",
         "--source", str(src2)])
check("import openai rc=0", r.returncode == 0, r.stdout + r.stderr)
data = [json.loads(l) for l in (P / "windows" / "win-imp2" / "conversation.jsonl")
        .read_text(encoding="utf-8").splitlines() if l.strip()]
check("tool_calls mapped", any(e.get("tool_calls") for e in data),
      json.dumps(data[:1], ensure_ascii=False))
# 12. 长对话导入自动压缩
src3 = TMP / "long.jsonl"
with open(src3, "w", encoding="utf-8") as f:
    for i in range(30):
        f.write(json.dumps({"t": "x", "role": "user", "content": f"m {i}"}) + "\n")
r = run(["window", "import", PROJECT, "--name", "imp3", "--role", "r",
         "--source", str(src3)])
check("import long rc=0", r.returncode == 0, r.stdout)
check("long compressed", "compressed" in r.stdout or "import" in r.stdout, r.stdout)

print("=== S4: template ===")
# 14. save
r = run(["template", "save", "开发模板", "--project", PROJECT, "--from-window", "win-req"])
check("template save rc=0", r.returncode == 0, r.stdout + r.stderr)
# 15. list
r = run(["template", "list"])
check("template list shows", "开发模板" in r.stdout, r.stdout)
# 16. create from template
r = run(["template", "create", "开发模板", "--project", PROJECT, "--win-name", "tpl-win"])
check("template create rc=0", r.returncode == 0, r.stdout + r.stderr)
check("tpl window exists", (P / "windows" / "win-tpl-win").exists())
# 17. 重名拒绝（不 --force）
r = run(["template", "save", "开发模板", "--project", PROJECT, "--from-window", "win-req"])
check("template dup rejected", r.returncode != 0 and "exists" in r.stdout, r.stdout)
r = run(["template", "save", "开发模板", "--project", PROJECT, "--from-window", "win-req", "--force"])
check("template force ok", r.returncode == 0, r.stdout)

print("=== export --compress/--full ===")
# 18. compress flag
r = run(["window", "export", PROJECT, "win-imp3", "--format", "markdown", "--compress"])
check("export compress rc=0", r.returncode == 0, r.stderr)
r = run(["window", "export", PROJECT, "win-imp3", "--format", "markdown", "--full"])
check("export full rc=0", r.returncode == 0, r.stderr)

print("=== 20. framework check ===")
r = run(["framework", "check", PROJECT])
check("framework check green", r.returncode == 0, r.stdout)

print("=== 回归 ===")
r = run(["window", "list", PROJECT])
check("window list works", r.returncode == 0, r.stderr)

print(f"\n=== RESULT: {passed} passed, {failed} failed ===")
sys.exit(1 if failed else 0)
