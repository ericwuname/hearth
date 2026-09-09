#!/bin/bash
cd "$(dirname "$0")"
grep -q "struct DbConfig" src/lib.rs || { echo "NO_STRUCT"; exit 1; }
grep -q "connect" src/lib.rs || { echo "NO_METHOD"; exit 1; }
source ~/.cargo/env 2>/dev/null
cargo test 2>&1 | grep -q "test result: ok" && echo VERIFY_PASS || { echo TEST_FAIL; exit 1; }
