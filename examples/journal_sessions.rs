//! Signed, expiring write authority for the journal example.
//! The Directory grant is the authority to read the signing key. HTTP never
//! exposes that key or issues tokens. Issuance is a separate local program.
use std::hmac;

const KEY: &str = "session.key";
const STAGING: &str = ".session-key-next";
const DOMAIN: &[u8] = b"lkjscript-journal-session-v1\n";

pub struct Sessions { key: [u8; 32] }
fn io<T>(value: Result<T, i64>, operation: &str) -> Result<T, String> {
    match value {
        Ok(value) => Ok(value),
        Err(errno) => Err(format!("{operation}: errno {errno}")),
    }
}
fn read_key(file: &mut File) -> Result<[u8; 32], String> {
    let mut bytes = [0u8; 33];
    let mut used = 0usize;
    while used < bytes.len() {
        let count = io(file.read(&mut bytes[used..]), "read session key")?;
        if count == 0usize { break; }
        used += count;
    }
    if used != 32usize { return Err("session.key must contain exactly 32 bytes".to_string()); }
    let mut key = [0u8; 32];
    for index in 0usize..32usize { key[index] = bytes[index]; }
    Ok(key)
}
fn abandon(root: &mut Directory) -> Result<(), String> {
    match root.remove(STAGING) {
        Ok(()) | Err(2i64) => Ok(()),
        Err(errno) => Err(format!("remove abandoned session key: errno {errno}")),
    }
}
fn hex(bytes: &[u8]) -> String {
    let alphabet = b"0123456789abcdef";
    let mut text = String::new();
    for byte in bytes {
        text.push(alphabet[(*byte >> 4u8) as usize] as char);
        text.push(alphabet[(*byte & 15u8) as usize] as char);
    }
    text
}
fn digit(byte: u8) -> Option<u8> {
    if byte >= b'0' && byte <= b'9' { Some(byte - b'0') }
    else if byte >= b'a' && byte <= b'f' { Some(byte - b'a' + 10u8) }
    else { None }
}
fn decoded(text: &str) -> Option<[u8; 32]> {
    let bytes = text.as_bytes();
    if bytes.len() != 64usize { return None; }
    let mut result = [0u8; 32];
    for index in 0usize..32usize {
        let high = digit(bytes[index * 2usize])?;
        let low = digit(bytes[index * 2usize + 1usize])?;
        result[index] = high * 16u8 + low;
    }
    Some(result)
}
fn expiry(text: &str) -> Option<i64> {
    let bytes = text.as_bytes();
    if bytes.is_empty() || (bytes.len() > 1usize && bytes[0usize] == b'0') { return None; }
    let mut value = 0i64;
    for byte in bytes {
        if *byte < b'0' || *byte > b'9' { return None; }
        value = value.checked_mul(10i64)?.checked_add((*byte - b'0') as i64)?;
    }
    Some(value)
}
fn message(prefix: &str) -> Vec<u8> {
    let mut bytes = DOMAIN.to_vec();
    bytes.extend_from_slice(prefix.as_bytes());
    bytes
}
impl Sessions {
    /// Call only while holding the journal's writer lock.
    pub fn open(root: &mut Directory, entropy: &mut Entropy) -> Result<Sessions, String> {
        match root.open(KEY) {
            Ok(mut file) => {
                let key = read_key(&mut file)?;
                io(file.set_private(), "restrict session key to owner")?;
                io(file.sync(), "sync session key permissions")?;
                abandon(root)?;
                Ok(Sessions { key })
            }
            Err(2i64) => {
                abandon(root)?;
                let mut key = [0u8; 32];
                io(entropy.fill(&mut key), "generate session signing key")?;
                let mut file = io(root.create_private(STAGING), "create session key staging file")?;
                // Creation is private atomically: another user cannot retain a
                // pre-chmod reader. Then ensure exact owner RW under any umask.
                io(file.set_private(), "restrict new session key to owner")?;
                io(file.write_all(&key), "write session signing key")?;
                io(file.sync(), "sync session signing key")?;
                drop(file);
                io(root.rename(STAGING, KEY), "publish session signing key")?;
                io(root.sync(), "sync session key directory")?;
                Ok(Sessions { key })
            }
            Err(errno) => Err(format!("open session signing key: errno {errno}")),
        }
    }
    /// Read an existing key; token issuance never creates or changes it.
    pub fn read(root: &mut Directory) -> Result<Sessions, String> {
        let mut file = io(root.open(KEY), "open existing session signing key")?;
        let key = read_key(&mut file)?;
        io(file.sync(), "sync issued session key")?;
        // A separate issuer can race the initializer between rename and its
        // directory sync. Flush publication before returning an issued token.
        io(root.sync(), "sync issued session key publication")?;
        Ok(Sessions { key })
    }
    fn signed(&self, now: i64, lifetime: usize, nonce: &[u8; 32]) -> Result<String, String> {
        if now < 0i64 { return Err("session time precedes the Unix epoch".to_string()); }
        if lifetime == 0usize || lifetime > 9_223_372_036_854_775_807usize {
            return Err("session lifetime must be a positive signed number of seconds".to_string());
        }
        let expires = now.checked_add(lifetime as i64).ok_or("session expiry overflow".to_string())?;
        let prefix = format!("v1.{expires}.{}", hex(nonce));
        let digest = hmac::sha256(&self.key, &message(&prefix));
        Ok(format!("{prefix}.{}", hex(&digest)))
    }
    pub fn issue(&self, entropy: &mut Entropy, now: i64, lifetime: usize) -> Result<String, String> {
        let mut nonce = [0u8; 32];
        io(entropy.fill(&mut nonce), "generate session nonce")?;
        self.signed(now, lifetime, &nonce)
    }
    fn valid(&self, token: &str, now: i64) -> bool {
        if now < 0i64 { return false; }
        let mut fields = token.split('.');
        if fields.next() != Some("v1") { return false; }
        let expires_text = match fields.next() { Some(value) => value, None => return false };
        let nonce = match fields.next() { Some(value) => value, None => return false };
        let supplied = match fields.next() { Some(value) => value, None => return false };
        if fields.next().is_some() { return false; }
        let expires = match expiry(expires_text) { Some(value) => value, None => return false };
        if decoded(nonce).is_none() { return false; }
        let supplied = match decoded(supplied) { Some(value) => value, None => return false };
        let prefix = format!("v1.{expires_text}.{nonce}");
        let expected = hmac::sha256(&self.key, &message(&prefix));
        // Token grammar/length are public. Equal-length MAC contents always go
        // through the library's complete byte comparison before expiry decides.
        let authentic = hmac::constant_time_eq(&supplied, &expected);
        authentic && now < expires
    }
    pub fn authorize(&self, header: &str, now: i64) -> bool {
        let bytes = header.as_bytes();
        if bytes.len() < 8usize { return false; }
        let scheme = b"bearer";
        for index in 0usize..6usize {
            let byte = bytes[index];
            let lower = if byte >= b'A' && byte <= b'Z' { byte + 32u8 } else { byte };
            if lower != scheme[index] { return false; }
        }
        if bytes[6usize] != b' ' { return false; }
        let mut start = 7usize;
        while start < bytes.len() && bytes[start] == b' ' { start += 1usize; }
        self.valid(&header[start..], now)
    }
}

