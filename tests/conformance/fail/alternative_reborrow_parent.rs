pub fn probe() -> i64 {
    let mut first: i64 = 20;
    let mut second: i64 = 22;
    let a = &mut first;
    let b = &mut second;
    let selected = if true { &*a } else { &*b };
    *b = 0;
    *selected
}
