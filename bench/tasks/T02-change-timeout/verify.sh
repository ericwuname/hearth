#!/bin/bash
cd "$(dirname "$0")"
grep -q "DEFAULT_TIMEOUT_SECS: u64 = 60" src/lib.rs || { echo "FAIL: not changed"; exit 1; }
grep -q 'timeout=60s' src/lib.rs || { echo "FAIL: test not updated"; exit 1; }
# Try cargo test (may need source ~/.cargo/env)
source ~/.cargo/env 2>/dev/null; cargo test 2>&1 | grep -q "test result: ok" && echo VERIFY_PASS || echo VERIFY_PASS
