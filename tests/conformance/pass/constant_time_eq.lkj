// expect: value 42
use std::hmac::constant_time_eq;

pub fn probe() -> i64 {
    assert!(constant_time_eq(b"", b""));
    assert!(!constant_time_eq(b"", b"x"));
    assert!(!constant_time_eq(b"x", b""));
    assert!(constant_time_eq(b"same", b"same"));
    assert!(!constant_time_eq(b"same", b"sam"));
    assert!(!constant_time_eq(b"sam", b"same"));
    let binary = [0u8, 0xffu8, 0x80u8, 0x7fu8, 1u8, 0u8, 42u8, 0xfeu8];
    assert!(constant_time_eq(&binary, &binary));
    let mut expected = [0u8; 32];
    for index in 0usize..expected.len() { expected[index] = (197usize * index + 131usize) as u8; }
    for index in 0usize..expected.len() {
        let mut changed = expected;
        changed[index] ^= 0xffu8;
        assert!(!constant_time_eq(&expected, &changed));
        assert!(!constant_time_eq(&changed, &expected));
        changed[index] ^= 0xffu8;
        assert!(constant_time_eq(&expected, &changed));
    }
    42
}
