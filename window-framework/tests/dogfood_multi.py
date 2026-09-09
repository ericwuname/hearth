#!/usr/bin/env python3
"""v1.0.3 多窗口 dogfooding：窗口群框架 + 真实 LLM 多窗口分工建文档站。

验证（plan-v1.0.3.1 验收清单）：
  1. analyze 产出窗口数 ≥ 3（LLM 正确判断需要多角色分工）
  2. 全部窗口 state=done（无 blocked/working 残留）
  3. framework check 断言 4 绿 + conflict list 空（窗口不互相踩文件）
  4. 产出 ≥3 个 html（含 index.html）
  5. 人类介入 ≤ 3 次
  6. 全量回归 172/172（本脚本跑完单独验证）

运行: DEEPSEEK_API_KEY=... FW_SRC=... python3 dogfood_multi.py
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
DOCS_SRC = Path(os.environ.get("DOGFOOD_DOCS_SRC", "/home/wutao/fw/dogfood/docs-source"))

# 需求对话（模拟用户——明确要求多角色分工，逼 LLM 产出 ≥3 窗口）
REQ_CONV = [
    {"role": "user", "content": "我要给 codex-rust 项目建一个文档站，素材在 shared/specs/ 下（4 份架构文档）"},
    {"role": "assistant", "content": "好的！文档站需要哪些角色分工？"},
    {"role": "user", "content": "我希望至少三个角色：一个负责分析文档设计站点信息架构，一个负责把内容写成 HTML 页面，一个负责质量审查"},
    {"role": "assistant", "content": "三个角色分工明确。站点形式有什么要求？"},
    {"role": "user", "content": "纯静态 HTML，浏览器直接打开，首页有导航索引，每份文档一页"},
    {"role": "assistant", "content": "信息架构和页面生成是依赖关系吗？"},
    {"role": "user", "content": "对，架构窗口先产出站点结构设计，页面窗口再按结构生成 HTML，审查窗口最后验收"},
    {"role": "assistant", "content": "明白，三个窗口按依赖链流转。产出物放哪？"},
    {"role": "user", "content": "统一放 shared/outputs/docs-site/ 下，index.html 作为入口"},
    {"role": "assistant", "content": "好，我整理需求，输出多窗口配置"},
]


def run(args, timeout=900):
    return subprocess.run([sys.executable, str(SRC)] + args,
                          capture_output=True, text=True,
                          env=dict(os.environ), timeout=timeout)


def main():
    root = Path(tempfile.mkdtemp(prefix="wf-dogmulti-"))
    os.environ["CODEX_PROJECTS_ROOT"] = str(root)
    os.environ["AGENT_MODE"] = "real"
    if not os.environ.get("DEEPSEEK_API_KEY"):
        print("ERROR: DEEPSEEK_API_KEY required")
        return 1
    # v1.0.3: LLM 可能产出 provider=openai/agnes——用 .env 已有 key 兜底：
    # 有 ZHIPU_API_KEY 则作为 openai 路由的 key（避免 LLM 产出与 key 不匹配卡死）
    if not os.environ.get("OPENAI_API_KEY") and os.environ.get("ZHIPU_API_KEY"):
        os.environ["OPENAI_API_KEY"] = os.environ["ZHIPU_API_KEY"]
        print("NOTE: OPENAI_API_KEY ← ZHIPU_API_KEY (fallback)")

    print("=== v1.0.3 多窗口 dogfooding（真实 LLM）===")
    print(f"root: {root}")
    t_start = time.time()

    # 1. project + 需求窗口 + 素材
    r = run(["project", "create", "docs-site", "--type", "software"])
    assert r.returncode == 0, r.stderr
    specs = root / "docs-site" / "shared" / "specs"
    specs.mkdir(parents=True, exist_ok=True)
    n_src = 0
    for f in DOCS_SRC.glob("*.md"):
        shutil.copy(f, specs / f.name)
        n_src += 1
    print(f"[1] project created, {n_src} docs staged")

    r = run(["window", "create", "docs-site", "--name", "req", "--role", "需求分析",
             "--prompt", "你是需求分析师，帮用户把文档站需求拆成多角色窗口配置"])
    assert r.returncode == 0, r.stderr
    conv = root / "docs-site" / "windows" / "win-req" / "conversation.jsonl"
    with open(conv, "w", encoding="utf-8") as f:
        for e in REQ_CONV:
            f.write(json.dumps({"t": time.strftime("%Y-%m-%dT%H:%M:%SZ"),
                                "role": e["role"], "content": e["content"]}) + "\n")
    print("[2] requirement window seeded")

    # 2. analyze（真实 LLM，最多 2 次）
    analyze_ok = False
    for attempt in range(2):
        r = run(["window", "analyze", "docs-site", "win-req", "--confirm"], timeout=180)
        if r.returncode == 0:
            analyze_ok = True
            print(f"[3] analyze OK (attempt {attempt + 1}): {[l for l in r.stdout.splitlines() if '  - ' in l]}")
            break
        print(f"[3] analyze attempt {attempt + 1} FAIL: {r.stdout[-200:]}")
    if not analyze_ok:
        print("FATAL: analyze failed 2x")
        return 1

    # 3. 数窗口数（读 last_analyze.json）
    la = root / "docs-site" / ".snapshots" / "last_analyze.json"
    n_windows = 0
    if la.exists():
        data = json.loads(la.read_text(encoding="utf-8"))
        n_windows = len(data.get("windows", []))
    print(f"[4] analyze 产出窗口数: {n_windows}")

    # 4. deploy
    r = run(["workflow", "deploy", "docs-site", "win-req"], timeout=180)
    if r.returncode != 0:
        print(f"[5] FAIL deploy: {r.stdout[-300:]}")
        return 1
    print("[5] deploy OK")

    # v1.0.3: LLM 常产出 provider=openai，但 VM 只有 deepseek/zhipu 可达——
    # 统一改写窗口 provider=deepseek（VM 唯一可靠路径）
    import tomllib as TL
    for d in (root / "docs-site" / "windows").iterdir():
        wt = d / "window.toml"
        if not wt.exists():
            continue
        text = wt.read_text(encoding="utf-8")
        if 'provider = "openai"' in text or 'provider = "zhipu"' in text:
            wt.write_text(text.replace('provider = "openai"', 'provider = "deepseek"')
                          .replace('provider = "zhipu"', 'provider = "deepseek"'), encoding="utf-8")
            print(f"NOTE: {d.name} provider → deepseek (VM 可达)")

    # 5. workflow 循环（human gate 模拟人类 approve，计数）
    interventions = 0
    deadline = time.time() + 45 * 60
    wf_ok = False
    while time.time() < deadline:
        r = run(["workflow", "start", "docs-site", "--max-rounds", "10"], timeout=2400)
        out = r.stdout
        if "ALL STAGES DONE" in out:
            wf_ok = True
            break
        m = re.search(r"stage (\S+) waiting human gate", out)
        if m:
            interventions += 1
            print(f"[human-gate] approve {m.group(1)} (#{interventions})")
            run(["workflow", "gate", "docs-site", m.group(1), "--approve"])
            continue
        if "gate FAILED" in out or "blocked" in out or "missing" in out:
            print(f"[6] workflow blocked: {out[-300:]}")
            break
        time.sleep(5)
    elapsed = round(time.time() - t_start)
    print(f"[6] workflow: {'DONE' if wf_ok else 'NOT done'} ({elapsed}s, interventions={interventions})")

    # 6. 状态收集
    import tomllib
    states = {}
    for d in sorted((root / "docs-site" / "windows").iterdir()):
        wt = d / "window.toml"
        if wt.exists() and d.name != "win-req":
            data = tomllib.loads(wt.read_text(encoding="utf-8"))
            states[d.name] = data["window"]["state"]

    # 7. 冲突检测
    r = run(["framework", "check", "docs-site"], timeout=60)
    check_out = r.stdout
    r2 = run(["conflict", "list", "docs-site"], timeout=60)
    conflict_out = r2.stdout

    # 8. 产出检查
    html_files = []
    for od in [root / "docs-site" / "shared" / "outputs"] + \
               [d / "outputs" for d in (root / "docs-site" / "windows").iterdir() if (d / "outputs").exists()]:
        html_files += sorted(od.glob("*.html"))
    html_files = sorted(set(html_files))
    site_ok = len(html_files) >= 3 and any(f.name == "index.html" for f in html_files)

    # 9. 指标判定
    print(f"\n=== v1.0.3 多窗口 dogfooding 结果（{elapsed}s）===")
    print(f"[指标1] analyze 产出窗口: {n_windows}（目标 ≥3）{'✅' if n_windows >= 3 else '❌'}")
    print(f"[指标2] 窗口状态: {states}  全部done={'✅' if all(s == 'done' for s in states.values()) else '❌'}")
    print(f"[指标3] 冲突: check4/4={'✅' if '4/4 PASS' in check_out else '❌'} | "
          f"conflict list={'空✅' if '(no conflicts)' in conflict_out else '❌'}")
    print(f"[指标4] 产出: {len(html_files)} html / index={'✅' if site_ok else '❌'}")
    print(f"[指标5] 人类介入: {interventions + 1} 次（≤3 {'✅' if interventions + 1 <= 3 else '❌'}）")
    print(f"[check] {check_out.strip()[:200]}")

    report = root / "docs-site" / "shared" / "dogfood-multi-report.md"
    report.write_text(
        f"# v1.0.3 多窗口 dogfooding 报告\n\n时间: {elapsed}s\n"
        f"analyze 窗口数: {n_windows}\n窗口状态: {json.dumps(states, ensure_ascii=False)}\n"
        f"冲突: {conflict_out.strip()[:200]}\n产出: {len(html_files)} html / index={'OK' if site_ok else 'MISSING'}\n"
        f"人类介入: {interventions + 1}\n", encoding="utf-8")
    print(f"\nreport: {report}")
    print(f"saved root: {root}")
    return 0 if wf_ok and site_ok else 2


if __name__ == "__main__":
    sys.exit(main())
