fn apply<A, B, F: FnOnce(A) -> B>(f: F, value: A) -> B {
    f(value)
}
pub fn probe() -> i64 {
    let value = 0i64;
    let result: &i64;
    {
        let short = 42i64;
        let captured = &short;
        result = apply(move |_input: &i64| captured, &value);
    }
    *result
}
