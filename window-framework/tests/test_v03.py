#!/usr/bin/env python3
"""窗口群框架 v0.3 测试 — 工作流引擎 + workflow deploy + 自动快照（plan-v03 §v0.3.1 补充4 的 15 项）。
运行: python3 tests/test_v03.py
"""
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

SRC = Path(__file__).resolve().parent.parent / "src" / "framework.py"
TMP = Path(tempfile.mkdtemp(prefix="wf-v03-"))
PROJECT = "app"

passed = 0
failed = 0


def run(args, env_extra=None):
    env = dict(os.environ)
    env["CODEX_PROJECTS_ROOT"] = str(TMP)
    env["AGENT_MODE"] = "replay"  # 补充3：测试不烧 LLM
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


def write_proj_toml(project_root, stages_text):
    pt = project_root / "project.toml"
    text = pt.read_text(encoding="utf-8")
    text = text.replace("[workflow]\nstages = []", f"[workflow]\n{stages_text}".rstrip())
    pt.write_text(text, encoding="utf-8")


P = TMP / PROJECT
run(["project", "create", PROJECT, "--type", "software"])

print("=== S1: 工作流引擎 ===")
# 建 3 个窗口（arch / backend / review，全带 prompt + budget + gate）
for name, role, gate in [("arch", "架构", "shared/gates/arch-gate.sh"),
                          ("backend", "后端", "shared/gates/impl-gate.sh"),
                          ("review", "审查", "shared/gates/review-gate.sh")]:
    run(["window", "create", PROJECT, "--name", name, "--role", role, "--prompt", f"p-{name}"])
    # 写 gate 字段到 window.toml（deploy 会自动写，这里手动模拟）
    wt = P / "windows" / f"win-{name}" / "window.toml"
    text = wt.read_text(encoding="utf-8").replace('gate = ""', f'gate = "{gate}"')
    wt.write_text(text, encoding="utf-8")
    gp = P / gate
    gp.parent.mkdir(parents=True, exist_ok=True)
    gp.write_text("#!/bin/bash\necho VERIFY_PASS\nexit 0\n", encoding="utf-8")

write_proj_toml(P, """[[workflow.stages]]
id = "design"
trigger = "project_start"
windows = ["win-arch"]
gate = "auto:shared/gates/arch-gate.sh"

[[workflow.stages]]
id = "implementation"
trigger = "design:done"
windows = ["win-backend"]
gate = "auto:shared/gates/impl-gate.sh"

[[workflow.stages]]
id = "review"
trigger = "implementation:done"
windows = ["win-review"]
gate = "human:人类审核"
""")

# 1. 读取 stages（含 human gate → 首轮 rc=1 等待是设计行为）
r = run(["workflow", "start", PROJECT])
check("workflow start reaches human gate", r.returncode == 1, r.stdout + r.stderr)
# 2. design 自动启动（arch 从 pending → done 因为 replay）
check("arch started", "started win-arch" in r.stdout, r.stdout)
# 3. 串行：backend 在 design done 前不启动——replay 一次全跑完，检查顺序
check("design done before implementation", r.stdout.index("design done") < r.stdout.index("started win-backend"), r.stdout)
# 4. 自动 gate 通过推进
check("implementation started after gate", "started win-backend" in r.stdout)
# 5. review 是 human gate → 停在 stage
check("review waiting human", "waiting human gate" in r.stdout, r.stdout)
check("ALL STAGES DONE NOT printed", "ALL STAGES DONE" not in r.stdout)
# 6. 人工 approve review
r = run(["workflow", "gate", PROJECT, "review", "--approve"])
check("approve rc=0", r.returncode == 0)
r = run(["workflow", "start", PROJECT])
check("all done after approve", "ALL STAGES DONE" in r.stdout, r.stdout)

print("=== 7. 并行 stage ===")
run(["project", "create", "par", "--type", "software"])
for name in ["a", "b"]:
    run(["window", "create", "par", "--name", name, "--role", "r", "--prompt", "p"])
    wt = TMP / "par" / "windows" / f"win-{name}" / "window.toml"
    wt.write_text(wt.read_text().replace('gate = ""', 'gate = "shared/gates/g.sh"'), encoding="utf-8")
(TMP / "par" / "shared" / "gates").mkdir(parents=True, exist_ok=True)
(TMP / "par" / "shared" / "gates" / "g.sh").write_text("#!/bin/bash\nexit 0\n")
write_proj_toml(TMP / "par", """[[workflow.stages]]
id = "s1"
trigger = "project_start"
windows = ["win-a", "win-b"]
parallel = true
gate = "auto:shared/gates/g.sh"
""")
r = run(["workflow", "start", "par"])
check("parallel both started", "started win-a" in r.stdout and "started win-b" in r.stdout, r.stdout)
check("parallel stage done", "stage s1 done" in r.stdout, r.stdout)

