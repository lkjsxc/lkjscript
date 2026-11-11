fn replace(slot: &mut Box<i64>) { *slot = Box::new(42); }
pub fn probe() -> i64 {
    let mut values = vec![Box::new(7i64)];
    values[0] = Box::new(9);
    replace(&mut values[0]);
    *values[0]
}
