//! Small owned incremental SHA-256 implementation for fixed content identities.

const INITIAL_STATE: [u32; 8] = [
    0x6a09_e667,
    0xbb67_ae85,
    0x3c6e_f372,
    0xa54f_f53a,
    0x510e_527f,
    0x9b05_688c,
    0x1f83_d9ab,
    0x5be0_cd19,
];

const ROUND_CONSTANTS: [u32; 64] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

/// Incremental SHA-256 state with a fixed-size working set.
#[derive(Clone)]
pub struct Sha256 {
    state: [u32; 8],
    buffer: [u8; 64],
    buffer_len: usize,
    bit_length: u64,
}

impl Default for Sha256 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha256 {
    /// Create an empty SHA-256 state.
    pub const fn new() -> Self {
        Self {
            state: INITIAL_STATE,
            buffer: [0; 64],
            buffer_len: 0,
            bit_length: 0,
        }
    }

    /// Add bytes to the digest without allocating.
    ///
    /// SHA-256 encodes the low 64 bits of the message bit length, so length accounting wraps as
    /// required by the algorithm while every supplied byte is still compressed.
    pub fn update(&mut self, mut input: &[u8]) {
        if self.buffer_len != 0 {
            let take = (64 - self.buffer_len).min(input.len());
            let end = self.buffer_len + take;
            self.buffer[self.buffer_len..end].copy_from_slice(&input[..take]);
            self.add_short_length(take);
            self.buffer_len = end;
            input = &input[take..];
            if self.buffer_len == 64 {
                compress(&mut self.state, &self.buffer);
                self.buffer_len = 0;
            } else {
                return;
            }
        }

        let mut blocks = input.chunks_exact(64);
        for block in &mut blocks {
            compress(&mut self.state, block);
            self.bit_length = self.bit_length.wrapping_add(512);
        }
        let remainder = blocks.remainder();
        self.buffer[..remainder.len()].copy_from_slice(remainder);
        self.buffer_len = remainder.len();
        self.add_short_length(remainder.len());
    }

    /// Finish the digest.
    pub fn finish(mut self) -> [u8; 32] {
        self.buffer[self.buffer_len] = 0x80;
        self.buffer[self.buffer_len + 1..].fill(0);
        if self.buffer_len >= 56 {
            compress(&mut self.state, &self.buffer);
            self.buffer.fill(0);
        }
        self.buffer[56..].copy_from_slice(&self.bit_length.to_be_bytes());
        compress(&mut self.state, &self.buffer);

        let mut output = [0_u8; 32];
        for (index, word) in self.state.iter().enumerate() {
            output[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        output
    }

    fn add_short_length(&mut self, byte_length: usize) {
        debug_assert!(byte_length <= 64);
        for _ in 0..byte_length {
            self.bit_length = self.bit_length.wrapping_add(8);
        }
    }
}

/// Return the SHA-256 digest of `input`.
pub fn sha256(input: &[u8]) -> [u8; 32] {
    let mut state = Sha256::new();
    state.update(input);
    state.finish()
}

fn compress(state: &mut [u32; 8], block: &[u8]) {
    let mut schedule = [0_u32; 64];
    for (index, word) in schedule.iter_mut().take(16).enumerate() {
        let start = index * 4;
        let mut bytes = [0_u8; 4];
        bytes.copy_from_slice(&block[start..start + 4]);
        *word = u32::from_be_bytes(bytes);
    }
    for index in 16..64 {
        let s0 = schedule[index - 15].rotate_right(7)
            ^ schedule[index - 15].rotate_right(18)
            ^ (schedule[index - 15] >> 3);
        let s1 = schedule[index - 2].rotate_right(17)
            ^ schedule[index - 2].rotate_right(19)
            ^ (schedule[index - 2] >> 10);
        schedule[index] = schedule[index - 16]
            .wrapping_add(s0)
            .wrapping_add(schedule[index - 7])
            .wrapping_add(s1);
    }

    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *state;
    for (word, constant) in schedule.into_iter().zip(ROUND_CONSTANTS) {
        let sum1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let choose = (e & f) ^ ((!e) & g);
        let temporary1 = h
            .wrapping_add(sum1)
            .wrapping_add(choose)
            .wrapping_add(constant)
            .wrapping_add(word);
        let sum0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let majority = (a & b) ^ (a & c) ^ (b & c);
        let temporary2 = sum0.wrapping_add(majority);
        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(temporary1);
        d = c;
        c = b;
        b = a;
        a = temporary1.wrapping_add(temporary2);
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

#[cfg(test)]
mod tests {
    use super::{sha256, Sha256};

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    #[test]
    fn nist_known_answer_vectors() {
        for (input, expected) in [
            (
                b"".as_slice(),
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            ),
            (
                b"abc".as_slice(),
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            ),
            (
                b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq".as_slice(),
                "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
            ),
        ] {
            assert_eq!(hex(&sha256(input)), expected);
        }

        let mut million_a = Sha256::new();
        let chunk = [b'a'; 1_000];
        for _ in 0..1_000 {
            million_a.update(&chunk);
        }
        assert_eq!(
            hex(&million_a.finish()),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }

    #[test]
    fn streaming_matches_one_shot_across_adversarial_boundaries() {
        let vectors = [
            b"".as_slice(),
            b"abc".as_slice(),
            b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq".as_slice(),
            b"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdefx".as_slice(),
        ];
        let patterns: &[&[usize]] = &[
            &[1],
            &[2, 1, 3],
            &[55, 1, 7],
            &[56, 8],
            &[63, 1, 64, 1],
            &[64],
            &[65, 3],
        ];

        for input in vectors {
            for pattern in patterns {
                let mut state = Sha256::new();
                let mut offset = 0;
                let mut step = 0;
                while offset < input.len() {
                    let chunk_len = pattern[step % pattern.len()];
                    let end = offset.saturating_add(chunk_len).min(input.len());
                    state.update(&input[offset..end]);
                    offset = end;
                    step += 1;
                }
                state.update(&[]);
                assert_eq!(state.finish(), sha256(input));
            }
        }
    }
}
