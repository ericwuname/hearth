#!/usr/bin/env python3
"""Hearth SimUser Analyzer — 12 detectors (P4-REVALIDATION-01 Node 01 §3.3).

输入 = 终端侧会话日志（用户可见 Reality，含 ANSI 与裸 tracing 行）。
输出 = findings JSONL（每条：detector/evidence/severity/line/no）。
判据全部**确定性**（字符串/计数），模型"感觉"不作为证据。

file_issue dry-run（砺裁决）：detector 2/7/8/9 + projection_leak 的输出结构与
未来 file_issue JSONL schema 同构（category/severity/evidence 指针/description）。

七分纪律（总包 §19）：findings 归 DRIVER-INDUCED 的可能性由调用方对照
driver events 判——本文件只报告确定性与位置。
"""
from __future__ import annotations

import json
import re
import sys
from typing import List, Optional

# 预编译模式
RE_PROJECTION_LEAK = re.compile(
    r"^\d{4}-\d{2}-\d{2}T[0-9:.]+Z?\s+(ERROR|WARN)\s+(agent_core|sandbox|planner|tools_builtin|llm_openai|codex_cli)")
RE_T4 = re.compile(r"stalled: 2 consecutive replans produced identical TaskGraph")
RE_TASK_FAILED = re.compile(r"Task failed")
RE_TASK_COMPLETED = re.compile(r"Task completed")
RE_DONE = re.compile(r"[✓✗]\s+Done \((\d+) steps\)")
RE_GOAL_REVISION = re.compile(r"目标已修订（revision (\d+)）")
RE_TRUNCATED = re.compile(r"(\.\.\.|…|\[\d+ chars truncated\])\s*$")
RE_GIVEUP = re.compile(r"give_up")
RE_OVERRIDDEN = re.compile(r"GIVE_UP_OVERRIDDEN")
RE_INTROSPECT_127 = re.compile(r"introspect: 未找到命令|bash: 行 1: introspect")
RE_REVISION_EXPLOSION = re.compile(r"revision (\d+)")
RE_ANAPHORA = re.compile(r"(刚才那个|上一轮|你刚才说的|之前那个|同一个问题|还记得吗)")
RE_ESCALATION = re.compile(r"(需要我|请你|帮我决定|等您|待您|等待用户|InteractionRequest)")
RE_REPETITION = re.compile(r"^(.{20,120})\1+$")  # 同文重复块（吸引子放大器特征）


def _findings() -> List[dict]:
    return []


def _add(out: List[dict], detector: str, line_no: int, evidence: str,
         severity: str, note: str = "") -> None:
    out.append({
        "detector": detector,
        "line": line_no,
        "evidence": evidence.strip()[:220],
        "severity": severity,
        "note": note,
    })


def detect_projection_leak(lines: List[str], out: List[dict]) -> None:
    """裸 tracing 行直达用户终端（RC38/P0-A；RC51-A isolation）。severity=high。"""
    for i, l in enumerate(lines, 1):
        if RE_PROJECTION_LEAK.match(l.strip()):
            _add(out, "projection_leak", i, l, "high",
                 note="file_issue 同构：category=projection_leak")


def detect_consecutive_failure_run(lines: List[str], out: List[dict],
                                   threshold: int = 5) -> None:
    """连续 N 轮 failed（RC52 吸引子；N≥5 报警）。severity=high。"""
    run = 0
    start_no = 0
    for i, l in enumerate(lines, 1):
        if RE_TASK_FAILED.search(l):
            if run == 0:
                start_no = i
            run += 1
        elif RE_TASK_COMPLETED.search(l):
            if run >= threshold:
                _add(out, "consecutive_failure_run", start_no,
                     f"连续 {run} 轮 failed（L{start_no}-{i}）", "high",
                     note="RC52 吸引子特征")
            run = 0
    if run >= threshold:
        _add(out, "consecutive_failure_run", start_no,
             f"连续 {run} 轮 failed（至文件尾）", "high")


