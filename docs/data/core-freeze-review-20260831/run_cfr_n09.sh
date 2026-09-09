#!/bin/bash
# CFR Node 09: Long-run Product Benchmark ×2（不同任务）
# Task A: todoapi（多文件 JSON 存储库，含受控失败）
cd /tmp/cfr_n09
unset HEARTH_URL
export HEARTH_ALLOW_NO_CGROUP=1
export HEARTH_TASK_TIMEOUT_SECS=600
hearth chat '创建一个 Rust 库项目 todoapi/，包含两个模块，按以下顺序严格执行：
1) 用 bash 执行 mkdir -p todoapi/src
2) 用 write_file 创建 todoapi/Cargo.toml，内容为 [package] name=todoapi version=0.1.0 edition=2021
3) 用 write_file 创建 todoapi/src/model.rs，实现：
   #[derive(Debug, PartialEq)] pub struct Todo { pub id: u32, pub title: String }
   pub fn new(id: u32, title: &str) -> Todo { Todo { id, title: title.to_string() } }
4) 用 write_file 创建 todoapi/src/store.rs，实现：
   use crate::model::Todo;
   pub struct Store { pub items: Vec<Todo> }
   impl Store { pub fn add(&mut self, t: Todo) { self.items.push(t); } pub fn find(&self, id: u32) -> Option<&Todo> { self.items.iter().find(|t| t.id == id) } pub fn remove(&mut self, id: u32) -> bool { let before = self.items.len(); self.items.retain(|t| t.id != id); self.items.len() < before } }
   以及测试模块：#[cfg(test)] mod tests { use crate::model::Todo; use crate::store::Store; #[test] fn t_flow() { let mut s = Store { items: vec![] }; s.add(Todo::new(1, "a")); s.add(Todo::new(2, "b")); assert!(s.find(1).is_some()); assert!(s.remove(1)); assert!(s.find(1).is_none()); } }
5) 用 write_file 创建 todoapi/src/lib.rs，内容为 pub mod model; pub mod store;
6) 用 bash 执行 cargo test --manifest-path todoapi/Cargo.toml —— 若失败记录输出并修复至通过；若通过直接进入下一步
7) 用 bash 执行 seq 1 1000（制造历史体积）
8) 用 write_file 创建 todoapi/NOTES.md，内容为 "todoapi: minimal json-less todo store"
每步完成后立即进入下一步。' \
  --budget 80 \
  --acceptance 'cmd: cargo test --manifest-path todoapi/Cargo.toml' \
  --acceptance 'file: todoapi/NOTES.md contains todoapi'
echo "N09A_EXIT=$?"

# Task B: units（温度换算库，受控失败=常数错误拓扑）
cd /tmp/cfr_n09b
mkdir -p /tmp/cfr_n09b
cd /tmp/cfr_n09b
unset HEARTH_URL
export HEARTH_ALLOW_NO_CGROUP=1
export HEARTH_TASK_TIMEOUT_SECS=600
hearth chat '创建一个 Rust 库项目 units/，按以下顺序严格执行：
1) 用 bash 执行 mkdir -p units/src
2) 用 write_file 创建 units/Cargo.toml，内容为 [package] name=units version=0.1.0 edition=2021
3) 用 write_file 创建 units/src/lib.rs，实现：
   pub fn c_to_f(c: f64) -> f64 { c * 1.8 + 32.0 }
   pub fn f_to_c(f: f64) -> f64 { (f - 32.0) / 1.8 }
   测试模块：#[cfg(test)] mod tests { use super::*; #[test] fn t_c() { assert!((c_to_f(100.0) - 212.0).abs() < 1e-9); } #[test] fn t_f() { assert!((f_to_c(32.0) - 0.0).abs() < 1e-9); } #[test] fn t_round() { assert!((f_to_c(c_to_f(37.0)) - 37.0).abs() < 1e-9); } }
4) 用 bash 执行 cargo test --manifest-path units/Cargo.toml —— 应通过；若失败记录并修复
5) 用 bash 执行 seq 1 900 和 seq 1000 1900（制造历史体积）
6) 用 write_file 创建 units/CHANGELOG.md，内容为 "units: temperature conversions v0.1.0"
7) 用 bash 执行 cargo test --manifest-path units/Cargo.toml 最终确认
每步完成后立即进入下一步。' \
  --budget 80 \
  --acceptance 'cmd: cargo test --manifest-path units/Cargo.toml' \
  --acceptance 'file: units/CHANGELOG.md contains units'
echo "N09B_EXIT=$?"
