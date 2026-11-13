fn apply<A, B, F: FnOnce(A) -> B>(function: F, value: A) -> B {
    function(value)
}
fn capture_selected() -> i64 {
    let value = 42i64;
    let ignored = 0i64;
    let captured = &value;
    let result: &i64 = apply(move |_: &i64| captured, &ignored);
    *result
}
fn argument_selected() -> i64 {
    let value = 42i64;
    let result: &i64;
    {
        let short = 1i64;
        let captured = &short;
        result = apply(
            move |input: &i64| {
                let _ = *captured;
                input
            },
            &value,
        );
    }
    *result
}
pub fn probe() -> i64 {
    assert_eq!(capture_selected(), 42);
    argument_selected()
}
