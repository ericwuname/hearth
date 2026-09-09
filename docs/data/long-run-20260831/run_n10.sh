#!/bin/bash
# P2-LR Node 10: Long-run Product Task B (geoutil, cross-file dependency topology)
cd /tmp/lr_n10
unset HEARTH_URL
export HEARTH_ALLOW_NO_CGROUP=1
export HEARTH_TASK_TIMEOUT_SECS=600
hearth chat '创建一个 Rust 库项目 geoutil/，包含两个模块，按以下顺序严格执行：
1) 用 bash 执行 mkdir -p geoutil/src
2) 用 write_file 创建 geoutil/Cargo.toml，内容为 [package] name=geoutil version=0.1.0 edition=2021
3) 用 write_file 创建 geoutil/src/point.rs，实现：
   pub struct Point { pub x: f64, pub y: f64 }
   pub fn distance(a: &Point, b: &Point) -> f64 { ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)) }（故意留 bug：缺少 .sqrt()）
4) 用 write_file 创建 geoutil/src/polygon.rs，实现：
   use crate::point::{Point, distance};
   pub fn perimeter(points: &[Point]) -> f64 { points.windows(2).map(|w| distance(&w[0], &w[1])).sum() }
   以及测试模块（在 polygon.rs）：#[cfg(test)] mod tests { use crate::point::Point; #[test] fn t_perim() { let ps = vec![Point{x:0.0,y:0.0}, Point{x:3.0,y:4.0}, Point{x:3.0,y:0.0}, Point{x:0.0,y:0.0}]; assert!((perimeter(&ps) - 12.0).abs() < 1e-9); } }
5) 用 write_file 创建 geoutil/src/lib.rs，内容为 pub mod point; pub mod polygon;
6) 用 bash 执行 cargo test --manifest-path geoutil/Cargo.toml —— perimeter 测试会失败（distance 缺 sqrt），记录失败输出
7) 用 write_file 修复 point.rs 的 distance（加上 .sqrt()）
8) 用 bash 执行 cargo test --manifest-path geoutil/Cargo.toml 确认通过
每步完成后立即进入下一步。' \
  --budget 80 \
  --acceptance 'cmd: cargo test --manifest-path geoutil/Cargo.toml' \
  --acceptance 'file: geoutil/src/point.rs contains sqrt' \
  --acceptance 'file: geoutil/src/polygon.rs contains perimeter'
echo "N10_EXIT=$?"
