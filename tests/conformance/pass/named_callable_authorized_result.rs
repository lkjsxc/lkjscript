fn helper<'a, F: FnOnce(&'a mut i64)>(f: F, value: &'a mut i64) -> &'a mut i64 {
    drop(f);
    value
}
fn invoke<'a, F: FnOnce(&'a mut i64)>(f: F, value: &'a mut i64, choose: bool) -> i64 {
    let result = if choose { helper(f, value) } else { value };
    *result = *result + 1;
    *result
}
pub fn probe() -> i64 {
    let mut left = 20i64;
    let mut right = 20i64;
    invoke(|_: &mut i64| {}, &mut left, true) + invoke(|_: &mut i64| {}, &mut right, false)
}
