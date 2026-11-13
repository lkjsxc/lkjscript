fn apply<A, B, F: FnOnce(A) -> B>(function: F, value: A) -> B {
    function(value)
}
pub fn probe() -> i64 {
    let mut left = 20i64;
    let mut right = 22i64;
    let result: (&mut i64, &mut i64) = apply(
        |pair: (&mut i64, &mut i64)| (pair.1, pair.0),
        (&mut left, &mut right),
    );
    *result.0 = *result.0 + 1;
    *result.1 = *result.1 - 1;
    *result.0 + *result.1
}
