pub fn probe() -> i64 {
    let mut bytes: Vec<u8> = Vec::new();
    let mut n: usize = 0;
    while n < 100000 { bytes.push(42u8); n += 1; }
    bytes[99999] as i64
}
