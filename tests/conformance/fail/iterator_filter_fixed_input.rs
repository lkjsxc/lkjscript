fn selected<'a>(values: &'a [i64], input: &'a &'a i64) -> i64 {
    let mut iterator = values.iter().filter(|value: &'a &'a i64| *value == *input);
    *iterator.next().unwrap()
}
pub fn probe() -> i64 {
    let values = [42i64];
    let input = &values[0];
    selected(&values, &input)
}
