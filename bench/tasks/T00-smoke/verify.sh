#!/bin/bash
# T00-smoke 判据（M0/D-222 统一强判据模板；判据库见 bench/verify_lib.sh）
set -euo pipefail
LIB="$(dirname "$0")/verify_lib.sh"
[ -f "$LIB" ] || { echo "VERIFY_FAIL: 缺少 verify_lib.sh（判据库）" >&2; exit 1; }
. "$LIB"
vh_require_cargo_test
vh_assert_src src/lib.rs 'fn multiply'
vh_assert_src src/lib.rs 'test_multiply'
vh_pass T00
