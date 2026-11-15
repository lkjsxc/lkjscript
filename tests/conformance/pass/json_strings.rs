// expect: value 42
use std::json::{Value, parse, stringify};

fn decoded(encoded: &str, expected: &str) {
    let value = parse(encoded.as_bytes()).unwrap();
    assert_eq!(value.as_str().unwrap(), expected);
    let written = stringify(&value).unwrap();
    let again = parse(written.as_bytes()).unwrap();
    assert_eq!(again.as_str().unwrap(), expected);
}
pub fn probe() -> i64 {
    decoded("\"\\\"\\\\\\/\\b\\f\\n\\r\\t\\u0000\"", "\"\\/\u{8}\u{c}\n\r\t\0");
    decoded("\"\\u007f\\u0080\\u07ff\\u0800\\uD7ff\\ue000\\uFFFF\"", "\u{7f}\u{80}\u{7ff}\u{800}\u{d7ff}\u{e000}\u{ffff}");
    decoded("\"\\ud800\\udc00\\ud834\\udd1e\\uDBFF\\uDFFF\"", "\u{10000}𝄞\u{10ffff}");
    decoded("\"é水ह😀e\u{301}\u{2028}\u{2029}\"", "é水ह😀e\u{301}\u{2028}\u{2029}");
    let mut controls = Vec::new();
    for byte in 0u8..32u8 {
        controls.push(byte);
        let raw = vec![b'"', byte, b'"'];
        let error = parse(&raw).unwrap_err();
        assert_eq!(error.offset, 1usize);
        assert_eq!(error.message, "unescaped control character in string");
    }
    let original = String::from_utf8(controls).unwrap();
    let value = Value::String(original.clone());
    let written = stringify(&value).unwrap();
    assert_eq!(parse(written.as_bytes()).unwrap().as_str().unwrap(), original.as_str());
    let malformed = vec![
        vec![b'"', 0x80u8, b'"'],
        vec![b'"', 0xc0u8, 0xafu8, b'"'],
        vec![b'"', 0xc1u8, 0xbfu8, b'"'],
        vec![b'"', 0xe0u8, 0x80u8, 0x80u8, b'"'],
        vec![b'"', 0xedu8, 0xa0u8, 0x80u8, b'"'],
        vec![b'"', 0xf0u8, 0x80u8, 0x80u8, 0x80u8, b'"'],
        vec![b'"', 0xf4u8, 0x90u8, 0x80u8, 0x80u8, b'"'],
        vec![b'"', 0xf5u8, 0x80u8, 0x80u8, 0x80u8, b'"'],
        vec![b'"', 0xe2u8, 0x28u8, 0xa1u8, b'"'],
        vec![b'"', 0xf0u8, 0x90u8, 0x28u8, 0x80u8, b'"'],
        vec![b'"', 0xe2u8, 0x82u8],
        vec![b'n', b'u', b'l', b'l', 0xffu8],
    ];
    for bytes in &malformed {
        let error = parse(bytes).unwrap_err();
        assert_eq!(error.message, "invalid UTF-8");
        assert!(error.offset < bytes.len());
    }
    let invalid = vec![
        "\"\\x20\"".to_string(), "\"\\U0000\"".to_string(), "\"\\u12x4\"".to_string(), "\"\\u123\"".to_string(),
        "\"\\ud800\"".to_string(), "\"\\ud800x\"".to_string(), "\"\\ud800\\n\"".to_string(), "\"\\ud800\\u0041\"".to_string(),
        "\"\\ud800\\ud800\"".to_string(), "\"\\udc00\"".to_string(), "\"\\udfff\"".to_string(), "\"\\udc00\\ud800\"".to_string(),
    ];
    for index in 0usize..invalid.len() { assert!(parse(invalid[index].as_bytes()).is_err()); }
    assert_eq!(parse(b"\"a\\x\"").unwrap_err().offset, 3usize);
    assert_eq!(parse(b"\"\\u12x4\"").unwrap_err().offset, 5usize);
    assert_eq!(parse(b"\"\\udc00\"").unwrap_err().offset, 1usize);
    42
}
