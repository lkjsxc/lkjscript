pub fn probe() -> i64 {
    let values = vec![
        0u64,
        9223372036854775808u64,
        9221120237041090561u64,
        18444492273895866369u64,
        9218868437227405313u64,
    ];
    for bits in values {
        let value = f64::from_bits(bits);
        assert_eq!(value.to_bits(), bits);
        assert_eq!((-value).to_bits(), bits ^ (1u64 << 63));
    }
    42
}
