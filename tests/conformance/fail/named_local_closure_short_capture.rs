fn test<'a>() -> i64 {
    let value = 42i64;
    let read = || {
        let pointer: &'a i64 = &value;
        *pointer
    };
    read()
}
pub fn probe() -> i64 {
    42
}
