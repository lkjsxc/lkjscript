fn test<'a, 'b: 'a>(input: &'b i64) -> i64 {
    let mut pointer = input;
    {
        let short: &mut &'a i64 = &mut pointer;
        drop(short);
    }
    let long: &'b i64 = pointer;
    *long
}
pub fn probe() -> i64 {
    let input = 42i64;
    test(&input)
}
