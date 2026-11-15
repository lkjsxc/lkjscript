// expect: reject E_BORROW
// witness: {"rustAccept":false}
fn duplicate<T: Clone>(value: &T) -> T { value.clone() }
fn leak<'a>(external: &'a i64) -> &'a i64 {
    let local = 7i64;
    let view = &local;
    duplicate(&view)
}
pub fn probe() -> i64 { let value = 42i64; *leak(&value) }
