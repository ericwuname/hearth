#!/bin/bash
# T08-rename-function 参考解：calc_sum → sum，calc_avg → average（含调用点与测试）。
set -euo pipefail
grep -q 'pub fn calc_sum' src/lib.rs || { echo "solution(T08): 未找到 calc_sum（fixture 形状已变）" >&2; exit 1; }

sed -i 's/calc_sum/sum/g; s/calc_avg/average/g' src/lib.rs

grep -q 'fn sum' src/lib.rs || { echo "solution(T08): 未见 fn sum" >&2; exit 1; }
grep -q 'fn average' src/lib.rs || { echo "solution(T08): 未见 fn average" >&2; exit 1; }
if grep -q 'calc_' src/lib.rs; then echo "solution(T08): 仍有 calc_* 残留" >&2; exit 1; fi
