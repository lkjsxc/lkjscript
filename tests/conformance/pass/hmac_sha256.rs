// expect: value 42
// HMAC-SHA-256 vectors 1 through 6, including the specified 128-bit prefix:
// https://www.rfc-editor.org/rfc/rfc4231.html#section-4
use std::hmac;

fn hex(bytes: &[u8]) -> String {
    let digits = b"0123456789abcdef";
    let mut text = String::new();
    for byte in bytes {
        text.push(digits[(*byte as usize) / 16usize] as char);
        text.push(digits[(*byte as usize) % 16usize] as char);
    }
    text
}
fn vector(key: &[u8], message: &[u8], expected: &str) {
    let digest = hmac::sha256(key, message);
    assert!(hex(&digest) == expected);
}
pub fn probe() -> i64 {
    let key1 = [0x0bu8; 20];
    vector(&key1, b"Hi There",
        "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7");
    vector(b"Jefe", b"what do ya want for nothing?",
        "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843");
    let key3 = [0xaau8; 20];
    let message3 = [0xddu8; 50];
    vector(&key3, &message3,
        "773ea91e36800e46854db8ebd09181a72959098b3ef8c122d9635514ced565fe");
    let mut key4 = [0u8; 25];
    for index in 0usize..key4.len() { key4[index] = (index + 1usize) as u8; }
    let message4 = [0xcdu8; 50];
    vector(&key4, &message4,
        "82558a389a443c0ea4cc819899f2083a85f0faa3e578f8077a2e3ff46729665b");
    let key5 = [0x0cu8; 20];
    let digest5 = hmac::sha256(&key5, b"Test With Truncation");
    assert!(hex(&digest5[..16]) == "a3b6167473100ee06e0c796c2955552b");
    let key6 = [0xaau8; 131];
    vector(&key6, b"Test Using Larger Than Block-Size Key - Hash Key First",
        "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54");
    42
}
