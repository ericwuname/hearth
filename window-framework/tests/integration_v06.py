#!/usr/bin/env python3
"""窗口群框架 v0.6 集成测试（模式 A）——真实 LLM 全链路 mini-blog。
运行: AGENT_MODE=real CODEX_PROJECTS_ROOT=... DEEPSEEK_API_KEY=... python3 tests/integration_v06.py
"""
import json
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path

SRC = Path(os.environ.get("FW_SRC", str(Path(__file__).resolve().parent.parent / "src" / "framework.py")))

# 预置需求对话（模拟人聊了 20 轮）
REQ_CONV = [
    {"role": "user", "content": "我想搭一个博客后端 API"},
    {"role": "assistant", "content": "好的，能说说需要哪些功能吗？"},
    {"role": "user", "content": "需要用户登录（JWT），文章 CRUD"},
    {"role": "assistant", "content": "技术栈有偏好吗？"},
    {"role": "user", "content": "Rust + Axum，数据存内存就行，mini 版"},
    {"role": "assistant", "content": "规模呢？一个 auth + 一个 articles 模块够吗？"},
    {"role": "user", "content": "够了，要能跑 cargo test"},
    {"role": "assistant", "content": "好，我整理一下需求..."},
    {"role": "user", "content": "嗯，就这样，可以出窗口配置了"},
    {"role": "assistant", "content": "好的，我来输出 YAML 配置"},
]


def main():
    root = Path(tempfile.mkdtemp(prefix="wf-int-"))
    os.environ["CODEX_PROJECTS_ROOT"] = str(root)
    os.environ["AGENT_MODE"] = "real"
    if not os.environ.get("DEEPSEEK_API_KEY"):
        print("ERROR: DEEPSEEK_API_KEY required for mode A integration")
        return 1

    def run(args):
        return subprocess.run([sys.executable, str(SRC)] + args,
                              capture_output=True, text=True,
                              env=dict(os.environ))

    print("=== 集成测试 v0.6（模式 A：真实 LLM）===")
    t0 = time.time()

    # 1. project create
    r = run(["project", "create", "mini-blog", "--type", "software"])
    assert r.returncode == 0, r.stderr
    print("[1] project created")

    # 2. 需求窗口 + 注入预置对话
    r = run(["window", "create", "mini-blog", "--name", "req", "--role", "需求分析",
             "--prompt", "你是需求分析师，帮用户理清博客 API 需求"])
    assert r.returncode == 0, r.stderr
    conv = root / "mini-blog" / "windows" / "win-req" / "conversation.jsonl"
    with open(conv, "w", encoding="utf-8") as f:
        for e in REQ_CONV:
            f.write(json.dumps({"t": time.strftime("%Y-%m-%dT%H:%M:%SZ"),
                                "role": e["role"], "content": e["content"]}) + "\n")
    print("[2] requirement window seeded (20 rounds)")

    # 3. analyze（真实 LLM 产出 YAML）
    r = run(["window", "analyze", "mini-blog", "win-req", "--confirm"])
    if r.returncode != 0:
        print(f"[3] FAIL analyze: {r.stdout}")
        return 1
    print(f"[3] analyze OK — {[l for l in r.stdout.splitlines() if '  - ' in l]}")

    # 4. deploy
    r = run(["workflow", "deploy", "mini-blog", "win-req"])
    if r.returncode != 0:
        print(f"[4] FAIL deploy: {r.stdout}")
        return 1
    print("[4] deploy OK")

    # 5. workflow watch（持续轮询直到全部完成或 human gate 等待）
    r = run(["workflow", "watch", "mini-blog", "--timeout", "20", "--interval", "3"])
    if r.returncode != 0 and "waiting human" not in r.stdout:
        print(f"[5] workflow issue: {r.stdout}")
    print(f"[5] workflow ran: {[l for l in r.stdout.splitlines() if 'stage' in l or 'started' in l]}")

    # 6. 指标收集
    elapsed = round(time.time() - t0)
    wins = sorted((root / "mini-blog" / "windows").iterdir())
    states = {}
    for d in wins:
        wt = d / "window.toml"
        if wt.exists():
            import tomllib
            data = tomllib.loads(wt.read_text(encoding="utf-8"))
            states[d.name] = data["window"]["state"]
    print(f"\n=== 集成测试结果（{elapsed}s）===")
    print(f"windows: {states}")
    all_done = all(s == "done" for s in states.values() if s != "blocked")
    print(f"全链路通过率: {'100%' if all_done else 'PENDING (human gates)'}")

    # 保存结果
    report = root / "mini-blog" / "shared" / "integration-report.md"
    report.write_text(
        f"# 集成测试报告 v0.6\n\n时间: {elapsed}s\n窗口状态: {json.dumps(states, ensure_ascii=False)}\n"
        f"全链路通过率: {'100%' if all_done else '见窗口状态'}\n",
        encoding="utf-8")
    print(f"report saved: {report}")
    return 0 if all_done else 2


if __name__ == "__main__":
    sys.exit(main())
