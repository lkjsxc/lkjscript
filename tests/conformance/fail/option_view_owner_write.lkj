// expect: reject E_BORROW
// witness: {"rustAccept":false,"rustCode":"E0506"}
pub fn probe() -> i64 {
    let mut value = Some(Box::new(20i64));
    let view = value.as_ref();
    value = Some(Box::new(22));
    **view.unwrap()
}
