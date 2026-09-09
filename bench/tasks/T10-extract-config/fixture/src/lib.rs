pub fn db_connect() -> Result<String, String> {
    let host = "localhost"; // hardcoded
    let port = 5432;        // hardcoded
    let user = "admin";      // hardcoded
    Ok(format!("postgres://{}:{}@{}:{}", user, "***", host, port))
}
#[cfg(test)] mod tests { use super::*;
  #[test] fn t_connect() { assert!(db_connect().unwrap().contains("localhost:5432")); }
}