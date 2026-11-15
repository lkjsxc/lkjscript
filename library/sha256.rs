//! SHA-256 for byte strings, following FIPS 180-4 / RFC 6234 sections 4–6.
//! The 64-bit bit-length field admits fewer than 2^61 input bytes.
//! This is an unkeyed digest; authenticate messages with HMAC, not digest(key || data).

pub struct Sha256 {
    state: [u32; 8],
    block: [u8; 64],
    used: usize,
    bytes: u64,
}

fn rotate(value: u32, bits: u32) -> u32 {
    (value >> bits) | (value << (32u32 - bits))
}

fn compress(state: &mut [u32; 8], block: &[u8]) {
    let constants = [
        0x428a2f98u32, 0x71374491u32, 0xb5c0fbcfu32, 0xe9b5dba5u32,
        0x3956c25bu32, 0x59f111f1u32, 0x923f82a4u32, 0xab1c5ed5u32,
        0xd807aa98u32, 0x12835b01u32, 0x243185beu32, 0x550c7dc3u32,
        0x72be5d74u32, 0x80deb1feu32, 0x9bdc06a7u32, 0xc19bf174u32,
        0xe49b69c1u32, 0xefbe4786u32, 0x0fc19dc6u32, 0x240ca1ccu32,
        0x2de92c6fu32, 0x4a7484aau32, 0x5cb0a9dcu32, 0x76f988dau32,
        0x983e5152u32, 0xa831c66du32, 0xb00327c8u32, 0xbf597fc7u32,
        0xc6e00bf3u32, 0xd5a79147u32, 0x06ca6351u32, 0x14292967u32,
        0x27b70a85u32, 0x2e1b2138u32, 0x4d2c6dfcu32, 0x53380d13u32,
        0x650a7354u32, 0x766a0abbu32, 0x81c2c92eu32, 0x92722c85u32,
        0xa2bfe8a1u32, 0xa81a664bu32, 0xc24b8b70u32, 0xc76c51a3u32,
        0xd192e819u32, 0xd6990624u32, 0xf40e3585u32, 0x106aa070u32,
        0x19a4c116u32, 0x1e376c08u32, 0x2748774cu32, 0x34b0bcb5u32,
        0x391c0cb3u32, 0x4ed8aa4au32, 0x5b9cca4fu32, 0x682e6ff3u32,
        0x748f82eeu32, 0x78a5636fu32, 0x84c87814u32, 0x8cc70208u32,
        0x90befffau32, 0xa4506cebu32, 0xbef9a3f7u32, 0xc67178f2u32,
    ];
    let mut words = [0u32; 64];
    let mut i = 0usize;
    while i < 16 {
        let at = i * 4;
        words[i] = ((block[at] as u32) << 24u32)
            | ((block[at + 1] as u32) << 16u32)
            | ((block[at + 2] as u32) << 8u32)
            | block[at + 3] as u32;
        i += 1;
    }
    while i < 64 {
        let x = words[i - 15];
        let y = words[i - 2];
        let a = rotate(x, 7u32) ^ rotate(x, 18u32) ^ (x >> 3u32);
        let b = rotate(y, 17u32) ^ rotate(y, 19u32) ^ (y >> 10u32);
        words[i] = words[i - 16].wrapping_add(a).wrapping_add(words[i - 7]).wrapping_add(b);
        i += 1;
    }
    let mut a = state[0];
    let mut b = state[1];
    let mut c = state[2];
    let mut d = state[3];
    let mut e = state[4];
    let mut f = state[5];
    let mut g = state[6];
    let mut h = state[7];
    i = 0;
    while i < 64 {
        let upper = rotate(e, 6u32) ^ rotate(e, 11u32) ^ rotate(e, 25u32);
        let choice = (e & f) ^ ((!e) & g);
        let first = h.wrapping_add(upper).wrapping_add(choice)
            .wrapping_add(constants[i]).wrapping_add(words[i]);
        let lower = rotate(a, 2u32) ^ rotate(a, 13u32) ^ rotate(a, 22u32);
        let majority = (a & b) ^ (a & c) ^ (b & c);
        let second = lower.wrapping_add(majority);
        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(first);
        d = c;
        c = b;
        b = a;
        a = first.wrapping_add(second);
        i += 1;
    }
    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
    state[4] = state[4].wrapping_add(e);
    state[5] = state[5].wrapping_add(f);
    state[6] = state[6].wrapping_add(g);
    state[7] = state[7].wrapping_add(h);
}

impl Sha256 {
    pub fn new() -> Self {
        Sha256 {
            state: [
                0x6a09e667u32, 0xbb67ae85u32, 0x3c6ef372u32, 0xa54ff53au32,
                0x510e527fu32, 0x9b05688cu32, 0x1f83d9abu32, 0x5be0cd19u32,
            ],
            block: [0u8; 64],
            used: 0,
            bytes: 0,
        }
    }

    /// Append arbitrary bytes without retaining a view of the input.
    pub fn update(&mut self, input: &[u8]) {
        assert!((input.len() as u64) <= 2305843009213693951u64 - self.bytes);
        self.bytes += input.len() as u64;
        let mut at = 0usize;
        if self.used != 0 {
            while at < input.len() && self.used < 64 {
                self.block[self.used] = input[at];
                self.used += 1;
                at += 1;
            }
            if self.used == 64 {
                compress(&mut self.state, &self.block);
                self.used = 0;
            }
        }
        while input.len() - at >= 64 {
            compress(&mut self.state, &input[at..at + 64]);
            at += 64;
        }
        while at < input.len() {
            self.block[self.used] = input[at];
            self.used += 1;
            at += 1;
        }
    }

    /// Consume the state and return the 32 big-endian digest bytes.
    pub fn finish(mut self) -> [u8; 32] {
        self.block[self.used] = 128u8;
        self.used += 1;
        if self.used > 56 {
            while self.used < 64 {
                self.block[self.used] = 0;
                self.used += 1;
            }
            compress(&mut self.state, &self.block);
            self.used = 0;
        }
        while self.used < 56 {
            self.block[self.used] = 0;
            self.used += 1;
        }
        let bits = self.bytes * 8u64;
        let mut i = 0usize;
        while i < 8 {
            self.block[56 + i] = (bits >> ((7 - i) * 8)) as u8;
            i += 1;
        }
        compress(&mut self.state, &self.block);
        let mut result = [0u8; 32];
        i = 0;
        while i < 32 {
            result[i] = (self.state[i / 4] >> ((3 - i % 4) * 8)) as u8;
            i += 1;
        }
        result
    }
}

pub fn digest(input: &[u8]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(input);
    hash.finish()
}

/// Lowercase hexadecimal for content-addressed names.
pub fn hex_digest(input: &[u8]) -> String {
    let bytes = digest(input);
    let alphabet = b"0123456789abcdef";
    let mut result = String::new();
    let mut i = 0usize;
    while i < 32 {
        result.push(alphabet[(bytes[i] >> 4u8) as usize] as char);
        result.push(alphabet[(bytes[i] & 15u8) as usize] as char);
        i += 1;
    }
    result
}