def detect_t4_stall_burst(lines: List[str], out: List[dict]) -> None:
    """T4 同图停滞计数与分布（RC52/RC32 相关；阈值敏感性数据源）。severity=medium。"""
    hits = [i for i, l in enumerate(lines, 1) if RE_T4.search(l)]
    if len(hits) >= 2:
        _add(out, "t4_stall_burst", hits[0],
             f"T4 identical-TaskGraph ×{len(hits)}（首现 L{hits[0]}）", "medium",
             note=f"分布={hits[:10]}")
    elif hits:
        _add(out, "t4_stall_burst", hits[0], "T4 ×1", "low")


def detect_revision_explosion(lines: List[str], out: List[dict]) -> None:
    """goal_revision 数异常增长（RC33/RC39）。severity=medium。"""
    revs = [(i, int(m.group(1))) for i, l in enumerate(lines, 1)
            for m in [RE_GOAL_REVISION.search(l)] if m]
    if len(revs) >= 2:
        delta = revs[-1][1] - revs[0][1]
        if delta >= 3:
            _add(out, "revision_explosion", revs[0][0],
                 f"revision 增长 {delta}（{revs[0][1]}→{revs[-1][1]}）", "medium")


def detect_compaction_adjacency(lines: List[str], out: List[dict],
                                window: int = 200) -> None:
    """失败点距压缩/切片标记 ≤window 行（RC5/RC37 放大器证据）。severity=medium。"""
    compaction = [i for i, l in enumerate(lines, 1)
                  if "[compacted 会话摘要]" in l or "[history note]" in l]
    failures = [i for i, l in enumerate(lines, 1) if RE_TASK_FAILED.search(l)]
    for f in failures:
        for c in compaction:
            if 0 < abs(f - c) <= window:
                _add(out, "compaction_adjacency", f,
                     f"失败 L{f} 距压缩标记 L{c}（{abs(f-c)} 行）", "medium",
                     note="放大器时序证据")
                break


def detect_truncated_output(lines: List[str], out: List[dict]) -> None:
    """输出以 …/truncated 标记结尾（RC35 截断族——有无标记都算，标记本身=已发生）。"""
    for i, l in enumerate(lines, 1):
        if RE_TRUNCATED.search(l) and len(l) > 80:
            _add(out, "truncated_output", i, l[-120:], "low")


def detect_giveup_completed_coexist(lines: List[str], out: List[dict]) -> None:
    """同 run 内 give_up 与 completed 共存且无 OVERRIDDEN 痕迹（F9/拦截链可观测性）。
    P0 Q-C 已判 FALSE POSITIVE 一次（判定轨迹 vs 终态）——此处继续计数供分布观察。"""
    giveup = [i for i, l in enumerate(lines, 1) if RE_GIVEUP.search(l)]
    overridden = [i for i, l in enumerate(lines, 1) if RE_OVERRIDDEN.search(l)]
    completed = [i for i, l in enumerate(lines, 1) if RE_TASK_COMPLETED.search(l)]
    for c in completed:
        near_giveup = [g for g in giveup if 0 < c - g <= 60]
        if near_giveup and not any(0 < c - o <= 60 for o in overridden):
            _add(out, "giveup_completed_coexist", c,
                 f"completed L{c} 附近有 give_up L{near_giveup[-1]} 无 OVERRIDDEN",
                 "low", note="P0 Q-C 已有 FALSE POSITIVE 先例——分布观察")


def detect_tool_misroute(lines: List[str], out: List[dict]) -> None:
    """结构化工具名出现在 bash 命令串（RC53；exit 127 特征）。severity=medium。"""
    for i, l in enumerate(lines, 1):
        if RE_INTROSPECT_127.search(l):
            _add(out, "tool_misroute", i, l, "medium",
                 note="file_issue 同构：category=tool_misroute")


