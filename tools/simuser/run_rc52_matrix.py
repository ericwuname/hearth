#!/usr/bin/env python3
"""P4-REVALIDATION Node 03 — RC52 因果实验矩阵 runner（PTY driver 版）。
条件：A fresh 继续型 ×5 / B contaminated 继续型 ×5 / C resume 继续型 ×3。
每跑采集：终端侧全量 log + driver events + planner dump（HEARTH_DEBUG_PLANNER_INPUT=1）。
判定矩阵：fresh 败→RC47 族主因；fresh 过+contaminated/resume 败→会话污染主因；全同→task type。
"""
import json
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from driver import HearthDriver, DriverConfig, REPL_PROMPT

HOME = "/home/wutao"
OUT = "/home/wutao/fa/p4"
PREFIX = [
    "现在怎么样啊，身体通不通",
    "木有特定的检查项，你自己先自检一下，看看",
    "不用检查代码库，就是看你能不能做哪些事，住户体验，你住在hearth这个harness上",
    "2026-08-31T05:00:49.591654Z ERROR agent_core::r#loop: do_plan_inner failed error=stalled: 2 consecutive replans produced identical TaskGraph (4 nodes) — semantic progress absent, giving up early (Tier3 T4)",
    "你的完整自测计划是什么，怎么设计的",
    "那你继续跑吧",
    "继续跑吧",
    "现在是否偏离了，我说继续跑吧，但是你目前输出的内容是：**T12 观察第 4 轮（新数据）**：✓ Done (14 steps)",
    "那你继续吧",
    "上一轮放弃了，✗ Done (5 steps) 这个原因是什么",
    "你的本轮回复是，✓ Done (21 steps)。就这个✓ Done (21 steps)。但是我不知道你发生了什么，我需要猜你的结果，这个是不是一个问题。",
    "你登记到台账吧。",
    "现在还有什么可以测试的吗",
]
CONT = "继续你的提议吧"
TASK = "创建 p4_smoke.txt，内容写 P4_OK，写完宣告完成"

# R-1（P4 Node 13 仪器修复，Node03 v1.1 附录 A 授权内）：session id 提取——
# 旧法 `reports/<36>/` 正则在终端只打 8 位短 id 时恒 None → uuid 空 → resume
# 必败 → C 条件 BLOCKED（旧数据 67% 作废）。新法 = sessions 目录文件系统 diff：
# chat 前后快照对比，新增/变化的 .jsonl 即本次会话（文件名 = 36 位 uuid）。
SESS_DIR = os.path.expanduser("~/.config/hearth/sessions")


def _session_snapshot() -> dict:
    if not os.path.isdir(SESS_DIR):
        return {}
    out = {}
    for f in os.listdir(SESS_DIR):
        if f.endswith(".jsonl"):
            p = os.path.join(SESS_DIR, f)
            out[f[: -len(".jsonl")]] = os.path.getmtime(p)
    return out


def _newest_changed_session(before: dict) -> str:
    """返回 chat 后新增/修改的会话 id（mtime 最新者）；无则空串。"""
    changed = {
        sid: m for sid, m in _session_snapshot().items() if before.get(sid) != m
    }
    if not changed:
        return ""
    return max(changed.items(), key=lambda kv: kv[1])[0]


def env() -> dict:
    return {
        "HEARTH_ALLOW_NO_CGROUP": "1",
        "HEARTH_DEBUG_PLANNER_INPUT": "1",
    }


def run_a(i: int) -> dict:
    """A fresh 继续型：全新 REPL → 小任务完成 → 继续型请求。"""
    d = HearthDriver(DriverConfig(["hearth", "repl"], cwd=HOME, env_extra=env()))
    d.start()
    try:
        d.expect([REPL_PROMPT], 60)
        t1 = d.send_and_wait_turn(TASK)
        t2 = d.send_and_wait_turn(CONT)
        return {"cond": f"A{i}", "task_terminal": t1, "continue_terminal": t2,
                "session_ids": d.run.session_ids}
    finally:
        try:
            d.send("/quit")
        except Exception:
            pass
        time.sleep(2)
        d.close()
        _dump(d, f"p4_A{i}")


