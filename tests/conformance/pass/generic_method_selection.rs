struct Item { value: i64 }

impl Clone for Item {
    fn clone(&self) -> Self { Item { value: 1 } }
}

trait Score { fn score(&self) -> i64; }
impl Score for Item {
    fn score(&self) -> i64 { 41 }
}

impl Item {
    fn clone(&self) -> Self { Item { value: 2 } }
    fn score(&self) -> i64 { 100 }
}

fn copied<T: Clone>(value: &T) -> T { value.clone() }
fn scored<T: Score>(value: &T) -> i64 { value.score() }

pub fn probe() -> i64 {
    let value = Item { value: 0 };
    assert_eq!(value.clone().value, 2);
    assert_eq!(value.score(), 100);
    copied(&value).value + scored(&value)
}
