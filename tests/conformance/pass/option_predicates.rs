pub fn probe() -> i64 {
    let mut calls: i64 = 0;
    let none: Option<i64> = None;
    assert!(!none.is_some_and(|value| { calls = calls + 1; value == 42 }));
    let none: Option<i64> = None;
    assert!(none.is_none_or(|value| { calls = calls + 1; value == 42 }));
    assert!(Some(Box::new(40i64)).is_some_and(|value| *value == 40));
    assert!(!Some(41i64).is_none_or(|value| value == 42));
    let mut value = 40i64;
    assert!(Some(&mut value).is_some_and(|slot| { *slot = *slot + 2; true }));
    value + calls
}
