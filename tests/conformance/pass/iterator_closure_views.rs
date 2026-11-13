pub fn probe() -> i64 {
    let values = [42i64, 0];
    let mut input = values.as_slice().iter().map(|value| value);
    let first = input.next().unwrap();
    drop(input);
    assert_eq!(*first, 42);
    let left = 20i64;
    let right = 22i64;
    let mut captured = values.as_slice().iter().map(|_| (&left, &right));
    let pair = captured.next().unwrap();
    drop(captured);
    assert_eq!(*pair.0 + *pair.1, 42);
    let mut filtered = values.as_slice().iter().filter(|value| **value > 0);
    let chosen = filtered.next().unwrap();
    drop(filtered);
    assert_eq!(*chosen, 42);
    let mut mapped = values
        .as_slice()
        .iter()
        .filter_map(|value| if *value > 0 { Some(value) } else { None });
    let answer = mapped.next().unwrap();
    drop(mapped);
    *answer
}
