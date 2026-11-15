// expect: reject E_BORROW
// witness: {"rustAccept":false}
pub fn probe() -> i64 {
    let read = { let local: i64 = 42; || local };
    read()
}
