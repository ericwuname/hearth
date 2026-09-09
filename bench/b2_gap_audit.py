#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""B2-A (backend-intelligence): 10 真实任务 gap 质量审计。

每任务: hearth chat 直跑（deepseek）→ 读 ~/hearth/observer/<sid>.jsonl 的
plan_draft 事件原始 JSON → 审计每条 gap 的 from/why、blocking=false 的 assume。
输出审计表 JSON + 汇总。
"""
import json, os, re, subprocess, sys, time, glob

HOME = os.environ.get("AUDIT_HOME", "/tmp/hearth_audit")
HEARTH = "/home/wutao/codex/target/release/hearth"
KEY = "sk-28d7376b6b7d4c339f5023e5a0aedbf1"
os.makedirs(HOME, exist_ok=True)
os.makedirs(f"{HOME}/works", exist_ok=True)

TASKS = [
    ("T01 写代码", "写一个 Rust 函数 add(a, b) 返回 a+b，并写一个单元测试验证。"),
    ("T02 改bug", "修复这个文件里的 bug：fn max(a: i32, b: i32) -> i32 { if a > b { a } else { a } } 返回了错误的值。"),
    ("T03 查资料", "解释 Rust 的 ownership（所有权）是什么，和垃圾回收有什么不同？"),
    ("T04 重构", "把这段重复代码重构为函数：let a = vec![1,2,3]; let b = vec![4,5,6]; let s1: i32 = a.iter().sum(); let s2: i32 = b.iter().sum();"),
    ("T05 配置", "写一个 Cargo.toml，包含 serde 和 serde_json 依赖，并设置 edition 2021。"),
    ("T06 测试", "为这个函数写单元测试覆盖边界情况：fn divide(a: f64, b: f64) -> f64 { a / b }"),
    ("T07 解释概念", "用简单的话解释 async/await 是什么，什么时候该用？"),
    ("T08 多文件", "创建一个 Rust 项目结构：src/lib.rs 定义 greet 函数，src/main.rs 调用它打印问候语。"),
    ("T09 模糊意图", "优化这个项目。"),
    ("T10 跨语言", "用 Rust 实现 Python 的列表推导式 [x*x for x in range(10)] 的等价功能。"),
]

def run_task(name, goal):
    """跑 hearth chat，返回 (plan_draft_json, sid, exit_log)"""
    work = f"{HOME}/works/{name.split()[0]}"
    os.makedirs(work, exist_ok=True)
    env = dict(os.environ)
    env.update({"HOME": HOME, "HEARTH_API_KEY": KEY})
    # --budget 6: 够规划 + 初步执行；每任务独立工作目录（隔离产物）
    cmd = f"cd {work} && timeout 180 {HEARTH} chat {json.dumps(goal)} --budget 6"
    proc = subprocess.run(["bash", "-c", cmd], env=env, capture_output=True, text=True, timeout=200)
    log = proc.stdout + proc.stderr
    # 找最新 AI 侧 jsonl（本任务 sid）
    sid_files = sorted(glob.glob(f"{HOME}/hearth/observer/*.jsonl"),
                       key=os.path.getmtime, reverse=True)
    plan = None
    sid = None
    for f in sid_files:
        if "human-" in f:
            continue
        try:
            lines = open(f).read().strip().splitlines()
            for line in lines:
                ev = json.loads(line)
                if ev.get("type") == "plan_draft":
                    plan = ev
                    sid = ev.get("span_id") or os.path.basename(f).replace(".jsonl", "")
                    break
        except Exception:
            continue
        if plan:
            break
    return plan, sid, log

def audit(plan, name):
    """审计 plan_draft 的 gap 质量。返回 (ok, issues[])"""
    issues = []
    if not plan:
        return False, ["NO plan_draft 事件（任务未完成规划）"]
    gaps_found = plan.get("gaps_found", 0)
    gaps_to_ask = plan.get("gaps_to_ask", 0)
    auto_assumed = plan.get("auto_assumed") or []
    ask_details = plan.get("gaps_to_ask_details") or []
    steps = plan.get("steps") or []
    # 1. from+why 完整性
    for g in ask_details:
        if not g.get("from") or not g.get("why"):
            issues.append(f"blocking gap 缺 from/why: {g}")
        if not g.get("from"):
            issues.append(f"blocking gap 无 from: {g}")
    for a in auto_assumed:
        if not a.get("from") or not a.get("why"):
            issues.append(f"non-blocking gap 缺 from/why: {a}")
        # 2. blocking=false ⇒ assume 100%
        if a.get("assume") is not True:
            issues.append(f"non-blocking gap assume 非 true: {a}")
    # 3. 无通用问题（为问而问）
    for g in ask_details:
        why = (g.get("why") or "").lower()
        if "有什么要求" in why or "有什么问题" in why or "请说明" in why and "具体" not in why:
            issues.append(f"通用问题（为问而问）: {g}")
    ok = len(issues) == 0
    return ok, issues

def main():
    results = []
    for name, goal in TASKS:
        print(f"=== {name} 跑起 ===", flush=True)
        plan, sid, log = run_task(name, goal)
        ok, issues = audit(plan, name)
        # 留日志（端到端证据）
        with open(f"{HOME}/works/{name.split()[0]}/run.log", "w") as f:
            f.write(log)
        entry = {
            "task": name,
            "goal": goal,
            "ok": ok,
            "issues": issues,
            "gaps_found": (plan or {}).get("gaps_found", 0),
            "gaps_to_ask": (plan or {}).get("gaps_to_ask", 0),
            "auto_assumed_count": len((plan or {}).get("auto_assumed") or []),
            "ask_details": (plan or {}).get("gaps_to_ask_details") or [],
            "auto_assumed": (plan or {}).get("auto_assumed") or [],
            "steps_count": len((plan or {}).get("steps") or []),
            "sid": sid,
        }
        results.append(entry)
        status = "PASS" if ok else "FAIL"
        print(f"  [{status}] {name}: gaps={entry['gaps_found']} 待问={entry['gaps_to_ask']} 假设={entry['auto_assumed_count']} issues={issues}", flush=True)
        # 输出审计明细（写入统一文件）
        with open(f"{HOME}/audit_results.jsonl", "a") as f:
            f.write(json.dumps(entry, ensure_ascii=False) + "\n")
        time.sleep(1)
    # 汇总
    passed = sum(1 for r in results if r["ok"])
    print(f"\n=== 审计汇总: {passed}/10 PASS ===")
    for r in results:
        if not r["ok"]:
            print(f"  FAIL {r['task']}: {r['issues']}")

if __name__ == "__main__":
    main()
