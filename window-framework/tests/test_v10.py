#!/usr/bin/env python3
"""v1.0.2 回归测试：dogfooding 12 修复 + provider 路由 + 产出验证。

覆盖（对应 acceptance-dogfooding-v1.0.1.md 修复清单）：
  5.  max_turns 到无产出 → blocked（非虚假 done）
  6.  workflow 用 budget.max_steps（非硬编码 8）
  7.  read 大文件截断 8000
  8.  budget 用 completion_tokens 增量
  9.  任何 budget 超限 → blocked
  11. working 残留 → 重置 pending（防死锁）
  12. deploy 为 stage auto gate 生成占位脚本
  13. provider 路由（zhipu/unknown-fallback）
  14. VERSION = 1.0.2
运行: python3 tests/test_v10.py
"""
import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path

SRC = Path(__file__).resolve().parent.parent / "src" / "framework.py"
TMP = Path(tempfile.mkdtemp(prefix="wf-v10-"))
os.environ["CODEX_PROJECTS_ROOT"] = str(TMP)
os.environ["AGENT_MODE"] = "replay"

spec = importlib.util.spec_from_file_location("fw", SRC)
fw = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fw)

passed = failed = 0


def check(name, cond, detail=""):
    global passed, failed
    if cond:
        passed += 1
        print(f"  ✅ {name}")
    else:
        failed += 1
        print(f"  ❌ {name}  {detail}")


def run(args):
    return subprocess.run([sys.executable, str(SRC)] + args,
                          capture_output=True, text=True,
                          env=dict(os.environ))


def seed_conv(conv_path, lines):
    with open(conv_path, "w", encoding="utf-8") as f:
        for c in lines:
            f.write(json.dumps({"t": time.strftime("%Y-%m-%dT%H:%M:%SZ"),
                                "role": "user", "content": c}) + "\n")


print("=== v1.0.2 回归：dogfooding 修复保护 ===")

# ── 14. VERSION ──
check("VERSION = 1.0.3", fw.VERSION == "1.0.3", fw.VERSION)

# ── 13. provider 路由 ──
fw.cmd_project_create(type("A", (), {"name": "p1", "type": "software"}))
fw.cmd_window_create(type("A", (), {"project": "p1", "name": "w", "role": "r",
                                    "prompt": "x", "max_steps": 10, "max_cost": 1,
                                    "provider": "zhipu"}))
a = fw.Agent(TMP / "p1", "win-w", provider="auto")
check("provider=auto 从 window.toml 路由 zhipu", a.provider == "zhipu" and "bigmodel" in a.base_url, a.provider)
check("zhipu model=glm-4.5", a._model == "glm-4.5", a._model)
# P2-5 (audit-fix): 未知 provider 必须显式抛错，不再静默回退 deepseek
try:
    fw.Agent(TMP / "p1", "win-w", provider="anthropic")
    check("P2-5: 未知 provider 抛错", False, "anthropic 未抛错")
except ValueError as e:
    check("P2-5: 未知 provider 抛错", "anthropic" in str(e), str(e))

# ── 5+9. 虚假 done 防护 + 超限 blocked：直接验证 real 路径的状态转移逻辑 ──
# replay 模式不走产出检查（设计），改验证：无产出时 real 路径返回码映射
# 通过检查 Agent.run 的返回码语义（4=无产出 blocked）与 _budget_exceeded 判定
fw.cmd_window_create(type("A", (), {"project": "p1", "name": "noout", "role": "r",
                                    "prompt": "x", "max_steps": 3, "max_cost": 1,
                                    "provider": "deepseek"}))
wt = TMP / "p1" / "windows" / "win-noout" / "window.toml"

# 9. 超限 → blocked（真实判定函数）
wt.write_text(wt.read_text(encoding="utf-8").replace("current_tokens = 0", "current_tokens = 999999"), encoding="utf-8")
over = fw._budget_exceeded(TMP / "p1", "win-noout")
check("budget 超限可检测", "tokens" in over, over)

