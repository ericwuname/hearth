#!/bin/bash
cd "$(dirname "$0")"
test -f benches/bench.rs || { echo "NO_BENCH_FILE"; exit 1; }
grep -q "sum_range" benches/bench.rs || { echo "NO_BENCH_SUM_RANGE"; exit 1; }
grep -q "sum_formula" benches/bench.rs || { echo "NO_BENCH_SUM_FORMULA"; exit 1; }
grep -q "\[\[bench\]\]" Cargo.toml || { echo "NO_BENCH_SECTION"; exit 1; }
source ~/.cargo/env 2>/dev/null
cargo test 2>&1 | grep -q "test result: ok" && echo VERIFY_PASS || { echo "TEST_FAIL"; exit 1; }
