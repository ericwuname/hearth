#!/bin/bash
# T00-smoke verify: check cargo build + test + grep multiply
set -e
cd "$(dirname "$0")"
cargo build --offline 2>&1 || cargo build 2>&1
cargo test 2>&1
grep -q "multiply" src/lib.rs && echo "VERIFY_PASS: multiply function found" || { echo "VERIFY_FAIL: multiply not in src/lib.rs"; exit 1; }
echo "== T00-smoke PASS =="
