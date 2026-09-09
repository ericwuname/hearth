#!/bin/bash
cd "$(dirname "$0")"
grep -q "serde" Cargo.toml || { echo "NO_SERDE_DEP"; exit 1; }
grep -q "Serialize\|Deserialize" src/lib.rs || { echo "NO_DERIVE"; exit 1; }
source ~/.cargo/env 2>/dev/null
cargo test 2>&1 | grep -q "test result: ok" && echo VERIFY_PASS || { echo TEST_FAIL; exit 1; }