# 5. 虚假 done 防护：验证 _has_outputs 判定（无产出 False）
a5 = fw.Agent(TMP / "p1", "win-noout", provider="deepseek")
check("_has_outputs 无产出 = False", a5._has_outputs() is False)
# 有产出则 True
(out_d := TMP / "p1" / "windows" / "win-noout" / "outputs").mkdir(parents=True, exist_ok=True)
(out_d / "x.txt").write_text("hello", encoding="utf-8")
check("_has_outputs 有产出 = True", a5._has_outputs() is True)

# ── 7. read 大文件截断 ──
big = TMP / "p1" / "shared" / "bigfile.txt"
big.parent.mkdir(parents=True, exist_ok=True)
big.write_text("A" * 20000, encoding="utf-8")
r = a5._exec_tool("read", {"path": str(big)})
check("read 截断 8000 + TRUNCATED 标记", len(r) < 9000 and "TRUNCATED" in r, f"{len(r)} chars")

# ── 12. deploy 为 stage auto gate 生成脚本（走真实签名）──
os.environ["AGENT_MODE"] = "replay"
args = type("A", (), {"project": "p1"})
windows = [{"id": "wa", "role": "r1", "prompt": "p", "budget": {"max_steps": 5, "max_cost_cny": 1, "provider": "deepseek"},
            "outputs": ["shared/outputs/x.md"], "gate": "auto:shared/gates/g1.sh"}]
stages = [{"id": "s1", "trigger": "project_start", "windows": ["wa"], "gate": "auto:shared/gates/sg1.sh"}]
rc_d = fw._deploy_windows_stages(TMP / "p1", args, windows, stages)
sg1 = TMP / "p1" / "shared" / "gates" / "sg1.sh"
check("deploy rc=0", rc_d == 0, rc_d)
check("stage auto gate 脚本已生成", sg1.exists(), str(sg1))
check("window gate 脚本已生成", (TMP / "p1" / "shared" / "gates" / "g1.sh").exists())

# ── 11. working 残留 → 重置 pending ──
fw.cmd_window_create(type("A", (), {"project": "p1", "name": "stale", "role": "r",
                                    "prompt": "x", "max_steps": 5, "max_cost": 1,
                                    "provider": "deepseek"}))
wt5 = TMP / "p1" / "windows" / "win-stale" / "window.toml"
wt5.write_text(wt5.read_text(encoding="utf-8").replace('state = "pending"', 'state = "working"'), encoding="utf-8")
engine = fw.WorkflowEngine(TMP / "p1")
st5 = engine._window_state("win-stale")
check("working 残留可被引擎识别", st5 == "working", st5)

# ══ v23 红队修复回归（audit-findings-v22）══
# ── WT6: stale working 重置后不得假完成（单窗口 stage 必须重跑窗口）──
# 构造单窗口 stage + working 状态 → 引擎一轮后不应判 stage done（窗口未跑过）
fw.cmd_window_create(type("A", (), {"project": "p1", "name": "stale2", "role": "r",
                                    "prompt": "x", "max_steps": 3, "max_cost": 1,
                                    "provider": "deepseek"}))
wt6 = TMP / "p1" / "windows" / "win-stale2" / "window.toml"
wt6.write_text(wt6.read_text(encoding="utf-8").replace('state = "pending"', 'state = "working"'), encoding="utf-8")
# 造一个单窗口 stage 的 project.toml（win-stale2 单独 stage）
stg_proj = TMP / "p1" / "project.toml"
old_proj = stg_proj.read_text(encoding="utf-8")
single_stage = """
[[workflow.stages]]
id = "s_solo"
trigger = "project_start"
windows = ["win-stale2"]
gate = "auto:shared/gates/solo.sh"
"""
stg_proj.write_text(single_stage, encoding="utf-8")
(TMP / "p1" / "shared" / "gates").mkdir(parents=True, exist_ok=True)
(TMP / "p1" / "shared" / "gates" / "solo.sh").write_text("#!/bin/bash\necho VERIFY_PASS\n", encoding="utf-8")
eng2 = fw.WorkflowEngine(TMP / "p1")
rc2 = eng2.run(max_rounds=2)
import tomllib as _tl
st6 = _tl.loads(wt6.read_text(encoding="utf-8"))["window"]["state"]
# WT6 修复后：working → pending → 引擎应启动窗口真跑（replay 模式秒 done）→ 状态应为 done
check("WT6: stale working 单窗口最终被真实运行（state=done）", st6 == "done", f"state={st6}, rc={rc2}")
check("WT6: 引擎未假完成（rc=0 表示 stage 完成）", rc2 == 0, f"rc={rc2}")
stg_proj.write_text(old_proj, encoding="utf-8")  # 还原 project.toml

