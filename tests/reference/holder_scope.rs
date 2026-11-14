// expect: value 42
// End the carrier's lexical scope before borrowing its owner exclusively.
struct View<'a> {
    value: &'a i64,
}
pub fn probe() -> i64 {
    let mut values = vec![41i64];
    {
        let reference = &values[0];
        let view = View { value: reference };
        assert!(*view.value == 41);
    }
    let exclusive = &mut values[0];
    *exclusive += 1;
    *exclusive
}
