#!/bin/bash
# T04-json-output 参考解：手写 JSON 的 to_json（不引 serde）+ 一个断言测试。
set -euo pipefail
grep -q 'pub fn greet' src/lib.rs || { echo "solution(T04): 未找到 greet（fixture 形状已变）" >&2; exit 1; }

cat > src/lib.rs <<'RUST'
pub struct User { pub name: String, pub age: u8 }
pub fn format_user(user: &User) -> String { format!("{} ({})", user.name, user.age) }
pub fn greet(users: &[User]) -> Vec<String> { users.iter().map(format_user).collect() }
pub fn to_json(users: &[User]) -> String {
    let items: Vec<String> = users
        .iter()
        .map(|u| format!("{{\"name\":\"{}\",\"age\":{}}}", u.name, u.age))
        .collect();
    format!("[{}]", items.join(","))
}
#[cfg(test)] mod tests { use super::*;
  #[test] fn t_greet() { let u = User{name:"Ali".into(),age:25}; assert_eq!(greet(&[u])[0], "Ali (25)"); }
  #[test] fn t_json() {
    let u = User{name:"Ali".into(),age:25};
    assert_eq!(to_json(&[u]), "[{\"name\":\"Ali\",\"age\":25}]");
  }
}
RUST

grep -q 'fn to_json' src/lib.rs || { echo "solution(T04): 未写入 to_json" >&2; exit 1; }
