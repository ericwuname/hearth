pub fn list_items(page: usize) -> Vec<String> {
    let all = (1..=100).map(|i| format!("item-{}", i)).collect::<Vec<_>>();
    let start = (page - 1) * 10;
    let end = start + 10;
    all.get(start..end).unwrap_or(&[]).to_vec()
}
#[cfg(test)] mod tests { use super::*;
  #[test] fn t_page1() { assert_eq!(list_items(1).len(), 10); }
  #[test] fn t_page10() { assert_eq!(list_items(10).len(), 10); }
}