// expect: value 42
pub fn probe() -> i64 {
    let values = vec![10i64, 11];
    let mut sum = 0i64;
    for value in values.iter().map(|value| *value * 2i64) {
        sum += value;
    }
    let mut calls = 0i64;
    let answer = Some(sum).unwrap_or_else(|| {
        calls += 1;
        0i64
    });
    assert!(calls == 0);
    let absent: Option<i64> = None;
    assert!(absent.is_none_or(|value| value > 0));
    answer
}
