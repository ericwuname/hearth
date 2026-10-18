#!/bin/bash
# T11-read-struct 参考解：把 Product 的字段名与类型写进 ANSWER.md。
set -euo pipefail
grep -q 'pub struct Product' src/lib.rs || { echo "solution(T11): 未找到 Product（fixture 形状已变）" >&2; exit 1; }

cat > ANSWER.md <<'MD'
id: u64, name: String, price_cents: u32, in_stock: bool
MD

grep -q 'in_stock' ANSWER.md || { echo "solution(T11): ANSWER.md 未写入" >&2; exit 1; }
