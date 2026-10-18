#!/bin/bash
# T14-add-serde 参考解：Cargo.toml 补 serde 依赖（serde_json 已预置为 dev-dep）+ derive + 往返测试。
set -euo pipefail
grep -q 'pub struct Config' src/lib.rs || { echo "solution(T14): 未找到 Config（fixture 形状已变）" >&2; exit 1; }

# Cargo.toml：补 [dependencies].serde（幂等：已有则跳过）
if ! grep -q '^serde = ' Cargo.toml; then
  cat >> Cargo.toml <<'TOML'

[dependencies]
serde = { version = "1", features = ["derive"] }
TOML
fi

cat > src/lib.rs <<'RUST'
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Config {
    pub name: String,
    pub version: u32,
    pub features: Vec<String>,
}
pub fn default_config() -> Config {
    Config { name: "app".into(), version: 1, features: vec!["a".into()] }
}
#[cfg(test)] mod tests { use super::*;
  #[test] fn t_default() { assert_eq!(default_config().name, "app"); }
  #[test] fn t_roundtrip() {
    let c = default_config();
    let s = serde_json::to_string(&c).unwrap();
    let back: Config = serde_json::from_str(&s).unwrap();
    assert_eq!(back.name, c.name);
  }
}
RUST

grep -q 'Serialize' src/lib.rs || { echo "solution(T14): 未写入 Serialize" >&2; exit 1; }
grep -q '^serde = ' Cargo.toml || { echo "solution(T14): Cargo.toml 未补 serde" >&2; exit 1; }
