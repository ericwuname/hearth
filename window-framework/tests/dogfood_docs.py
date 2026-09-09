#!/usr/bin/env python3
"""dogfooding: 窗口群框架 + 真实 LLM 给自己建文档站（codex-docs）。

观测三指标（CHANGELOG 维护合同）：
  1. analyze 首胜率     目标 > 80%（3 次独立尝试）
  2. 全链路不卡死        目标 arch → backend → review 全部 done
  3. 人类介入次数        目标 ≤ 3（需求审核 1 次 + gate 审核 ≤ 2 次）

运行: DEEPSEEK_API_KEY=... FW_SRC=/home/wutao/fw/framework.py python3 dogfood_docs.py
素材: 从本地 docs/ 选取 4 份核心文档 → VM shared/specs/（agent 只读范围）
"""
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

SRC = Path(os.environ.get("FW_SRC", str(Path(__file__).resolve().parent.parent / "src" / "framework.py")))
DOCS_SRC = Path(os.environ.get("DOGFOOD_DOCS_SRC", "/home/wutao/wf/dogfood/docs-source"))

# 需求对话（模拟用户与需求分析师聊了 14 轮）
REQ_CONV = [
    {"role": "user", "content": "我想给 codex-rust 项目建一个文档站"},
    {"role": "assistant", "content": "好的！能说下素材和想要的形态吗？"},
    {"role": "user", "content": "素材是 docs 目录里的架构文档，有基因表达系统设计、版本迭代手册、全局全景、架构总结几份"},
    {"role": "assistant", "content": "这些文档在哪个目录？我确认下路径"},
    {"role": "user", "content": "在项目 shared/specs/ 下，我已经放好了"},
    {"role": "assistant", "content": "好的。想要什么形式的站点？"},
    {"role": "user", "content": "纯静态 HTML，不依赖框架，浏览器直接打开就能看"},
    {"role": "assistant", "content": "需要导航和索引吗？"},
    {"role": "user", "content": "要，首页有文档索引，每份文档一页，有侧边导航能跳转"},
    {"role": "assistant", "content": "风格上有什么偏好吗？深色还是浅色？"},
    {"role": "user", "content": "简洁专业就行，浅色，重点突出"},
    {"role": "assistant", "content": "明白。产出物是 shared/outputs/docs-site/ 下的 HTML 文件对吗？"},
    {"role": "user", "content": "对，index.html 作为入口，能离线打开"},
    {"role": "assistant", "content": "好，我整理一下需求，输出窗口配置"},
]


def run(args, timeout=600):
    return subprocess.run([sys.executable, str(SRC)] + args,
                          capture_output=True, text=True,
                          env=dict(os.environ), timeout=timeout)


def seed_requirement(root, name):
    """建需求窗口 + 注入对话。"""
    r = run(["window", "create", name, "--name", "req", "--role", "需求分析",
             "--prompt", "你是需求分析师，帮用户理清文档站需求"])
    assert r.returncode == 0, r.stderr
    conv = root / name / "windows" / "win-req" / "conversation.jsonl"
    with open(conv, "w", encoding="utf-8") as f:
        for e in REQ_CONV:
            f.write(json.dumps({"t": time.strftime("%Y-%m-%dT%H:%M:%SZ"),
                                "role": e["role"], "content": e["content"]}) + "\n")


def analyze_attempt(root, name, attempt_no):
    """一次完整 analyze 尝试（独立项目）。返回 (ok, stdout)。"""
    os.environ["CODEX_PROJECTS_ROOT"] = str(root)
    r = run(["project", "create", name, "--type", "software"])
    if r.returncode != 0:
        return False, f"project create failed: {r.stderr}"
    # 素材 → shared/specs/（agent 只读范围）
    proj = root / name
    specs = proj / "shared" / "specs"
    specs.mkdir(parents=True, exist_ok=True)
    n_src = 0
    for f in DOCS_SRC.glob("*.md"):
        shutil.copy(f, specs / f.name)
        n_src += 1
    if n_src == 0:
        return False, "no source docs found in DOCS_SRC"
    seed_requirement(root, name)
    # analyze --confirm（模拟人类审核确认 = 介入 1 次）
    t0 = time.time()
    r = run(["window", "analyze", name, "win-req", "--confirm"], timeout=180)
    dt = round(time.time() - t0)
    if r.returncode != 0:
        return False, f"analyze rc={r.returncode} ({dt}s): {r.stdout[-400:]}"
    wins = [l for l in r.stdout.splitlines() if "  - " in l]
    return True, f"analyze OK ({dt}s): {wins}"


