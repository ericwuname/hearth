#!/bin/bash
# T01-read-api 参考解：把四个公开函数写进 ANSWER.md。
set -euo pipefail
grep -q 'pub fn sum_positive' src/lib.rs || { echo "solution(T01): 未找到 sum_positive（fixture 形状已变）" >&2; exit 1; }

cat > ANSWER.md <<'MD'
sum_positive: 求整数切片中所有正数之和
factorial: 求 n 的阶乘（返回 u64）
is_palindrome: 判断字符串正反读是否相同
max_element: 返回切片中的最大元素（空切片为 None）
MD

grep -q 'sum_positive' ANSWER.md || { echo "solution(T01): ANSWER.md 未写入" >&2; exit 1; }
