fn classify(value: &u8) -> i64 {
    match value {
        ..=31 => 1,
        32..127 => 2,
        127.. => 3,
    }
}
fn upper(value: u64) -> i64 {
    match value {
        0..=9223372036854775807u64 => 4,
        9223372036854775808u64.. => 5,
    }
}
pub fn probe() -> i64 {
    let mut index: usize = 0;
    let mut total: i64 = 0;
    while index < 256 {
        let byte = index as u8;
        total = total + classify(&byte);
        index = index + 1;
    }
    assert_eq!(total, 609);
    assert_eq!(upper(18446744073709551615u64), 5);
    42
}
