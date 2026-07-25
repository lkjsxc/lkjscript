use super::*;

#[test]
fn sha256_returns_new_exact_digests_for_bounded_binary_ranges() {
    let mut arena = Arena::default();
    let input = buf_new(&mut arena, 6).expect("input buffer");
    for (index, byte) in [b'!', b'a', b'b', b'c', 0, b'?'].into_iter().enumerate() {
        buf_set(&mut arena, input, index as i64, i64::from(byte)).expect("set input byte");
    }
    let digest = sys_sha256(&mut arena, input, 1, 3).expect("hash abc range");
    assert_ne!(digest, input);
    assert_eq!(as_buf(&arena, digest).expect("digest buffer").len(), 32);
    assert_eq!(
        as_buf(&arena, digest).expect("digest buffer"),
        &[
            0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
            0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61,
            0xf2, 0x00, 0x15, 0xad,
        ]
    );
    let empty = sys_sha256(&mut arena, input, 0, 0).expect("hash empty range");
    assert_eq!(
        as_buf(&arena, empty).expect("empty digest"),
        &[
            0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f,
            0xb9, 0x24, 0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b,
            0x78, 0x52, 0xb8, 0x55,
        ]
    );
    let binary = buf_new(&mut arena, 1).expect("binary buffer");
    let binary_digest = sys_sha256(&mut arena, binary, 0, 1).expect("hash NUL byte");
    assert_eq!(
        as_buf(&arena, binary_digest).expect("binary digest"),
        &[
            0x6e, 0x34, 0x0b, 0x9c, 0xff, 0xb3, 0x7a, 0x98, 0x9c, 0xa5, 0x44, 0xe6, 0xbb, 0x78,
            0x0a, 0x2c, 0x78, 0x90, 0x1d, 0x3f, 0xb3, 0x37, 0x38, 0x76, 0x85, 0x11, 0xa3, 0x06,
            0x17, 0xaf, 0xa0, 0x1d,
        ]
    );
    assert!(sys_sha256(&mut arena, input, -1, 1).is_err());
    assert!(sys_sha256(&mut arena, input, 5, 2).is_err());
    assert!(sys_sha256(&mut arena, input, 0, MAX_BULK_IO_BYTES as i64 + 1).is_err());
    assert!(sys_sha256(&mut arena, Value::UNIT, 0, 0).is_err());
}
#[test]
fn buffer_narrowing_rejects_truncation_and_wrapping() {
    let mut arena = Arena::default();
    let buffer = buf_new(&mut arena, 4).expect("buffer");
    assert!(buf_set(&mut arena, buffer, 0, -1).is_err());
    assert!(buf_set(&mut arena, buffer, 0, 256).is_err());
    assert!(buf_set_u32(&mut arena, buffer, 0, -1).is_err());
    assert!(buf_set_u32(&mut arena, buffer, 0, i64::from(u32::MAX) + 1).is_err());
    assert!(buf_set(&mut arena, buffer, 0, 255).is_ok());
    assert!(buf_set_u32(&mut arena, buffer, 0, i64::from(u32::MAX)).is_ok());
}
