#!/bin/bash
# T02-change-timeout 判据（M0/D-222 统一强判据模板；判据库见 bench/verify_lib.sh）
# 修复前：末行 `cargo test … && echo VERIFY_PASS || echo VERIFY_PASS` —— 两个分支都打印
# VERIFY_PASS、且退出码恒 0 ⇒ 测试失败也判通过。
set -euo pipefail
LIB="$(dirname "$0")/verify_lib.sh"
[ -f "$LIB" ] || { echo "VERIFY_FAIL: 缺少 verify_lib.sh（判据库）" >&2; exit 1; }
. "$LIB"
vh_assert_src src/lib.rs 'DEFAULT_TIMEOUT_SECS: u64 = 60'
vh_assert_src src/lib.rs 'timeout=60s'
vh_require_cargo_test
vh_pass T02
