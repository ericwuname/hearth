#!/bin/bash
# T18-add-pagination 参考解：新增 struct Pagination{page,per_page}，接口改为 list_items(p: Pagination)，
# 并同步更新两个测试。
set -euo pipefail
grep -q 'pub fn list_items(page: usize)' src/lib.rs || { echo "solution(T18): 未找到 list_items(page)（fixture 形状已变）" >&2; exit 1; }

cat > src/lib.rs <<'RUST'
pub struct Pagination { pub page: usize, pub per_page: usize }
pub fn list_items(p: Pagination) -> Vec<String> {
    let all = (1..=100).map(|i| format!("item-{}", i)).collect::<Vec<_>>();
    let start = (p.page - 1) * p.per_page;
    let end = start + p.per_page;
    all.get(start..end).unwrap_or(&[]).to_vec()
}
#[cfg(test)] mod tests { use super::*;
  #[test] fn t_page1() { assert_eq!(list_items(Pagination{page:1, per_page:10}).len(), 10); }
  #[test] fn t_page10() { assert_eq!(list_items(Pagination{page:10, per_page:10}).len(), 10); }
}
RUST

grep -q 'struct Pagination' src/lib.rs || { echo "solution(T18): 未写入 Pagination" >&2; exit 1; }
