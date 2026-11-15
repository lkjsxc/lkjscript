// expect: value 42
// authority-expect: declared none; reachable operations none
pub fn probe() -> i64 {
    let values = vec![Box::new(40i64), Box::new(2i64)];
    assert!(values.len() == 2);
    *values[0] + *values[1]
}
