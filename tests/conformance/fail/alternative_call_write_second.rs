fn choose<'a>(first: &'a i64, second: &'a i64, left: bool) -> &'a i64 {
    if left { first } else { second }
}
pub fn probe() -> i64 {
    let first: i64 = 20;
    let mut second: i64 = 22;
    let selected = choose(&first, &second, true);
    second = 0;
    *selected
}
