#!/bin/bash
# T19-merge-duplicate 判据（M0/D-222 统一强判据模板；判据库见 bench/verify_lib.sh）
# goal.txt 明确声明：验收脚本用 grep 精确检查标识符 parse_positive（名字错即判失败），
# 故此处保留该源码判据；并叠加行为判据（原有测试必须继续通过）。
set -euo pipefail
LIB="$(dirname "$0")/verify_lib.sh"
[ -f "$LIB" ] || { echo "VERIFY_FAIL: 缺少 verify_lib.sh（判据库）" >&2; exit 1; }
. "$LIB"
vh_assert_src src/lib.rs 'parse_positive'
vh_require_cargo_test
vh_pass T19
