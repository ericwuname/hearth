#!/bin/bash
# T12-change-default 参考解：PAGE_SIZE 10→20，并同步 paginate(25) 的期望值 3→2。
set -euo pipefail
grep -q 'PAGE_SIZE: usize = 10' src/lib.rs || { echo "solution(T12): 未找到 PAGE_SIZE = 10（fixture 形状已变）" >&2; exit 1; }

sed -i 's/PAGE_SIZE: usize = 10/PAGE_SIZE: usize = 20/' src/lib.rs
sed -i 's/assert_eq!(paginate(25), 3)/assert_eq!(paginate(25), 2)/' src/lib.rs

grep -q 'PAGE_SIZE: usize = 20' src/lib.rs || { echo "solution(T12): PAGE_SIZE 未改到 20" >&2; exit 1; }
grep -q 'paginate(25), 2' src/lib.rs || { echo "solution(T12): 测试期望未同步到 2" >&2; exit 1; }
