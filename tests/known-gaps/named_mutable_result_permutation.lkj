// Rust accepts; the current definition-site return contract conservatively maps both same-region mutable result leaves to both inputs and rejects E_BORROW. A checked symbolic per-leaf declaration equation is still needed.
fn helper<'a, F: FnOnce(&'a mut i64)>(
    f: F,
    a: &'a mut i64,
    b: &'a mut i64,
) -> (&'a mut i64, &'a mut i64) {
    drop(f);
    (b, a)
}
fn test<'a, F: FnOnce(&'a mut i64)>(f: F, a: &'a mut i64, b: &'a mut i64) {
    let (left, right) = helper(f, a, b);
    *left = 20;
    *right = 22;
}
pub fn probe() -> i64 {
    42
}
