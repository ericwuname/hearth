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
}