def workflow_loop(root, name, timeout_min=45):
    """deploy + workflow 循环（human gate 模拟人类 approve 并计数）。"""
    interventions = 0
    deadline = time.time() + timeout_min * 60

    r = run(["workflow", "deploy", name, "win-req"], timeout=180)
    if r.returncode != 0:
        return {"ok": False, "why": f"deploy failed: {r.stdout[-300:]}"}

    # workflow 循环：start → 若 human gate → approve（计数）→ 继续
    # v1.0.1: 真实 LLM 单窗口 max_steps 轮 × ~20s/轮 ≈ 10-15min，timeout 放宽到 2400s
    while time.time() < deadline:
        r = run(["workflow", "start", name, "--max-rounds", "10"], timeout=2400)
        out = r.stdout
        if "ALL STAGES DONE" in out:
            return {"ok": True, "interventions": interventions, "tail": out[-600:]}
        m = re.search(r"stage (\S+) waiting human gate", out)
        if m:
            sid = m.group(1)
            interventions += 1
            print(f"[human-gate] approve stage {sid} (intervention #{interventions})")
            run(["workflow", "gate", name, sid, "--approve"])
            continue
        if "gate FAILED" in out or "blocked" in out:
            return {"ok": False, "interventions": interventions,
                    "why": f"blocked: {out[-400:]}"}
        # 无进展（deps waiting 或 agent 还在跑）
        time.sleep(5)

    return {"ok": False, "interventions": interventions, "why": "TIMEOUT (卡死)"}


def collect_states(root, name):
    import tomllib
    states = {}
    wdir = root / name / "windows"
    if wdir.exists():
        for d in sorted(wdir.iterdir()):
            wt = d / "window.toml"
            if wt.exists():
                data = tomllib.loads(wt.read_text(encoding="utf-8"))
                states[d.name] = data["window"]["state"]
    return states


def main():
    root = Path(tempfile.mkdtemp(prefix="wf-dog-"))
    os.environ["AGENT_MODE"] = "real"
    if not os.environ.get("DEEPSEEK_API_KEY"):
        print("ERROR: DEEPSEEK_API_KEY required (mode A)")
        return 1

    print("=== dogfooding: 窗口群框架 + 真实 LLM 给自己建文档站 ===")
    print(f"root: {root} | SRC: {SRC} | 素材: {DOCS_SRC}")
    t_start = time.time()

    # ── 指标 1: analyze 首胜率（3 次独立尝试）──
    analyze_results = []
    for i in range(3):
        pname = f"codex-docs-a{i}"
        print(f"\n--- analyze attempt {i + 1}/3 ({pname}) ---")
        ok, msg = analyze_attempt(root, pname, i)
        analyze_results.append(ok)
        print(msg)
        if ok:
            break  # 成功即进入全链路（首胜率=成功时尝试次数）

    attempts = len(analyze_results)
    successes = sum(1 for x in analyze_results if x)
    first_win_rate = successes / attempts * 100
    print(f"\n[指标1] analyze 首胜率: {successes}/{attempts} = {first_win_rate:.0f}%")
    if successes == 0:
        print("FATAL: all analyze attempts failed — aborting")
        return 1

    # 用成功那次的项目继续全链路
    ok_name = f"codex-docs-a{analyze_results.index(True)}"

    # ── 指标 2+3: 全链路不卡死 + 人类介入次数 ──
    print(f"\n--- workflow 全链路 ({ok_name}) ---")
    wf = workflow_loop(root, ok_name, timeout_min=45)
    elapsed = round(time.time() - t_start)
    states = collect_states(root, ok_name)

    # ── 产出检查（v1.0.1: 扫描所有窗口 outputs + shared/outputs 的 html）──
    html_files = []
    for od in [root / ok_name / "shared" / "outputs"] + \
               [d / "outputs" for d in (root / ok_name / "windows").iterdir() if (d / "outputs").exists()]:
        html_files += sorted(od.glob("*.html"))
    html_files = sorted(set(html_files))
    site_ok = len(html_files) >= 3 and any(f.name == "index.html" for f in html_files)
    site_size = sum(f.stat().st_size for f in html_files) if html_files else 0

    print(f"\n=== dogfooding 结果（{elapsed}s）===")
    print(f"[指标1] analyze 首胜率: {first_win_rate:.0f}% ({successes}/{attempts})  目标>80%")
    print(f"[指标2] 全链路: {'✅ DONE' if wf['ok'] else '❌ ' + wf.get('why','')}  目标全 done")
    print(f"[指标3] 人类介入: {wf.get('interventions', 0) + 1} 次（需求审核1 + gate {wf.get('interventions', 0)}）  目标≤3")
    print(f"[产出] docs-site: {len(html_files)} html, {site_size}B, index={'OK' if site_ok else 'MISSING'}")
    print(f"[状态] {states}")

    report = root / ok_name / "shared" / "dogfood-report.md"
    report.write_text(
        f"# dogfooding 报告（真实 LLM 文档站）\n\n时间: {elapsed}s\n"
        f"analyze 首胜率: {first_win_rate:.0f}% ({successes}/{attempts})\n"
        f"全链路: {'OK' if wf['ok'] else wf.get('why','')}\n"
        f"人类介入: {wf.get('interventions', 0) + 1} 次\n"
        f"站点: {len(html_files)} html / {site_size}B / index={'OK' if site_ok else 'MISSING'}\n"
        f"窗口状态: {json.dumps(states, ensure_ascii=False)}\n",
        encoding="utf-8")
    print(f"\nreport: {report}")
    print(f"saved root: {root}")
    return 0 if wf["ok"] and site_ok else 2


if __name__ == "__main__":
    sys.exit(main())
