// A scalar write through a checked alternative result changes no descriptor storage.
fn select<'a, F: FnOnce(&'a mut i64)>(
    callback: F,
    left: &'a mut i64,
    right: &'a mut i64,
    yes: bool,
) -> &'a mut i64 {
    drop(callback);
    if yes { left } else { right }
}
fn update<'a, F: FnOnce(&'a mut i64)>(
    callback: F,
    left: &'a mut i64,
    right: &'a mut i64,
    yes: bool,
) {
    let result = select(callback, left, right, yes);
    *result = 21;
}
pub fn probe() -> i64 {
    let mut left = 0i64;
    let mut right = 0i64;
    update(|_: &mut i64| {}, &mut left, &mut right, true);
    update(|_: &mut i64| {}, &mut left, &mut right, false);
    left + right
}
