#!/bin/bash
# T15-add-bench 参考解：新建 benches/bench.rs（普通 #[test]，不用 criterion / 不写 fn main）
# + Cargo.toml 追加 [[bench]] name = "bench"（不写 harness = false）；src/lib.rs 保持原样。
set -euo pipefail
grep -q 'pub fn sum_range' src/lib.rs || { echo "solution(T15): 未找到 sum_range（fixture 形状已变）" >&2; exit 1; }

mkdir -p benches
cat > benches/bench.rs <<'RUST'
use T15::{sum_formula, sum_range};

#[test]
fn bench_sum_range() {
    let mut acc = 0u64;
    for i in 1..=2000 {
        acc = acc.wrapping_add(sum_range(i));
    }
    assert!(acc > 0);
}

#[test]
fn bench_sum_formula() {
    let mut acc = 0u64;
    for i in 1..=2000 {
        acc = acc.wrapping_add(sum_formula(i));
    }
    assert!(acc > 0);
}
RUST

if ! grep -q '^\[\[bench\]\]' Cargo.toml; then
  cat >> Cargo.toml <<'TOML'

[[bench]]
name = "bench"
TOML
fi

test -f benches/bench.rs || { echo "solution(T15): 未生成 benches/bench.rs" >&2; exit 1; }
grep -q '^\[\[bench\]\]' Cargo.toml || { echo "solution(T15): Cargo.toml 未注册 [[bench]]" >&2; exit 1; }
