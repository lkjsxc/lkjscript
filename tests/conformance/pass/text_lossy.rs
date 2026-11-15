// expect: value 42
use std::text::utf8_lossy;
fn vector(bytes: &[u8], expected: &str) { assert!(utf8_lossy(bytes) == expected); }
pub fn probe() -> i64 {
    vector(b"", "");
    vector(b"ASCII\0text", "ASCII\0text");
    vector("café π 🐈".as_bytes(), "café π 🐈");
    vector(b"\xc2\x80", "\u{80}");
    vector(b"\xdf\xbf", "\u{7ff}");
    vector(b"\xe0\xa0\x80", "\u{800}");
    vector(b"\xed\x9f\xbf", "\u{d7ff}");
    vector(b"\xee\x80\x80", "\u{e000}");
    vector(b"\xef\xbf\xbf", "\u{ffff}");
    vector(b"\xf0\x90\x80\x80", "\u{10000}");
    vector(b"\xf4\x8f\xbf\xbf", "\u{10ffff}");
    vector(b"\x80\xbf\xc0\xc1\xf5\xff", "������");
    vector(b"\xc2", "�");
    vector(b"\xc2A", "�A");
    vector(b"\xc2\xc2\xa2", "�¢");
    vector(b"\xc0\xaf", "��");
    vector(b"\xe0", "�");
    vector(b"\xe0\xa0", "�");
    vector(b"\xe0\x9f\x80", "���");
    vector(b"\xed\xa0\x80", "���");
    vector(b"\xed\x9f", "�");
    vector(b"\xe2\x82", "�");
    vector(b"\xe2\x82A", "�A");
    vector(b"\xe2A\x82", "�A�");
    vector(b"\xe1\x80\xe1\x80\x80", "�\u{1000}");
    vector(b"\xf0", "�");
    vector(b"\xf0\x90", "�");
    vector(b"\xf0\x90\x80", "�");
    vector(b"\xf0\x8f\xbf\xbf", "����");
    vector(b"\xf4\x90\x80\x80", "����");
    vector(b"\xf0\x9fA\x80", "�A�");
    vector(b"\xf0\x9f\x92A", "�A");
    vector(b"\xf0\x90\x80\xf0\x90\x80\x80", "�\u{10000}");
    vector(b"\xf5\x80\x80\x80", "����");
    vector(b"\xe2\x82\xff", "��");
    42
}
