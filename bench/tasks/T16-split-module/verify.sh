#!/bin/bash
cd "$(dirname "$0")"
test -f src/math.rs || { echo "NO_MATH_RS"; exit 1; }
test -f src/string_utils.rs || { echo "NO_STRING_UTILS_RS"; exit 1; }
source ~/.cargo/env 2>/dev/null
cargo test 2>&1 | grep -q "test result: ok" && echo VERIFY_PASS || { echo TEST_FAIL; exit 1; }
