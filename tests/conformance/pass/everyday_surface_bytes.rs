fn literal() -> &'static [u8; 3] { b"\xff\0x" }
fn slice() -> &'static [u8] { b"\xff\0x" }
fn text() -> &'static str { "same" }
fn bytes() -> &'static [u8; 4] { b"same" }
pub fn probe() -> i64 {
    let value = literal();
    assert_eq!(value[0], 255u8);
    assert_eq!(value[1], 0u8);
    assert_eq!(slice().len(), 3usize);
    assert_eq!(slice()[2], b'x');
    assert_eq!(b"ab", b"ab");
    assert_ne!(b"ab", b"ac");
    assert_eq!(text().len(), bytes().len());
    assert_eq!(text().as_bytes()[2], bytes()[2]);
    let empty: &[u8] = b"";
    assert_eq!(empty.len(), 0usize);
    let owned = *b"copy";
    assert_eq!(owned[0], b'c');
    42
}
