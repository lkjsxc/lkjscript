type Inner<'x> = &'x i64;
fn test<'a, 'b>(input: &'a Inner<'b>) -> &i64 {
    *input
}
pub fn probe() -> i64 {
    let value = 42i64;
    *test(&&value)
}
