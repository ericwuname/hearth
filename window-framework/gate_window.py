#!/usr/bin/env python3
"""window-framework 常驻回归门禁（守门员建议 3：test_v10.py 等接入统一入口）。

用法：
    python3 gate_window.py          # 全量 9 套件，任一失败 exit 1
    python3 gate_window.py -v       # 显示每个套件明细

套件：framework + v02 + v03 + v04 + v05 + v06 + v07 + v09 + v10
（v10 含 P0-5/P0-6/P1-5 与 dogfooding 修复回归；缺失套件计 FAIL 不跳过——
    防止「文档写 187/187 但按命令复现不出来」的 CT8 类失真复现。）
"""
import re
import subprocess
import sys
import time

SUITES = ["framework", "v02", "v03", "v04", "v05", "v06", "v07", "v09", "v10"]
PY = sys.executable


def run_suite(name: str, verbose: bool) -> tuple[int, int, str]:
    """返回 (passed, failed, 尾部输出)。"""
    proc = subprocess.run(
        [PY, f"tests/test_{name}.py"],
        capture_output=True,
        text=True,
        timeout=600,
    )
    out = (proc.stdout or "") + (proc.stderr or "")
    # 套件输出形如 `=== RESULT: 21 passed, 0 failed ===` —— 正则一次性取数
    m_p = re.search(r"(\d+)\s+passed", out)
    m_f = re.search(r"(\d+)\s+failed", out)
    passed = int(m_p.group(1)) if m_p else 0
    failed = int(m_f.group(1)) if m_f else 0
    if proc.returncode != 0 and passed == 0 and failed == 0:
        # 套件自身崩溃（无 pytest 汇总行）——按 FAIL 处理
        failed = 1
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