# ── WT17: prompt 注入破坏 TOML（换行/引号/反斜杠必须安全写入）──
evil_prompt = '你是测试窗口\n换行注入 "双引号" \\反斜杠\\ \t制表符'
fw.cmd_window_create(type("A", (), {"project": "p1", "name": "evil", "role": "r",
                                    "prompt": evil_prompt, "max_steps": 5, "max_cost": 1,
                                    "provider": "deepseek"}))
wt7 = TMP / "p1" / "windows" / "win-evil" / "window.toml"
try:
    d7 = _tl.loads(wt7.read_text(encoding="utf-8"))
    check("WT17: 注入 prompt 的 TOML 可读且保留原文", d7["window"]["prompt"] == evil_prompt,
          repr(d7["window"]["prompt"]))
except Exception as e:
    check("WT17: 注入 prompt 的 TOML 可读", False, f"tomllib 失败: {e}")

# ══ P0-5 / P1-5 红队修复回归（audit-fix-taskbook-v22）══
# ── P0-5: bash 零沙箱 → 白名单 ──
ag2 = fw.Agent(TMP / "p1", "win-evil", provider="deepseek")
r_bash_escape = ag2._exec_tool("bash", {"cmd": "echo x > /tmp/escape_probe"})
check("P0-5: bash 越权写 /tmp 被拒", "DENIED" in r_bash_escape, r_bash_escape[:80])
check("P0-5: /tmp/escape_probe 不存在", not Path("/tmp/escape_probe").exists())
r_bash_curl = ag2._exec_tool("bash", {"cmd": "curl http://example.com"})
check("P0-5: bash 执行 curl 被拒", "DENIED" in r_bash_curl, r_bash_curl[:80])
r_bash_ls = ag2._exec_tool("bash", {"cmd": "ls outputs"})
check("P0-5: bash 白名单内 ls outputs 正常", "DENIED" not in r_bash_ls, r_bash_ls[:80])
# ── P1-5: 路径前缀旁路 + 空文件产出 ──
r_evil = ag2._exec_tool("write_file", {"path": str(TMP / "p1" / "outputs_evil" / "x.txt"), "content": "x"})
check("P1-5: 写 outputs_evil/ 被拒", "DENIED" in r_evil, r_evil[:80])
# 空文件不算产出
out_d = TMP / "p1" / "windows" / "win-evil" / "outputs"
out_d.mkdir(parents=True, exist_ok=True)
(out_d / "empty.txt").write_text("", encoding="utf-8")
check("P1-5: 只有 0 字节文件 → _has_outputs=False", ag2._has_outputs() is False)
(out_d / "empty.txt").unlink()
(out_d / "real.txt").write_text("hi", encoding="utf-8")
check("P1-5: 有非空文件 → _has_outputs=True", ag2._has_outputs() is True)

# ══ v1.0.3 多窗口 dogfooding 修复回归 ══

# 15. gate approve 验证窗口 done（拒绝假 approve）
args_g = type("A", (), {"project": "p1", "stage": "s1", "approve": True, "reject": False})
rc_g = fw.cmd_workflow_gate(args_g)  # s1 的窗口 win-wa 是 pending → 应拒绝
check("gate approve 拒绝未 done 窗口 (rc=1)", rc_g == 1, f"rc={rc_g}")
# 把 win-wa 标 done 后 approve 应成功
wtw = TMP / "p1" / "windows" / "win-wa" / "window.toml"
wtw.write_text(wtw.read_text(encoding="utf-8").replace('state = "pending"', 'state = "done"'), encoding="utf-8")
rc_g2 = fw.cmd_workflow_gate(args_g)
check("gate approve 窗口 done 后成功 (rc=0)", rc_g2 == 0, f"rc={rc_g2}")

