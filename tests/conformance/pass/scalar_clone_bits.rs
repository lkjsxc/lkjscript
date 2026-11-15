// expect: value 42
// witness: {}
fn duplicate<T: Clone>(value: &T) -> T { value.clone() }
fn preserved<T: Clone + PartialEq>(source: T) {
    let mut target = source.clone();
    assert!(target == source && duplicate(&source) == source);
    target.clone_from(&source);
    assert!(target == source);
}
pub fn probe() -> i64 {
    preserved(());
    preserved(false);
    preserved(true);
    preserved('🦀');
    preserved(-128i8);
    preserved(-32768i16);
    preserved(-2147483648i32);
    preserved(-9223372036854775808i64);
    preserved(-9223372036854775808isize);
    preserved(255u8);
    preserved(65535u16);
    preserved(4294967295u32);
    preserved(18446744073709551615u64);
    preserved(!0usize);
    assert!(255u8.clone() == 255u8 && '水'.clone() == '水');
    let representations = [
        0u64,
        9223372036854775808u64,
        9218868437227405312u64,
        18442240474082181120u64,
        9221120237041090561u64,
        18444492273895866369u64,
        9218868437227405313u64,
    ];
    for bits in &representations[..] {
        let value = f64::from_bits(*bits);
        assert!(value.clone().to_bits() == *bits);
        assert!(duplicate(&value).to_bits() == *bits);
        let mut destination = 1.5f64;
        destination.clone_from(&value);
        assert!(destination.to_bits() == *bits);
    }
    42
}
