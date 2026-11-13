fn test<'a>(input: &'a i64) -> i64 {
    let mut target = Box::new(0i64);
    let mut other = Box::new(1i64);
    let mut pointer = &mut target;
    let mut visits = 0i64;
    *{
        visits += 1;
        let read = || {
            let fixed: &'a i64 = input;
            *fixed
        };
        assert_eq!(read(), 42);
        std::mem::replace(&mut pointer, &mut other)
    } = Box::new(42i64);
    drop(pointer);
    assert!(visits == 1);
    *target
}
pub fn probe() -> i64 {
    let value = 42i64;
    test(&value)
}
