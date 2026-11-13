fn invoke<F: FnMut(&mut i64) -> i64>(mut f: F, value: &mut i64) -> i64 {
    let mut index = 0usize;
    while index < 2 {
        f(&mut *value);
        index = index + 1;
    }
    *value = *value + 2;
    *value
}
pub fn probe() -> i64 {
    let mut value = 38i64;
    invoke(
        |input: &mut i64| {
            *input = *input + 1;
            *input
        },
        &mut value,
    )
}
