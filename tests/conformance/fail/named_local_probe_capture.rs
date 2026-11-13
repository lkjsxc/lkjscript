fn test<'a>(input: &'a i64) -> i64 {
    let local = 42i64;
    let short = &local;
    let mut target = Box::new(0i64);
    let mut other = Box::new(1i64);
    let mut pointer = &mut target;
    *{
        let read = || {
            let fixed: &'a i64 = short;
            *fixed
        };
        assert_eq!(read(), 42);
        std::mem::replace(&mut pointer, &mut other)
    } = Box::new(42i64);
    drop(pointer);
    *target
}
pub fn probe() -> i64 {
    let value = 42i64;
    test(&value)
}
