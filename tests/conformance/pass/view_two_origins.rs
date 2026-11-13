fn choose<'a>(first: &'a i64, second: &'a i64, left: bool) -> &'a i64 {
    if left { first } else { second }
}
pub fn probe() -> i64 {
    let first: i64 = 42;
    let second: i64 = 0;
    *choose(&first, &second, true)
}
