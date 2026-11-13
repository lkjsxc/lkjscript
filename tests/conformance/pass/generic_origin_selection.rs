struct Capture<'a, 'b> { left: &'a i64, right: &'b i64 }
fn choose<T>(first: T, second: T, left: bool) -> T {
    if left { first } else { second }
}
fn reference<'a>(first: &'a i64, second: &'a i64, left: bool) -> &'a i64 {
    if left { first } else { second }
}
fn selected(left: bool) -> i64 {
    let a: i64 = 10;
    let b: i64 = 20;
    let c: i64 = 5;
    let d: i64 = 7;
    let captured = choose(Capture { left: &a, right: &b }, Capture { left: &c, right: &d }, left);
    *captured.left + *captured.right
}
fn exclusive(left: bool) -> i64 {
    let mut a: i64 = 10;
    let mut b: i64 = 20;
    let picked = choose(&mut a, &mut b, left);
    *picked = *picked + 12;
    drop(picked);
    a + b
}
pub fn probe() -> i64 {
    let a: i64 = 20;
    let b: i64 = 22;
    selected(true) + selected(false) + exclusive(true) + exclusive(false)
        + *reference(&a, &b, true) + *reference(&a, &b, false) - 126
}
