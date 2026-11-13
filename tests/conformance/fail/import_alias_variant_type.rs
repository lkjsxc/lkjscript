enum Value { Item }
type Alias = Value;
fn wrong(_value: Alias::Item) -> i64 { 42 }
pub fn probe() -> i64 { wrong(Alias::Item) }
