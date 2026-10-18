#!/bin/bash
# T02-change-timeout 参考解：DEFAULT_TIMEOUT_SECS 30→60，并同步测试期望值。
set -euo pipefail
grep -q 'DEFAULT_TIMEOUT_SECS: u64 = 30' src/lib.rs || { echo "solution(T02): 未找到 30（fixture 形状已变）" >&2; exit 1; }

sed -i 's/DEFAULT_TIMEOUT_SECS: u64 = 30/DEFAULT_TIMEOUT_SECS: u64 = 60/' src/lib.rs
sed -i 's/timeout=30s/timeout=60s/' src/lib.rs

grep -q 'DEFAULT_TIMEOUT_SECS: u64 = 60' src/lib.rs || { echo "solution(T02): 常量未改到 60" >&2; exit 1; }
grep -q 'timeout=60s' src/lib.rs || { echo "solution(T02): 测试期望未同步到 60" >&2; exit 1; }
