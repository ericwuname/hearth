#!/usr/bin/env python3
"""gate_window 自检套件（D-132）：直接测本项目的**结果判定函数** `parse_suite_result`。

为什么要有这一套：`gate_window.py` 是全项目的门禁尺子，它自己 fail-open 就等于
没有门禁。历史验收只验证过两种红——「失败套件→exit 1」「缺失套件→exit 1」——
**没验证过**「套件先打印 `N passed, 0 failed` 再崩（退出码非零）」：旧实现只看解析出的
数字、退出码仅在"一个数字都解析不出"时才参与 ⇒ 这种情形被判 **PASS**（尺子自己失守，
与 D-123/D-127 的"防线 fail-open"同族）。本套件把该判定钉死。

运行: python3 tests/test_gate_window.py
"""
import importlib.util
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

passed = 0
failed = 0


def check(name, cond, detail=""):
    global passed, failed
    if cond:
        passed += 1
        print(f"  [PASS] {name}")
    else:
        failed += 1
        print(f"  [FAIL] {name} — {detail}")


# 以文件路径加载 gate_window（它不是包；顶层只有定义 + `__main__` 守卫，导入无副作用）
_spec = importlib.util.spec_from_file_location("gate_window", ROOT / "gate_window.py")
gw = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(gw)


def summary(n_passed, n_failed):
    return f"=== RESULT: {n_passed} passed, {n_failed} failed ===\n"


print("=== 1. 正常绿：退出码 0 + 汇总行 ===")
p, f, note = gw.parse_suite_result(0, summary(21, 0))
check("passed 取到 21", p == 21, f"got {p}")
check("failed 为 0", f == 0, f"got {f}")
check("无备注", note == "", f"got {note!r}")

print("=== 2. 正常红：汇总行已有失败数 ===")
p, f, _ = gw.parse_suite_result(1, summary(18, 3))
check("failed 取到 3", f == 3, f"got {f}")
check("passed 取到 18", p == 18, f"got {p}")

print("=== 3. 尺子 fail-open 主案：先打印 N passed 再崩（退出码非零）===")
# 历史验收未覆盖的第三种红；旧实现会判 PASS。
p, f, note = gw.parse_suite_result(1, summary(21, 0))
check("非零退出码必须计 FAIL", f >= 1, f"failed={f}（旧实现为 0 ⇒ 假 PASS）")
check("备注点名退出码", "退出码" in note, f"note={note!r}")

print("=== 4. 套件崩溃、无汇总行（退出码非零）===")
p, f, note = gw.parse_suite_result(1, "Traceback (most recent call last):\n  ...\n")
check("计 FAIL", f >= 1, f"failed={f}")
check("备注点名无汇总行", "无 RESULT" in note, f"note={note!r}")

print("=== 5. 空套件：零断言 ⇒ 无效绿（vacuous green）===")
p, f, note = gw.parse_suite_result(0, summary(0, 0))
check("汇总行 0/0 计 FAIL", f >= 1, f"failed={f}")
p, f, note = gw.parse_suite_result(0, "")
check("无输出且退出码 0 也计 FAIL", f >= 1, f"failed={f}")

print("=== 6. 汇总行锚定：'RESULT:' 之前出现的 'N passed' 不得被当总数 ===")
noisy = "  [PASS] case\n3 passed in 0.1s\n" + summary(21, 0)
p, f, _ = gw.parse_suite_result(0, noisy)
check("取 RESULT 行的 21（而非前面的 3）", p == 21, f"got {p}")
check("failed 仍为 0", f == 0, f"got {f}")

print(f"\n=== RESULT: {passed} passed, {failed} failed ===")
sys.exit(1 if failed else 0)
