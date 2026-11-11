fn identity<T>(value: T) -> T { value }
pub fn probe() -> i64 { identity::<i64>(42) }
