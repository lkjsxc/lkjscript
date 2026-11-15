// expect: value 42
use std::json::parse;

pub fn probe() -> i64 {
    let invalid = vec![
        "".to_string(), " ".to_string(), "\t\r\n".to_string(), "nul".to_string(), "TRUE".to_string(), "falsefalse".to_string(), "true false".to_string(), "null\0".to_string(),
        "[]x".to_string(), "[".to_string(), "[,]".to_string(), "[1,]".to_string(), "[1 2]".to_string(), "[1,,2]".to_string(), "{".to_string(), "{\"a\"}".to_string(),
        "{\"a\":}".to_string(), "{\"a\":1,}".to_string(), "{a:1}".to_string(), "{\"a\":1 \"b\":2}".to_string(), "{\"a\":1,,\"b\":2}".to_string(),
        "[/*comment*/1]".to_string(), "//comment\nnull".to_string(), "+1".to_string(), "--1".to_string(), ".1".to_string(), "1.".to_string(), "01".to_string(), "-01".to_string(),
        "1e".to_string(), "1e+".to_string(), "1e-".to_string(), "NaN".to_string(), "Infinity".to_string(), "-Infinity".to_string(), "1e1.0".to_string(), "1_000".to_string(),
        "0x10".to_string(), "[1e999x]".to_string(), "\"".to_string(), "\"abc".to_string(), "\"\\".to_string(), "\u{feff}null".to_string(),
        "\u{a0}null".to_string(), "\u{b}null".to_string(), "\u{c}null".to_string(), "{\"a\":1,\"a\":2}".to_string(),
        "{\"a\":1,\"\\u0061\":2}".to_string(), "{\"𝄞\":1,\"\\ud834\\udd1e\":2}".to_string(),
    ];
    for index in 0usize..invalid.len() {
        assert!(parse(invalid[index].as_bytes()).is_err());
    }
    assert_eq!(parse(b"   ").unwrap_err().offset, 3usize);
    assert_eq!(parse(b"[1,]").unwrap_err().offset, 3usize);
    assert_eq!(parse(b"{\"a\" 1}").unwrap_err().offset, 5usize);
    assert_eq!(parse(b"true x").unwrap_err().offset, 5usize);
    assert_eq!(parse(b"01").unwrap_err().offset, 1usize);
    assert_eq!(parse(b"1e+").unwrap_err().offset, 3usize);
    let duplicate = parse(b"{\"a\":1,\"\\u0061\":2}").unwrap_err();
    assert_eq!(duplicate.offset, 7usize);
    assert_eq!(duplicate.message, "duplicate object key");
    assert_eq!(format!("{duplicate}"), "JSON at byte 7: duplicate object key");
    let document = "{\"a\":[{\"s\":\"é\\u0041\"},true,12.5e+6]}";
    let bytes = document.as_bytes();
    for length in 0usize..bytes.len() { assert!(parse(&bytes[..length]).is_err()); }
    assert!(parse(bytes).is_ok());
    42
}
