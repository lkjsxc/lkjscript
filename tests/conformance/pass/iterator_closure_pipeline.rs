pub fn probe() -> i64 {
    let values = vec![1i64, 2, 3, 4];
    let result = values
        .iter()
        .map(|value| *value * 2)
        .filter(|value| *value > 2)
        .fold(24i64, |sum, value| sum + value);
    assert_eq!(result, 42);
    let mut observed = 0i64;
    let selected = values
        .iter()
        .filter_map(|value| if *value > 2 { Some(*value) } else { None })
        .inspect(|value| {
            observed = observed + *value;
        })
        .fold(35i64, |sum, value| sum + value);
    assert_eq!(observed, 7);
    assert_eq!(selected, 42);
    let mut sum = 32i64;
    values.iter().for_each(|value| {
        sum = sum + *value;
    });
    sum
}
