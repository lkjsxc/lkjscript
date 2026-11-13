pub fn probe() -> i64 {
    let mut first: i64 = 20;
    let mut second: i64 = 22;
    let selected = if true { &first } else { &second };
    first = 0;
    *selected
}