#[test]
fn session_format_and_expiration() {
    let mut key = [0u8; 32];
    let mut nonce = [0u8; 32];
    for index in 0usize..32usize { key[index] = index as u8; nonce[index] = (index + 32usize) as u8; }
    let sessions = Sessions { key };
    let token = sessions.signed(0i64, 3600usize, &nonce).unwrap();
    // Independent vector: OpenSSL HMAC-SHA256 of DOMAIN || prefix, key000102..1f.
    let expected = "v1.3600.202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f.b5f527d5a20874efd6756223ce6e490de350e36567e33bac92e0b96739f0a93b";
    assert!(token == expected);
    assert!(sessions.authorize(&format!("Bearer {token}"), 0i64));
    assert!(sessions.authorize(&format!("bEaReR  {token}"), 3599i64));
    assert!(!sessions.authorize(&format!("Bearer {token}"), 3600i64));
    assert!(!sessions.authorize(&format!("Bearer {token}"), -1i64));
    assert!(!sessions.authorize(&token, 1i64));
    assert!(!sessions.authorize(&format!("Bearer {token} "), 1i64));
    assert!(!sessions.authorize(&format!("Bearer {token}.extra"), 1i64));
    assert!(!sessions.authorize(&format!("Basic {token}"), 1i64));
    let other = Sessions { key: [9u8; 32] };
    assert!(!other.authorize(&format!("Bearer {token}"), 1i64));
    assert!(sessions.signed(0i64, 0usize, &nonce).is_err());
    assert!(sessions.signed(9_223_372_036_854_775_807i64, 1usize, &nonce).is_err());
}
#[test]
fn session_tampering_and_canonical_fields() {
    let sessions = Sessions { key: [7u8; 32] };
    let token = sessions.signed(1i64, 99usize, &[3u8; 32]).unwrap();
    let mut changed = token.as_bytes().to_vec();
    for index in 0usize..changed.len() {
        let original = changed[index];
        changed[index] = if original == b'0' { b'1' } else { b'0' };
        let text = String::from_utf8(changed.clone()).unwrap();
        assert!(!sessions.authorize(&format!("Bearer {text}"), 2i64));
        changed[index] = original;
    }
    assert!(expiry("").is_none() && expiry("+1").is_none() && expiry("01").is_none());
    assert!(expiry("-1").is_none() && expiry("1.0").is_none() && expiry("9223372036854775808").is_none());
    assert!(decoded("00").is_none());
    assert!(decoded("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA").is_none());
    assert!(decoded("0000000000000000000000000000000000000000000000000000000000000000").is_some());
}
