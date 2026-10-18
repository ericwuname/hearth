#!/bin/bash
# T10-extract-config 参考解：硬编码提到 DbConfig{host,port,user} + `connect()`；测试改用 DbConfig::default()。
set -euo pipefail
grep -q 'pub fn db_connect' src/lib.rs || { echo "solution(T10): 未找到 db_connect（fixture 形状已变）" >&2; exit 1; }

cat > src/lib.rs <<'RUST'
#[derive(Debug, Clone)]
pub struct DbConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
}
impl Default for DbConfig {
    fn default() -> Self {
        DbConfig { host: "localhost".into(), port: 5432, user: "admin".into() }
    }
}
impl DbConfig {
    pub fn connect(&self) -> Result<String, String> {
        Ok(format!("postgres://{}:{}@{}:{}", self.user, "***", self.host, self.port))
    }
}
pub fn db_connect() -> Result<String, String> {
    DbConfig::default().connect()
}
#[cfg(test)] mod tests { use super::*;
  #[test] fn t_connect() { assert!(db_connect().unwrap().contains("localhost:5432")); }
  #[test] fn t_default_connect() { assert!(DbConfig::default().connect().unwrap().contains("localhost:5432")); }
}
RUST

grep -q 'struct DbConfig' src/lib.rs || { echo "solution(T10): 未写入 DbConfig" >&2; exit 1; }
grep -q 'fn connect' src/lib.rs || { echo "solution(T10): 未写入 connect" >&2; exit 1; }
