// expect: value 42
// Independent binary/padding vectors cross-checked using OpenSSL 3.0.13
// "openssl dgst -sha256 -mac HMAC -macopt hexkey:..." (empty key: -hmac "").
// Key byte i is (197*i+131)%256; message byte i is (113*i+29)%256.
// The construction is https://www.rfc-editor.org/rfc/rfc2104.html#section-2 .
use std::hmac;

fn vector(key: &[u8], message: &[u8], expected: &str) {
    let digest = hmac::sha256(key, message);
    let digits = b"0123456789abcdef";
    let mut actual = String::new();
    for byte in &digest[..] {
        actual.push(digits[(*byte as usize) / 16usize] as char);
        actual.push(digits[(*byte as usize) % 16usize] as char);
    }
    assert!(actual == expected);
}
pub fn probe() -> i64 {
    let mut key = [0u8; 131];
    let mut message = [0u8; 128];
    for index in 0usize..key.len() { key[index] = (197usize * index + 131usize) as u8; }
    for index in 0usize..message.len() { message[index] = (113usize * index + 29usize) as u8; }
    vector(b"", b"", "b613679a0814d9ec772f95d778c35fc5ff1697c493715653c6c712144292c5ad");
    vector(b"", &message[..1], "a47c13e12c99fa6c62a970be139debdeb8cc78d0ea43f5d9df06f9d88fb0f6a1");
    vector(&key[..1], &message[..55], "5e513c65bf2750c814b733f8f4dfbc0770903919e893142ba54a5270f06834f3");
    vector(&key[..32], &message[..56], "558d06896fac2d679d3b4a3f7ce2b46b274483de27b802257481ddfabec982f2");
    vector(&key[..63], &message[..63], "9a37ab30f511f34a45f15f09c5dc90d014c81f251c2fbe68993404bbbe0bbd0d");
    vector(&key[..64], &message[..64], "3001dddb233adaba77786ccd5e634020431f2f63a3545c39795bdab6c1d8c92c");
    vector(&key[..65], &message[..65], "e3dca18c23180e243f792cfab52e1ebc3b0639a72fd32061a3ddf37d5d61fc24");
    vector(&key[..64], &message[..119], "5f8052605d76223a9297a3247d79211dc082309a994f8472bb7ce90a3de50a89");
    vector(&key, &message[..120], "06027a2e3f23963215e1e333208a511b24b3a76767c822f8ac158d1ea53189e4");
    vector(&key[..65], &message, "3ab5a94c1ec27127561af789697800a7602e1b0c7247e79c8d4831ae37ea9f79");
    42
}
