#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
log_analyze.py —— 通用服务日志分析小工具

用法:
    python3 log_analyze.py <logfile> [--top N]

默认对 app.log 分析。以后换了新日志，只要行格式形如:
    2026-09-01 08:04:24 [ERROR] scheduler: null pointer in handler
    即 [级别] 模块: 消息，本工具即可直接复用（自动识别级别与模块）。
也可用 --fmt 手动指定“级别在方括号内、模块在级别后”的其它格式。

功能:
    - 总行数 / 各级别计数（INFO/WARN/ERROR/DEBUG/FATAL，未匹配则归为 OTHER）
    - ERROR 与 WARN 条数
    - ERROR 按模块分布（含占 ERROR 的比例）
    - WARN 按模块分布
    - 综合判断“问题最严重”的模块：按 ERROR 数为主、WARN 数为次的排名
    - 高频错误消息 Top N（同一模块内相同的错误文案）
"""

import argparse
import re
import sys
from collections import Counter, defaultdict

# 兼容多种写法: [ERROR] module: msg  / 2026... [ERROR] module: ... / 纯文本含 ERROR module:
# 取“级别（方括号或大写字母）+ 其后第一个 标识: ”作为模块。
LINE_RE = re.compile(
    r"""
    ^(?P<date>\d{4}-\d{2}-\d{2})\s+(?P<time>\d{2}:\d{2}:\d{2})\s+   # 时间戳（可选）
    \[(?P<level>[A-Za-z]+)\]\s*                                          # [级别]
    (?P<module>[A-Za-z0-9_\-]+)\s*:\s*                                   # 模块:
    (?P<msg>.*)$
    """,
    re.VERBOSE,
)

# 宽松匹配：没有时间戳、或级别未用方括号的情况
LOOSE_RE = re.compile(
    r"""
    (?P<level>(ERROR|WARN|WARNING|INFO|DEBUG|FATAL|TRACE))
    \]?\s+
    (?P<module>[A-Za-z0-9_\-]+)\s*:\s*
    (?P<msg>.*)$
    """,
    re.VERBOSE,
)

LEVELS = ["ERROR", "WARN", "WARNING", "INFO", "DEBUG", "FATAL", "TRACE", "OTHER"]
# 归一化
NORM = {"WARNING": "WARN"}


def parse_line(line: str):
    """返回 (level_norm, module, msg) 或 None（无法解析）。"""
    m = LINE_RE.match(line)
    if not m:
        m2 = LOOSE_RE.search(line)
        if not m2:
            return None
        level = m2.group("level")
        module = m2.group("module")
        msg = m2.group("msg").strip()
    else:
        level = m.group("level").upper()
        module = m.group("module")
        msg = m.group("msg").strip()
    level = NORM.get(level, level)
    return level, module, msg


def analyze(path: str, top: int = 5):
    total = 0
    matched = 0
    level_cnt = Counter()
    module_total = Counter()
    module_error = Counter()
    module_warn = Counter()
    error_msgs = defaultdict(Counter)   # module -> Counter(msg)
    warn_msgs = defaultdict(Counter)

    with open(path, "r", encoding="utf-8", errors="replace") as f:
        for line in f:
            total += 1
            line = line.rstrip("\n")
            if not line.strip():
                continue
            p = parse_line(line)
            if p is None:
                level_cnt["OTHER"] += 1
                continue
            matched += 1
            level, module, msg = p
            level_cnt[level] += 1
            module_total[module] += 1
            if level == "ERROR":
                module_error[module] += 1
                error_msgs[module][msg] += 1
            elif level == "WARN":
                module_warn[module] += 1
                warn_msgs[module][msg] += 1

    err_total = sum(module_error.values())
    warn_total = sum(module_warn.values())

    print("=" * 60)
    print(f"日志文件: {path}")
    print(f"总行数: {total}   (可解析 {matched}, 未匹配 {total - matched})")
    print("-" * 60)

    print("【级别分布】")
    for lv in ["ERROR", "WARN", "INFO", "FATAL", "DEBUG", "TRACE", "OTHER"]:
        if level_cnt.get(lv):
            print(f"  {lv:<8} {level_cnt[lv]:>6}  " + "#" * min(40, level_cnt[lv] // max(1, total // 40)))
    print(f"  ERROR 共 {err_total} 条,  占全部日志 {err_total / total * 100:.1f}%")
    print(f"  WARN  共 {warn_total} 条,  占全部日志 {warn_total / total * 100:.1f}%")
    print("-" * 60)

    print("【ERROR 按模块分布】")
    for m, c in module_error.most_common():
        pct = c / err_total * 100 if err_total else 0
        print(f"  {m:<12} {c:>4}  ({pct:5.1f}%)  " + "#" * int(pct / 2))

    print("\n【WARN 按模块分布】")
    for m, c in module_warn.most_common():
        pct = c / warn_total * 100 if warn_total else 0
        print(f"  {m:<12} {c:>4}  ({pct:5.1f}%)  " + "#" * int(pct / 2))
    print("-" * 60)

    # 综合排名: 主指标 ERROR 数, 次指标 WARN 数
    ranked = sorted(
        module_total.keys(),
        key=lambda m: (module_error.get(m, 0), module_warn.get(m, 0)),
        reverse=True,
    )
    print("【模块问题严重度排名】(按 ERROR 数为主、WARN 数为次)")
    for i, m in enumerate(ranked, 1):
        print(f"  #{i} {m:<12} ERROR={module_error.get(m,0):>3}  WARN={module_warn.get(m,0):>3}  总记录={module_total[m]}")

    worst = ranked[0] if ranked else None
    if worst:
        print("\n结论: 问题最严重的模块是 “%s” (ERROR %d 条 / WARN %d 条)"
              % (worst, module_error.get(worst, 0), module_warn.get(worst, 0)))
        print(f"该模块 Top 高频错误:")
        for msg, c in error_msgs[worst].most_common(top):
            print(f"    [{c:>3}] {msg}")
    print("=" * 60)


if __name__ == "__main__":
    ap = argparse.ArgumentParser(description="服务日志分析工具")
    ap.add_argument("logfile", nargs="?", default="app.log", help="日志文件路径 (默认 app.log)")
    ap.add_argument("--top", type=int, default=5, help="每个模块展示的 Top N 高频错误 (默认 5)")
    args = ap.parse_args()
    try:
        analyze(args.logfile, args.top)
    except FileNotFoundError:
        sys.exit(f"错误: 找不到文件 {args.logfile}")
