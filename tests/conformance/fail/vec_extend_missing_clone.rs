struct Item { value: i64 }
pub fn probe() -> i64 { let source = [Item { value: 42 }]; let mut target: Vec<Item> = Vec::new(); target.extend_from_slice(&source); target[0].value }
