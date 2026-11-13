fn test<'a, 'b: 'a>(input: &'b i64) -> i64 {
    let pointer = input;
    let short: &&'a i64 = &pointer;
    let read = move || -> &'b i64 { *short };
    *read()
}
pub fn probe() -> i64 {
    let input = 42i64;
    test(&input)
}
