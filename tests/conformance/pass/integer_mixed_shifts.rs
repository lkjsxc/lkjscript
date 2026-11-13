pub fn probe() -> i64 {
    let count = 3u8;
    let left = 5_i64 << count;
    let right = 16_u16 >> 3usize;
    let signed = -128_i8 >> 7u64;
    let narrow = 1_i8 << 7u64;
    let borrowed = &8_u16 >> &1_u8;
    if signed != -1 || narrow != -128 || borrowed != 4 {
        return 0;
    }
    left + right as i64
}
