pub fn probe() -> i64 {
    let values = [(20i64, 22i64)];
    let mapped = values
        .as_slice()
        .iter()
        .map(|(left, right)| *left + *right)
        .fold(0i64, |sum, value| sum + value);
    assert_eq!(mapped, 42);
    let mut total = 0i64;
    for (left, right) in values.as_slice().iter() {
        total = total + *left + *right;
    }
    assert_eq!(total, 42);
    let numbers = [42i64];
    let mut iterator = numbers.as_slice().iter();
    let literal = match iterator.next().unwrap() {
        42 => 42,
        _ => 0,
    };
    assert_eq!(literal, 42);
    let mut iterator = numbers.as_slice().iter();
    match iterator.next().unwrap() {
        40..=42 => 42,
        _ => 0,
    }
}