def run_b(i: int) -> dict:
    """B contaminated 继续型：逐字重放 run-001..012 → 同一继续型请求。"""
    d = HearthDriver(DriverConfig(["hearth", "repl"], cwd=HOME, env_extra=env()))
    d.start()
    try:
        d.expect([REPL_PROMPT], 60)
        t = None
        for line in PREFIX:
            t = d.send_and_wait_turn(line)
        result = {"cond": f"B{i}", "prefix_last_terminal": t,
                  "continue_terminal": d.send_and_wait_turn(CONT),
                  "session_ids": d.run.session_ids}
        try:
            d.send("/quit")
        except Exception:
            pass
        return result
    finally:
        time.sleep(2)
        d.close()
        _dump(d, f"p4_B{i}")


def run_c(i: int) -> dict:
    """C resume 继续型：chat 小任务 → resume 会话发继续型（resume 语义按 T3）。"""
    before = _session_snapshot()
    d0 = HearthDriver(DriverConfig(["hearth", "chat", TASK], cwd=HOME, env_extra=env()))
    d0.start()
    try:
        d0.wait_turn(600)
    finally:
        time.sleep(1)
        d0.close()
        _dump(d0, f"p4_C{i}-chat")
    # R-1 修复：sessions 目录 diff 取 36 位 uuid（旧正则 reports/<36>/ 在终端
    # 只打 8 位短 id 时恒 None → C 条件 BLOCKED）。旧正则保留为 fallback。
    uuid = _newest_changed_session(before)
    if not uuid:
        import re
        txt = d0.capture()
        m = re.search(r"reports/([0-9a-f-]{36})/", txt)
        if m:
            uuid = m.group(1)
    # DRIVER-INDUCED 五规则之 session id 非空断言：空 uuid 不允许继续跑
    assert uuid, (
        f"C{i}: session id 提取失败（sessions diff + reports 正则均空）"
        "——按 DRIVER-INDUCED 规则中止本条件，不得记 timeout 口径"
    )
    d = HearthDriver(DriverConfig(["hearth", "resume", uuid, CONT], cwd=HOME, env_extra=env()))
    d.start()
    try:
        t = d.wait_turn(600)
        return {"cond": f"C{i}", "task_terminal": "chat", "continue_terminal": t,
                "session_ids": [uuid]}
    finally:
        time.sleep(1)
        d.close()
        _dump(d, f"p4_C{i}")


def _dump(d: HearthDriver, tag: str) -> None:
    os.makedirs(OUT, exist_ok=True)
    with open(f"{OUT}/{tag}.log", "w", encoding="utf-8") as f:
        f.write(d.capture())
    with open(f"{OUT}/{tag}.events.json", "w", encoding="utf-8") as f:
        json.dump(d.run.events, f, ensure_ascii=False, indent=1)


def main() -> None:
    os.makedirs(OUT, exist_ok=True)
    results = []
    plan = ([("A", i) for i in range(1, 6)]
            + [("B", i) for i in range(1, 6)]
            + [("C", i) for i in range(1, 4)])
    for cond, i in plan:
        fn = {"A": run_a, "B": run_b, "C": run_c}[cond]
        try:
            r = fn(i)
        except Exception as exc:  # 单跑失败不阻断矩阵
            r = {"cond": f"{cond}{i}", "error": str(exc)[:200]}
        results.append(r)
        print(json.dumps(r, ensure_ascii=False), flush=True)
        with open(f"{OUT}/results.json", "w", encoding="utf-8") as f:
            json.dump(results, f, ensure_ascii=False, indent=1)
    print("MATRIX_DONE")


if __name__ == "__main__":
    main()
