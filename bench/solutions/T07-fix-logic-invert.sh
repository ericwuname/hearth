#!/bin/bash
# T07-fix-logic-invert 参考解：age > 0 → age >= 0（判据要求出现 `>= 0`）。
set -euo pipefail
grep -q 'age > 0 && age <= 120' src/lib.rs || { echo "solution(T07): 未找到 age > 0（fixture 形状已变）" >&2; exit 1; }

sed -i 's/age > 0 && age <= 120/age >= 0 \&\& age <= 120/' src/lib.rs

grep -q 'age >= 0' src/lib.rs || { echo "solution(T07): 条件未改到 >= 0" >&2; exit 1; }