print("=== S2: workflow deploy ===")
run(["project", "create", "dep", "--type", "software"])
# 造需求窗口对话（含 YAML）
conv_path = TMP / "dep" / "windows" / "win-req" / "conversation.jsonl"
run(["window", "create", "dep", "--name", "req", "--role", "需求", "--prompt", "p"])
conv_path.parent.mkdir(parents=True, exist_ok=True)
yaml_msg = '''聊完了。以下是窗口配置：

```yaml
project_type: software
goal: "博客 API"
windows:
  - id: "win-arch-01"
    role: "架构设计"
    prompt: "你是架构师"
    depends_on: []
    budget:
      max_steps: 20
      max_cost_cny: 0.2
      provider: "deepseek"
    outputs: ["shared/outputs/arch.md"]
    gate: "shared/gates/arch-gate.sh"
  - id: "win-backend-01"
    role: "后端开发"
    prompt: "你是后端"
    depends_on: ["win-arch-01"]
    budget:
      max_steps: 40
      max_cost_cny: 0.5
      provider: "deepseek"
    outputs: ["src/"]
    gate: "shared/gates/backend-gate.sh"
workflow_stages:
  - id: "design"
    trigger: "project_start"
    windows: ["win-arch-01"]
    gate: "human:人类审核"
  - id: "implementation"
    trigger: "design:done"
    windows: ["win-backend-01"]
    gate: "auto:shared/gates/backend-gate.sh"
```
'''
with open(conv_path, "a", encoding="utf-8") as f:
    f.write('{"t":"x","role":"assistant","content":"%s"}\n' % yaml_msg.replace('"', '\\"').replace('\n', '\\n'))

# 8. deploy 创建窗口
r = run(["workflow", "deploy", "dep", "win-req"])
check("deploy rc=0", r.returncode == 0, r.stdout + r.stderr)
# 9. 窗口数 = YAML 声明 2
check("2 windows created", (TMP / "dep" / "windows" / "win-arch-01").exists()
      and (TMP / "dep" / "windows" / "win-backend-01").exists(), r.stdout)
# 10. workflow 写入 project.toml
pt_text = (TMP / "dep" / "project.toml").read_text(encoding="utf-8")
check("stages written", 'id = "design"' in pt_text and 'id = "implementation"' in pt_text)
# 11. gate 脚本生成
check("gate scripts generated", (TMP / "dep" / "shared" / "gates" / "arch-gate.sh").exists()
      and (TMP / "dep" / "shared" / "gates" / "backend-gate.sh").exists())
# 15. deploy 后 framework check 全绿
r = run(["framework", "check", "dep"])
check("framework check after deploy", r.returncode == 0, r.stdout)

print("=== 11/12. deploy 拒绝缺字段 ===")
# 缺 budget 的 YAML → 拒绝
conv2 = TMP / "dep" / "windows" / "win-req2" / "conversation.jsonl"
run(["window", "create", "dep", "--name", "req2", "--role", "需求", "--prompt", "p"])
conv2.parent.mkdir(parents=True, exist_ok=True)
bad_yaml = '''```yaml
windows:
  - id: "win-bad-01"
    role: "x"
    prompt: "p"
    depends_on: []
    outputs: ["src/"]
    gate: "shared/gates/g.sh"
```
'''
with open(conv2, "a", encoding="utf-8") as f:
    f.write('{"t":"x","role":"assistant","content":"%s"}\n' % bad_yaml.replace('"', '\\"').replace('\n', '\\n'))
r = run(["workflow", "deploy", "dep", "win-req2"])
check("deploy rejects missing budget", r.returncode != 0 and "missing budget" in r.stdout, r.stdout)

print("=== 13. 自动快照 on stop ===")
run(["window", "create", PROJECT, "--name", "snapme", "--role", "r", "--prompt", "p"])
r = run(["window", "stop", PROJECT, "win-snapme"])
check("stop rc=0", r.returncode == 0, r.stderr)
snaps = list((P / ".snapshots").glob("win-snapme--*"))
check("auto snapshot on stop", len(snaps) == 1, f"{len(snaps)} snaps")

print("=== v0.1/v0.2 回归 ===")
r = run(["framework", "check", PROJECT])
check("framework check green", r.returncode == 0, r.stdout)

print(f"\n=== RESULT: {passed} passed, {failed} failed ===")
sys.exit(1 if failed else 0)
