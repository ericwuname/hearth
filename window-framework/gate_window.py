#!/usr/bin/env python3
"""window-framework 常驻回归门禁（守门员建议 3：test_v10.py 等接入统一入口）。

用法：
    python3 gate_window.py          # 全量 9 套件，任一失败 exit 1
    python3 gate_window.py -v       # 显示每个套件明细

套件：framework + v02 + v03 + v04 + v05 + v06 + v07 + v09 + v10 + gate_window
（v10 含 P0-5/P0-6/P1-5 与 dogfooding 修复回归；缺失套件计 FAIL 不跳过——
    防止「文档写 187/187 但按命令复现不出来」的 CT8 类失真复现。）
（gate_window 套件 = **尺子自检**：直接测本文件的结果判定函数 `parse_suite_result`
    ——尺子自己不 fail-open，才谈得上守门。）
"""
import re
import subprocess
import sys
import time

SUITES = ["framework", "v02", "v03", "v04", "v05", "v06", "v07", "v09", "v10",
          "gate_window"]
PY = sys.executable

# 套件末行的汇总格式（9 个套件统一）：`=== RESULT: {passed} passed, {failed} failed ===`
# **锚定到 RESULT 行**——旧实现用 `re.search(r"(\d+)\s+passed", out)` 取**第一处**
# 匹配，任何在汇总行之前出现的 "N passed" 字样（逐项打印、docstring 回显等）都会被
# 当成总数。
RESULT_RE = re.compile(r"RESULT:\s*(\d+)\s+passed,\s*(\d+)\s+failed")


def parse_suite_result(returncode: int, out: str) -> tuple[int, int, str]:
    """由「套件退出码 + 输出」判定 `(passed, failed, 备注)`。

    **退出码是权威信号**——每个套件末行都是 `sys.exit(1 if failed else 0)`，
    故 `returncode != 0` 一律意味着"这次没跑干净"。旧实现只在**解析不出任何数字**
    时才看退出码（`if rc != 0 and passed == 0 and failed == 0`），于是
    「先打印 `N passed, 0 failed` 再崩/再失败」会被判 **PASS**——尺子自己
    fail-open（与 D-123/D-127 的"防线失守"同族）。现改为：**非零退出码一律计
    FAIL**，即使汇总行里 `failed == 0`。
    """
    m = RESULT_RE.search(out)
    if m:
        passed, failed, note = int(m.group(1)), int(m.group(2)), ""
    else:
        # 套件自身崩溃 / 没跑到汇总行——按 FAIL 处理（不跳过，防 CT8 类失真）
        passed, failed, note = 0, 0, "无 RESULT 汇总行"
    if returncode != 0 and failed == 0:
        failed = 1
        note = f"{note}；" if note else ""
        note += f"退出码 {returncode}（非零即失败，不采信 failed=0）"
    if passed == 0 and failed == 0:
        # 零断言 = 无效绿（同 xray「空链视为断裂」口径）——套件什么都没断言也算失败，
        # 否则"把套件清空成 exit 0"就能把尺子骗绿。
        failed = 1
        note = f"{note}；" if note else ""
        note += "零断言（vacuous green）"
    return passed, failed, note


def run_suite(name: str, verbose: bool) -> tuple[int, int, str]:
    """返回 (passed, failed, 尾部输出)。"""
    proc = subprocess.run(
        [PY, f"tests/test_{name}.py"],
        capture_output=True,
        text=True,
        timeout=600,
    )
    out = (proc.stdout or "") + (proc.stderr or "")
    passed, failed, note = parse_suite_result(proc.returncode, out)
    if note:
        # 备注（退出码/零断言/无汇总行）随尾部输出一起呈现，失败时用户看得见
        out = f"[{note}]\n{out}"
    tail = out.strip().splitlines()
    tail = "\n".join(tail[-6:]) if tail else "(no output)"
    return passed, failed, tail


def main() -> int:
    verbose = "-v" in sys.argv
    total_p = total_f = 0
    results = []
    print(f"window-framework 常驻回归门禁 @ {time.strftime('%Y-%m-%d %H:%M:%S')}")
    print(f"python: {PY}\n")
    for name in SUITES:
        t0 = time.time()
        try:
            passed, failed, tail = run_suite(name, verbose)
        except FileNotFoundError:
            print(f"  [FAIL] {name}: 测试文件不存在 tests/test_{name}.py")
            failed, passed = 1, 0
            tail = "missing test file"
        except subprocess.TimeoutExpired:
            print(f"  [FAIL] {name}: 超时 600s")
            failed, passed = 1, 0
            tail = "timeout"
        total_p += passed
        total_f += failed
        mark = "PASS" if failed == 0 else "FAIL"
        print(f"  [{mark:4}] {name}: {passed} passed / {failed} failed ({time.time()-t0:.1f}s)")
        results.append((name, mark, tail))
    print(f"\n  TOTAL: {total_p} passed / {total_f} failed")
    if total_f:
        print("\n--- 失败套件尾部输出 ---")
        for name, mark, tail in results:
            if mark == "FAIL":
                print(f"### {name}\n{tail}\n")
    ok = total_f == 0
    print("\nGATE_RESULT:", "PASS" if ok else "FAIL")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
