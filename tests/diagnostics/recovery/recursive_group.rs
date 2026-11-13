fn even(value: i64) -> bool { if value == 0 { true } else { odd(value - 1) } }
fn odd(value: i64) -> bool { if value == 0 { false } else { even(value - 1) } }
fn unavailable() -> Missing { unavailable() }
fn independent(value: i64) -> i64 { value.unrelated() }