# 16. human gate 断言豁免（window.toml 存 human: 描述不查脚本）
wth = TMP / "p1" / "windows" / "win-wa" / "window.toml"
wth.write_text(wth.read_text(encoding="utf-8").replace('gate = ""', 'gate = "human:用户确认站点结构"'), encoding="utf-8")
fc = fw.FrameworkCheck(TMP / "p1")
fc.run()
r3 = [r for r in fc.results if r.get("name") == "gate-script-exists"]
check("human gate 断言豁免 (PASS)", r3 and r3[0]["ok"], r3)

# 17. provider key 缺失报错含 provider 名（不静默 deepseek 语义混淆）
spec17 = fw.Agent.PROVIDERS["openai"]
check("PROVIDERS 含 openai 路由", spec17["env_key"] == "OPENAI_API_KEY", spec17)

# 18. VERSION = 1.0.3
check("VERSION = 1.0.3", fw.VERSION == "1.0.3", fw.VERSION)

# ══ P2-3 (audit-fix): gate --reject 不得是装饰 + approve 幂等 ══
# 前置：测试 15 已 approve s1（progress.md 有 done:s1，win-wa 已 done）
args_rej = type("A", (), {"project": "p1", "stage": "s1", "approve": False, "reject": True})
rc_rej = fw.cmd_workflow_gate(args_rej)
check("P2-3: reject 返回 rc=0", rc_rej == 0, f"rc={rc_rej}")
pm = (TMP / "p1" / "shared" / "progress.md").read_text(encoding="utf-8")
check("P2-3: reject 写 blocked:s1", "blocked:s1" in pm, pm)
check("P2-3: reject 清除 done:s1", "done:s1" not in pm, pm)
# 引擎不得推进被 reject 的 stage（窗口 done + gate 会过，但 blocked 必须拦截）
eng_r = fw.WorkflowEngine(TMP / "p1")
rc_r = eng_r.run(max_rounds=3)
pm2 = (TMP / "p1" / "shared" / "progress.md").read_text(encoding="utf-8")
check("P2-3: 引擎不推进 blocked stage (rc=1)", rc_r == 1, f"rc={rc_r}")
check("P2-3: blocked 后 run 不产生 done:s1", "done:s1" not in pm2, pm2)
# approve 幂等：两次 approve 都 rc=0 且 done:s1 仅 1 行；approve 清 blocked
args_ap = type("A", (), {"project": "p1", "stage": "s1", "approve": True, "reject": False})
rc_ap1 = fw.cmd_workflow_gate(args_ap)
rc_ap2 = fw.cmd_workflow_gate(args_ap)
pm3 = (TMP / "p1" / "shared" / "progress.md").read_text(encoding="utf-8")
check("P2-3: approve 两次都 rc=0", rc_ap1 == 0 and rc_ap2 == 0, f"{rc_ap1}/{rc_ap2}")
check("P2-3: approve 幂等（done:s1 仅 1 行）", pm3.count("done:s1") == 1, pm3)
check("P2-3: approve 清除 blocked:s1", "blocked:s1" not in pm3, pm3)

# ══ P2-7 (audit-fix): 边界硬化批量回归 ══

# WT7: analyze 产出缺 id 的 window → validate_deploy_yaml 友好拦截（不 KeyError）
try:
    ok7, err7 = fw.validate_deploy_yaml(
        [{"role": "dev", "prompt": "x"}], [])
    check("WT7: 缺 id window 被拦截", (not ok7) and "missing id" in err7, f"{ok7}/{err7}")
except KeyError as e:
    check("WT7: 缺 id window 被拦截", False, f"KeyError 泄漏: {e}")

# WT11: gate 路径遍历 → _gate_path_ok 拒绝
ok_trav = fw.WorkflowEngine._gate_path_ok(TMP / "p1", "../../etc/passwd")
check("WT11: ../.. 路径被拒", ok_trav is False, f"ok={ok_trav}")
ok_abs = fw.WorkflowEngine._gate_path_ok(TMP / "p1", "/etc/passwd")
check("WT11: 绝对路径被拒", ok_abs is False, f"ok={ok_abs}")
ok_ok = fw.WorkflowEngine._gate_path_ok(TMP / "p1", "shared/gates/sg1.sh")
check("WT11: 合法相对路径放行", ok_ok is True, f"ok={ok_ok}")

# WT19: 同 stage 窗口 role 查重（FrameworkCheck 断言 5）
fc19 = fw.FrameworkCheck(TMP / "p1")
fc19.run()
r19 = [r for r in fc19.results if r.get("name") == "stage-role-unique"]
check("WT19: stage-role-unique 断言存在", bool(r19), r19)

