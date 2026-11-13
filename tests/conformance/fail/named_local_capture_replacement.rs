fn test<'a>(input: &'a i64) -> i64 {
    let local = 42i64;
    let short = &local;
    let mut stored = input;
    let token = Box::new(0i64);
    let act = move || {
        drop(token);
        let first: &'a i64 = stored;
        let value = *first;
        drop(first);
        stored = short;
        let second: &'a i64 = stored;
        value + *second
    };
    act()
}
pub fn probe() -> i64 {
    42
}
