pub fn probe() -> i64 {
    let mut value = Some(Box::new(20i64));
    assert!(**value.as_ref().unwrap() == 20);
    **value.as_mut().unwrap() = 21;
    let slot = value.get_or_insert(Box::new(0));
    **slot = **slot + 1;
    let old = value.replace(Box::new(22)).unwrap();
    assert!(*old == 22);
    let mut result: Result<Box<i64>, Box<i64>> = Err(Box::new(1));
    **result.as_mut().unwrap_err() = 20;
    assert!(**result.as_ref().unwrap_err() == 20);
    let mut inserted: Option<Box<i64>> = None;
    **inserted.insert(Box::new(1)) = 42;
    assert!(**inserted.as_ref().unwrap() == 42);
    let mut number = 20i64;
    let mut borrowed = Some(&mut number);
    **borrowed.as_mut().unwrap() = 22;
    drop(borrowed);
    number + **result.as_ref().unwrap_err()
}
