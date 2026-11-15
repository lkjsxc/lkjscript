// expect: reject E_BORROW
// pending: callable impl Trait results are not yet elaborated; escaping a borrowed capture must be rejected
// witness: {"rustAccept":false}
fn escaped() -> impl Fn() -> i64 {
    let value: i64 = 42;
    || value
}
pub fn probe() -> i64 { escaped()() }
