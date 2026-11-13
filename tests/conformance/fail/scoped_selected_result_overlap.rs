fn select<'a>(left: &'a mut i64, right: &'a mut i64, yes: bool) -> &'a mut i64 {
    if yes { left } else { right }
}
fn test<'a>(value: &'a mut i64, yes: bool) {
    let current: &'a mut i64 = &mut *value;
    let selected = select(current, &mut *value, yes);
    *selected = 42;
}
pub fn probe() -> i64 {
    42
}
