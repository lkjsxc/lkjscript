fn literal_bytes() -> &'static [u8] {
    let text = "static\0bytes";
    text.as_bytes()
}

pub fn probe() -> i64 {
    let mut owned = "α".to_string();
    owned.push_str("水\0");
    let text = owned.as_str();
    assert_eq!(text.len(), 6usize);
    assert_eq!(text.as_bytes()[2], 230u8);
    assert_eq!(text.as_bytes()[5], 0u8);
    let bytes = literal_bytes();
    assert_eq!(bytes.len(), 12usize);
    assert_eq!(bytes[6], 0u8);
    let invalid: [u8; 2] = [192u8, 128u8];
    assert!(std::str::from_utf8(&invalid).is_err());
    let valid = std::str::from_utf8(owned.as_bytes()).unwrap();
    assert_eq!(valid.len(), 6usize);
    42
}
