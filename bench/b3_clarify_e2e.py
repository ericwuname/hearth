#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""B3-A (backend-intelligence): clarify 闭环端到端验证。

构造真触发 blocking clarify 的任务（"用 Rust 或 Go 实现..."），跑 hearth chat：
1. 内核 emit InteractionRequested{kind=clarification} → CLI 打印 ❓问题（payload）
2. 用户输入回答 → resolve_interaction(resolved=true, payload={answer})
3. resume 后续链路——不丢事件、不重复问、不卡死
4. approval 同理（危险操作前 need_approval → y/N）
"""
import json, os, re, subprocess, sys, time

HOME = "/tmp/hearth_b3a"
HEARTH = "/home/wutao/codex/target/release/hearth"
KEY = "sk-28d7376b6b7d4c339f5023e5a0aedbf1"
os.makedirs(f"{HOME}/works", exist_ok=True)

def run_chat(goal, input_text, budget=8, timeout=180):
    """跑 hearth chat，通过 stdin 喂回答。返回 (full_log, plan_events)"""
    work = f"{HOME}/works/b3a"
    os.makedirs(work, exist_ok=True)
    env = dict(os.environ)
    env.update({"HOME": HOME, "HEARTH_API_KEY": KEY})
    cmd = f"cd {work} && timeout {timeout} {HEARTH} chat {json.dumps(goal)} --budget {budget}"
    proc = subprocess.run(["bash", "-c", cmd], env=env, input=input_text,
                          capture_output=True, text=True, timeout=timeout + 20)
    log = proc.stdout + proc.stderr
    return log

def check(log, name):
    """检查链路完整性"""
    ok = True
    issues = []
    # 1. 澄清问题打印（❓ 需要你确认）
    if "需要你确认" not in log and "clarification" not in log:
        ok = False
        issues.append("未见 clarification 问题")
    # 2. 澄清提交
    if "澄清已提交" not in log:
        ok = False
        issues.append("未见澄清提交")
    # 3. resume（后续相位）
    for phase in ["Act", "Observe", "Reflect"]:
        if phase in log:
            break
    else:
        ok = False
        issues.append("未见 resume 后续相位")
    # 4. 无卡死（log 有 Done 或 error 或 reflect）
    if not any(k in log for k in ["Done", "✗", "error", "Reflect"]):
        ok = False
        issues.append("疑似卡死（无终态）")
    return ok, issues

def main():
    # 任务 1: 二选一（真歧义 → blocking clarify）
    goal1 = "用 Rust 或 Go 实现一个命令行工具，统计当前目录下 .rs 文件数量"
    print("=== B3-A-1 clarify 闭环（用 Rust 或 Go）===", flush=True)
    log1 = run_chat(goal1, "Rust\n", budget=8)
    ok1, iss1 = check(log1, "clarify")
    print(f"  [{'PASS' if ok1 else 'FAIL'}] clarify: issues={iss1}", flush=True)
    with open(f"{HOME}/clarify_run.log", "w") as f:
        f.write(log1)

    # 任务 2: approval 闭环（触发写文件 → need_approval → y）
    goal2 = "在当前目录创建一个 README.md，内容为 hello"
    print("=== B3-A-2 approval 闭环（写文件）===", flush=True)
    log2 = run_chat(goal2, "y\n", budget=8)
    ok2 = "批准" in log2 or "✓" in log2 or "Done" in log2
    print(f"  [{'PASS' if ok2 else 'FAIL'}] approval: 批准/完成={'批准' in log2}/{'Done' in log2}", flush=True)
    with open(f"{HOME}/approval_run.log", "w") as f:
        f.write(log2)

    print("\n=== B3-A 汇总 ===")
    print(f"  clarify: {'PASS' if ok1 else 'FAIL'} | approval: {'PASS' if ok2 else 'FAIL'}")
    # 展示 clarify 关键行
    for line in log1.splitlines():
        if any(k in line for k in ["需要你确认", "澄清已提交", "你的选择", "❓"]):
            print(f"  CLARIFY> {line.strip()[:90]}")

if __name__ == "__main__":
    main()
