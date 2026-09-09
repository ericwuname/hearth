pub struct Product {
    pub id: u64,
    pub name: String,
    pub price_cents: u32,
    pub in_stock: bool,
}
pub fn can_afford(products: &[Product], budget_cents: u32) -> Vec<&Product> {
    products.iter().filter(|p| p.price_cents <= budget_cents && p.in_stock).collect()
}
#[cfg(test)] mod tests { use super::*;
  #[test] fn t_filter() {
    let p = vec![Product{id:1,name:"A".into(),price_cents:100,in_stock:true}];
    assert_eq!(can_afford(&p, 200).len(), 1);
  }
}