pub fn probe() -> i64 {
    let mut empty: Vec<i64> = Vec::new();
    empty.sort();
    empty.retain(|_| false);
    assert!(empty.is_empty());
    let mut values: Vec<i64> = Vec::new();
    let mut index = 0i64;
    let mut expected_sum = 0i64;
    while index < 257 {
        let value = (index * 73) % 257;
        expected_sum += value;
        values.push(value);
        index += 1;
    }
    values.sort();
    let mut index = 0usize;
    while index < values.len() {
        assert_eq!(values[index], index as i64);
        index += 1usize;
    }
    let mut observed = 0i64;
    for value in values {
        observed += value;
    }
    assert_eq!(observed, expected_sum);
    let mut values = vec![
        Box::new(1i64),
        Box::new(2i64),
        Box::new(3i64),
        Box::new(4i64),
    ];
    let mut visits = 0i64;
    values.retain(|value| {
        visits = visits * 10 + **value;
        **value % 2 == 1
    });
    assert_eq!(visits, 1234);
    let mut kept = 0i64;
    for value in values {
        kept = kept * 10 + *value;
    }
    assert_eq!(kept, 13);
    let mut duplicates = vec![3i64, 1, 3, 2, 1];
    duplicates.sort();
    assert_eq!(duplicates[0], 1);
    assert_eq!(duplicates[1], 1);
    assert_eq!(duplicates[2], 2);
    assert_eq!(duplicates[3], 3);
    assert_eq!(duplicates[4], 3);
    42
}
