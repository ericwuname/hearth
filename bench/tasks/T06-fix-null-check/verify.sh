#!/bin/bash
# T06-fix-null-check 判据（M0/D-222 统一强判据模板；判据库见 bench/verify_lib.sh）
# 纯行为级：fixture 自带的测试即验收契约。
set -euo pipefail
LIB="$(dirname "$0")/verify_lib.sh"
[ -f "$LIB" ] || { echo "VERIFY_FAIL: 缺少 verify_lib.sh（判据库）" >&2; exit 1; }
. "$LIB"
vh_require_cargo_test
vh_pass T06
