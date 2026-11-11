struct Item { value: i64 }

impl Clone for Item {
    fn clone(&self) -> Self { Item { value: self.value } }
}

impl PartialEq for Item {
    fn eq(&self, other: &Self) -> bool { self.value == other.value }
}

impl Eq for Item {}

pub fn probe() -> i64 {
    let first = Item { value: 42 };
    let second = first.clone();
    if first.eq(&second) { second.value } else { 0 }
}
