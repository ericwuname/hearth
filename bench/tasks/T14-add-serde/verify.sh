#!/bin/bash
# T14-add-serde 判据（M0/D-222 统一强判据模板；判据库见 bench/verify_lib.sh）
# 注：serde_json 已由 fixture 预置为 dev-dependency；此处只要求 serde 出现于 Cargo.toml
# （TOML 注释用 `#`，故走 vh_assert_toml 而非 vh_assert_src）。
set -euo pipefail
LIB="$(dirname "$0")/verify_lib.sh"
[ -f "$LIB" ] || { echo "VERIFY_FAIL: 缺少 verify_lib.sh（判据库）" >&2; exit 1; }
. "$LIB"
vh_assert_toml Cargo.toml 'serde'
vh_assert_src src/lib.rs 'Serialize|Deserialize'
vh_require_cargo_test
vh_pass T14
