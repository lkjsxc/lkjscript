fn test<'a, 'b: 'a>(input: &'b i64) -> i64 {
    let pointer = input;
    let short: &&'a i64 = &pointer;
    let copied = short;
    drop(short);
    let long: &'b i64 = *copied;
    *long
}
pub fn probe() -> i64 {
    let input = 42i64;
    test(&input)
}
