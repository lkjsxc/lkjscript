// expect: value 42
fn mark(log: &mut i64, digit: i64) -> i64 {
    *log = *log * 10 + digit;
    digit
}
pub fn probe() -> i64 {
    let mut log = 0i64;
    let mut values = [0i64; 2];
    values[mark(&mut log, 1) as usize] = mark(&mut log, 2);
    assert!(log == 21);
    log = 0;
    values[mark(&mut log, 1) as usize] += mark(&mut log, 3);
    assert!(log == 31 && values[1] == 5);
    let answer = (mark(&mut log, 4), mark(&mut log, 2));
    assert!(log == 3142);
    answer.0 * 10 + answer.1
}
