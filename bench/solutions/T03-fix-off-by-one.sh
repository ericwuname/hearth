#!/bin/bash
# T03-fix-off-by-one 参考解：循环上界 1..n（n-1 次）→ 1..=n（n 次）。
set -euo pipefail
grep -q 'for _ in 1\.\.n {' src/lib.rs || { echo "solution(T03): 未找到 off-by-one 循环（fixture 形状已变）" >&2; exit 1; }

sed -i 's/for _ in 1\.\.n {/for _ in 1..=n {/' src/lib.rs

grep -q 'for _ in 1\.\.=n {' src/lib.rs || { echo "solution(T03): 循环上界未改到 1..=n" >&2; exit 1; }
