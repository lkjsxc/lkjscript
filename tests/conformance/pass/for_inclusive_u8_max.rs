pub fn probe() -> i64 {
    let mut sum: i64 = 0;
    for value in 253u8..=255u8 { sum = sum + value as i64; }
    sum
}
