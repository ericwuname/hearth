#!/bin/bash
# T01-read-api 判据（M0/D-222 统一强判据模板；判据库见 bench/verify_lib.sh）
# 读代码作答型：无代码改动，判据即产物内容。
set -euo pipefail
LIB="$(dirname "$0")/verify_lib.sh"
[ -f "$LIB" ] || { echo "VERIFY_FAIL: 缺少 verify_lib.sh（判据库）" >&2; exit 1; }
. "$LIB"
vh_assert_file ANSWER.md
vh_assert_md ANSWER.md 'sum_positive|factorial|is_palindrome|max_element'
vh_pass T01
