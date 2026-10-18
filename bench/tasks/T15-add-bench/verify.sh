#!/bin/bash
# T15-add-bench 判据（M0/D-222 统一强判据模板；判据库见 bench/verify_lib.sh）
set -euo pipefail
LIB="$(dirname "$0")/verify_lib.sh"
[ -f "$LIB" ] || { echo "VERIFY_FAIL: 缺少 verify_lib.sh（判据库）" >&2; exit 1; }
. "$LIB"
vh_assert_file benches/bench.rs
vh_assert_src benches/bench.rs 'sum_range'
vh_assert_src benches/bench.rs 'sum_formula'
vh_assert_toml Cargo.toml '\[\[bench\]\]'
vh_require_cargo_test
vh_pass T15
