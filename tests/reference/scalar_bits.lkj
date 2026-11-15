// expect: value 42
pub fn probe() -> i64 {
    assert!(255u8.wrapping_add(1u8) == 0u8);
    assert!(255u8.checked_add(1u8).is_none());
    assert!(char::from_u32(0xd800u32).is_none());
    assert!(char::from_u32(0x10ffffu32).is_some());
    let bits = 0x7ff0000000000042u64;
    assert!(f64::from_bits(bits).to_bits() == bits);
    assert!((-f64::from_bits(bits)).to_bits() == (bits ^ 0x8000000000000000u64));
    assert!((-0.0f64).to_bits() == 0x8000000000000000u64);
    42
}
