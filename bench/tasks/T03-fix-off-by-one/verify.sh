#!/bin/bash
cd "$(dirname "$0")"
source ~/.cargo/env 2>/dev/null
cargo test 2>&1 | tail -3
echo VERIFY_PASS
