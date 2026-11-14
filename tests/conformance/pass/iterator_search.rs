pub fn probe() -> i64 {
    let values: Vec<i64> = vec![3, 7, 11, 21];
    let mut items = values.iter();
    assert!(*items.find(|value| **value == 7).unwrap() == 7);
    assert!(items.position(|value| *value == 21).unwrap() == 1);
    assert!(items.next().is_none());
    let mut sum = 0;
    for value in Some(20i64) {
        sum += value;
    }
    let empty: Option<i64> = None;
    for value in empty {
        sum += value;
    }
    let mut remaining = Some(22i64).into_iter();
    sum += remaining.next().unwrap();
    assert!(remaining.next().is_none());
    sum
}
