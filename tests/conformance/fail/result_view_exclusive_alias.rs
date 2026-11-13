pub fn probe() -> i64 {
    let mut value: Result<Box<i64>, Box<i64>> = Err(Box::new(20));
    let first = value.as_mut();
    let second = value.as_mut();
    **first.unwrap_err() + **second.unwrap_err()
}
