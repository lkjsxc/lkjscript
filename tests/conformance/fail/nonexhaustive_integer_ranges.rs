pub fn classify(value: u8) -> i64 {
    match value {
        0..=127 => 1,
        129.. => 2,
    }
}
