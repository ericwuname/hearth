#!/bin/bash
cd "$(dirname "$0")"
grep -q validate_length src/lib.rs || exit 1
source ~/.cargo/env 2>/dev/null
cargo test 2>&1 | grep -q "test result: ok" && echo VERIFY_PASS || exit 1