def detect_introspect_failure(lines: List[str], out: List[dict]) -> None:
    """内部工具失败率按工具名分桶（RC24-B 连带）。severity=low。"""
    fails = [i for i, l in enumerate(lines, 1)
             if re.search(r"工具出错.*?(introspect|status|civ_note)", l)]
    if fails:
        _add(out, "introspect_failure", fails[0], f"内部工具失败 ×{len(fails)}", "low")


def detect_user_visible_completion(lines: List[str], out: List[dict]) -> None:
    """Done 行后无可读正文（BUG-012/P0-C：分析内容只在报告文件）。severity=medium。"""
    for i, l in enumerate(lines, 1):
        if RE_DONE.search(l):
            nxt = lines[i:i+2]
            joined = " ".join(nxt)
            if "执行报告" in joined and len(joined.strip()) < 120:
                _add(out, "user_visible_completion", i,
                     f"Done 后仅报告指针无正文（L{i+1}-{i+2}）", "medium",
                     note="P0-C completion projection completeness")


def detect_anaphora_resolution_fail(lines: List[str], out: List[dict]) -> None:
    """指代消解失败线索（砺 S5 前移）：指代型输入紧跟 failed/重复澄清。severity=low。"""
    for i, l in enumerate(lines, 1):
        if RE_ANAPHORA.search(l):
            window = " ".join(lines[i:i+40])
            if RE_TASK_FAILED.search(window):
                _add(out, "anaphora_resolution_fail", i, l[:120], "low",
                     note="指代型输入 40 行内出现 failed——线索非判定")


def detect_escalation_run(lines: List[str], out: List[dict]) -> None:
    """升级/等待用户信号聚集（砺 S5 前移；RC24-B 连带）。severity=low。"""
    hits = [i for i, l in enumerate(lines, 1) if RE_ESCALATION.search(l)]
    if len(hits) >= 2:
        _add(out, "escalation_run", hits[0], f"升级信号 ×{len(hits)}", "low",
             note=f"分布={hits[:8]}")


def detect_repetition_amplifier(lines: List[str], out: List[dict]) -> None:
    """同文重复块（吸引子放大器特征——盲测 16 行 Debug 外泄同文重复）。severity=medium。"""
    for i, l in enumerate(lines, 1):
        if len(l) > 240 and RE_REPETITION.match(l):
            _add(out, "repetition_amplifier", i, l[:120], "medium")


DETECTORS = [
    ("projection_leak", detect_projection_leak),
    ("consecutive_failure_run", detect_consecutive_failure_run),
    ("t4_stall_burst", detect_t4_stall_burst),
    ("revision_explosion", detect_revision_explosion),
    ("compaction_adjacency", detect_compaction_adjacency),
    ("truncated_output", detect_truncated_output),
    ("giveup_completed_coexist", detect_giveup_completed_coexist),
    ("tool_misroute", detect_tool_misroute),
    ("introspect_failure", detect_introspect_failure),
    ("user_visible_completion", detect_user_visible_completion),
    ("anaphora_resolution_fail", detect_anaphora_resolution_fail),
    ("escalation_run", detect_escalation_run),
    ("repetition_amplifier", detect_repetition_amplifier),
]


def analyze(text: str) -> List[dict]:
    lines = text.split("\n")
    out: List[dict] = []
    for _, fn in DETECTORS:
        fn(lines, out)
    return sorted(out, key=lambda d: d["line"])


def main() -> None:
    if len(sys.argv) < 2:
        print("usage: analyzer.py <session.log> [--json]", file=sys.stderr)
        sys.exit(1)
    text = open(sys.argv[1], encoding="utf-8", errors="replace").read()
    findings = analyze(text)
    if "--json" in sys.argv:
        print(json.dumps(findings, ensure_ascii=False, indent=1))
    else:
        for f in findings:
            print(f"L{f['line']:>5} [{f['severity']:>6}] {f['detector']}: {f['evidence'][:100]}")
        print(f"-- {len(findings)} findings, {len(DETECTORS)} detectors --")


if __name__ == "__main__":
    main()
