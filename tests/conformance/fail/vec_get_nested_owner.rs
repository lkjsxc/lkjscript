// expect: reject E_BORROW
// witness: {"rustAccept":false,"rustCode":"E0505"}
pub fn probe() -> i64 {
    let values = vec![Box::new(42i64)];
    let shared = &values;
    let nested = &shared;
    let item = nested.get(0usize).unwrap();
    drop(values);
    **item
}
