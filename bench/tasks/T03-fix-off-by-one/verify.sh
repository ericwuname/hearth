#!/bin/bash
# T03-fix-off-by-one 判据（M0/D-222 统一强判据模板；判据库见 bench/verify_lib.sh）
# 修复前：`cargo test 2>&1 | tail -3; echo VERIFY_PASS` —— 无条件打印 VERIFY_PASS、
# 退出码恒 0，且 `tail -3` 把失败测试的输出藏掉 ⇒ 实测「2 failed」仍判通过。
# 本任务判据为**纯行为级**：fixture 自带 t_fib7/t_fib_bug 两个测试，修对才算过。
set -euo pipefail
LIB="$(dirname "$0")/verify_lib.sh"
[ -f "$LIB" ] || { echo "VERIFY_FAIL: 缺少 verify_lib.sh（判据库）" >&2; exit 1; }
. "$LIB"
vh_require_cargo_test
vh_pass T03
