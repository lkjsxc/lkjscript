enum Value { Item }
use Value::*;
fn wrong(_value: Item) -> i64 { 42 }
pub fn probe() -> i64 { wrong(Value::Item) }
