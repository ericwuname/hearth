#!/bin/bash
# T17-fix-race 判据（M0/D-222 统一强判据模板；判据库见 bench/verify_lib.sh）
# 纯行为级：fixture 自带的并发测试即验收契约。
# 注意（已知局限，登记于任务书 M1/M0 开放项）：竞态类缺陷的测试可能**偶发通过**，
# 单次 cargo test 绿不等于无竞态——本判据只保证「不 fail-open」，不保证「无漏判」。
set -euo pipefail
LIB="$(dirname "$0")/verify_lib.sh"
[ -f "$LIB" ] || { echo "VERIFY_FAIL: 缺少 verify_lib.sh（判据库）" >&2; exit 1; }
. "$LIB"
vh_require_cargo_test
vh_pass T17