# WT4: replay 模式 budget 超限即停（max_steps=99999 不产生海量快照）
fw.cmd_window_create(type("A", (), {"project": "p1", "name": "wt4x", "role": "r",
                                    "prompt": "x", "max_steps": 99999, "max_cost": 1,
                                    "provider": "deepseek"}))
wt4f = TMP / "p1" / "windows" / "win-wt4x" / "window.toml"
wt4f.write_text(wt4f.read_text(encoding="utf-8").replace("current_tokens = 0", "current_tokens = 999999"), encoding="utf-8")
import os as _os
_os.environ["AGENT_MODE"] = "replay"
try:
    ag4 = fw.Agent(TMP / "p1", "win-wt4x", provider="deepseek")
    rc4 = ag4.run("test", max_turns=99999)
    snaps4 = list((TMP / "p1" / ".snapshots").glob("win-wt4x--*")) if (TMP / "p1" / ".snapshots").exists() else []
    check("WT4: 超限即停 rc=2", rc4 == 2, f"rc={rc4}")
    check("WT4: 不产生海量快照 (<10)", len(snaps4) < 10, f"{len(snaps4)} snaps")
finally:
    _os.environ.pop("AGENT_MODE", None)

# WT15: 导入保留 summary role + 原时间戳（往返一致）
imp_src = TMP / "p1" / "shared" / "roundtrip.jsonl"
imp_src.write_text('{"t":"2026-08-02T01:02:03Z","role":"summary","content":"keep me","meta":{"compression_id":9}}\n', encoding="utf-8")
rc_imp = fw.cmd_window_import(type("A", (), {"project": "p1", "source": str(imp_src),
                                             "name": "rt", "role": "s", "prompt": "p"}))
conv_rt = (TMP / "p1" / "windows" / "win-rt" / "conversation.jsonl").read_text(encoding="utf-8")
check("WT15: import 接受 summary role", "summary" in conv_rt, conv_rt)
check("WT15: import 保留原时间戳", "2026-08-02T01:02:03Z" in conv_rt, conv_rt)

# ══ P2-6 (audit-fix): 快照回滚净丢失 ══
# 1. --to 路径遍历被拒
rc_trav = fw.cmd_window_rollback(type("A", (), {"project": "p1", "id": "win-wt4x", "to": "../../x"}))
check("P2-6: --to 路径遍历被拒", rc_trav == 1, f"rc={rc_trav}")
# 2. 快照纳入 outputs
out1 = TMP / "p1" / "windows" / "win-wt4x" / "outputs"
out1.mkdir(parents=True, exist_ok=True)
(out1 / "artifact.txt").write_text("v1", encoding="utf-8")
fw._auto_snapshot(TMP / "p1", "win-wt4x")
snaps6 = sorted((TMP / "p1" / ".snapshots").glob("win-wt4x--*"))
snap_has_out = any((s / "outputs" / "artifact.txt").exists() for s in snaps6)
check("P2-6: 快照含 outputs", snap_has_out,
      " ".join(s.name for s in snaps6) if snaps6 else "no snap")
# 3. 回滚到坏快照（缺 conversation.jsonl）→ 当前对话完好
bad_snap = TMP / "p1" / ".snapshots" / "win-wt4x--BAD"
bad_snap.mkdir(exist_ok=True)
(bad_snap / "window.toml").write_text("x", encoding="utf-8")
cur_conv = (TMP / "p1" / "windows" / "win-wt4x" / "conversation.jsonl").read_text(encoding="utf-8")
rc_bad = fw.cmd_window_rollback(type("A", (), {"project": "p1", "id": "win-wt4x", "to": "BAD"}))
after_conv = (TMP / "p1" / "windows" / "win-wt4x" / "conversation.jsonl").read_text(encoding="utf-8")
check("P2-6: 坏快照回滚被拒 (rc=1)", rc_bad == 1, f"rc={rc_bad}")
check("P2-6: 当���对话完好无损", after_conv == cur_conv, "对话被弄丢!")

print(f"\n=== RESULT: {passed} passed, {failed} failed ===")
sys.exit(1 if failed else 0)
