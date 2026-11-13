pub fn probe() -> i64 {
    let first = Some(Some(20i64)).flatten().expect("value");
    let input: Option<Result<i64, i64>> = Some(Ok(22i64));
    let transposed: Result<Option<i64>, i64> = input.transpose();
    let second = transposed.unwrap().unwrap();
    let zipped = Some(first).zip(Some(second));
    let (left, right) = zipped.unzip();
    let selected = Some(0i64).and(left).or(None).xor(None).unwrap();
    let start: Result<i64, i64> = Ok(1);
    let next: Result<i64, i64> = Ok(right.unwrap());
    let chosen = start.and(next);
    let fallback: Result<i64, i64> = Ok(0);
    let optional: Result<Option<i64>, i64> = Ok(Some(42));
    assert!(optional.transpose().unwrap().unwrap() == 42);
    let no_value: Option<Option<i64>> = Some(None);
    assert!(no_value.flatten().is_none());
    let absent: Option<(Box<i64>, Box<i64>)> = None;
    let (a, b) = absent.unzip();
    assert!(a.is_none() && b.is_none());
    let absent_result: Option<Result<i64, i64>> = None;
    assert!(absent_result.transpose().unwrap().is_none());
    let failure: Result<Option<i64>, i64> = Err(20);
    assert!(failure.transpose().unwrap().unwrap_err() == 20);
    selected + chosen.or(fallback).unwrap()
}
