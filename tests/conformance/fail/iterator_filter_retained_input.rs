pub fn probe() -> i64 {
    let values = [42i64];
    let mut retained: Option<&&i64> = None;
    let mut iterator = values.as_slice().iter().filter(|value| {
        retained = Some(value);
        true
    });
    let result = *iterator.next().unwrap();
    drop(iterator);
    if let Some(value) = retained {
        **value
    } else {
        result
    }
}
