// expect: reject E_BORROW
// A descriptor in a still-live view aggregate retains its shared loan.
struct View<'a> {
    value: &'a i64,
}
pub fn probe() -> i64 {
    let mut values = vec![42i64];
    let reference = &values[0];
    let view = View { value: reference };
    let exclusive = &mut values[0];
    *exclusive
}
