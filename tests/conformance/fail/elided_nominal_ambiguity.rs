struct Pair<'a, 'b> {
    left: &'a i64,
    right: &'b i64,
}
fn test<'a, 'b>(input: Pair<'a, 'b>) -> &i64 {
    input.left
}
pub fn probe() -> i64 {
    let value = 42i64;
    *test(Pair {
        left: &value,
        right: &value,
    })
}
