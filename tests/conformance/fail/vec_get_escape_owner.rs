// expect: reject E_BORROW
// witness: {"rustAccept":false,"rustCode":"E0515"}
pub fn escape<'a>() -> &'a Box<i64> {
    let values = vec![Box::new(42i64)];
    values.get(0usize).unwrap()
}
