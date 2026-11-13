pub fn probe() -> i64 {
    let result = Some(41i64).is_some_and(|value| value + 1);
    if result { 42 } else { 0 }
}
