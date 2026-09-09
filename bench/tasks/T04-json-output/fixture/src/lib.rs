pub struct User { pub name: String, pub age: u8 }
pub fn format_user(user: &User) -> String { format!("{} ({})", user.name, user.age) }
pub fn greet(users: &[User]) -> Vec<String> { users.iter().map(format_user).collect() }
#[cfg(test)] mod tests { use super::*;
  #[test] fn t_greet() { let u = User{name:"Ali".into(),age:25}; assert_eq!(greet(&[u])[0], "Ali (25)"); }
}