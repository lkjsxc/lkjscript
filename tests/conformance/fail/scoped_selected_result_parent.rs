fn select<'a>(left: &'a mut i64, right: &'a mut i64, yes: bool) -> &'a mut i64 {
    if yes { left } else { right }
}
fn test<'a>(left: &'a mut i64, right: &'a mut i64, yes: bool) {
    let current: &'a mut i64 = &mut *left;
    let selected = select(current, right, yes);
    *selected = 21;
    drop(selected);
    *left = 42;
}
pub fn probe() -> i64 {
    42
}
