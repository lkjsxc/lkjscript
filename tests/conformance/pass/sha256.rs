// expect: value 42
// Standard SHA-256 vectors: https://www.rfc-editor.org/rfc/rfc6234.html
use std::sha256::{Sha256, digest, hex_digest};

fn vector(input: &[u8], expected: &str) {
    assert!(hex_digest(input) == expected);
}

pub fn probe() -> i64 {
    vector(b"", "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    vector(b"abc", "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    vector(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1");
    vector(b"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu",
        "cf5b16a778af8380036ce59e7b0492370b249b11e8f07a51afac45037afee9d1");
    // Padding and block boundaries, cross-checked with sha256sum.
    let data = [b'a'; 129];
    assert!(hex_digest(&data[..55]) == "9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318");
    assert!(hex_digest(&data[..56]) == "b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a");
    assert!(hex_digest(&data[..63]) == "7d3e74a05d7db15bce4ad9ec0658ea98e3f06eeecf16b4c6fff2da457ddc2f34");
    assert!(hex_digest(&data[..64]) == "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb");
    assert!(hex_digest(&data[..65]) == "635361c48bb9eab14198e76ea8ab7f1a41685d6ad62aa9146d301d4f17eb0ae0");
    assert!(hex_digest(&data[..119]) == "31eba51c313a5c08226adf18d4a359cfdfd8d2e816b13f4af952f7ea6584dcfb");
    assert!(hex_digest(&data[..120]) == "2f3d335432c70b580af0e8e1b3674a7c020d683aa5f73aaaedfdc55af904c21c");
    assert!(hex_digest(&data[..127]) == "c57e9278af78fa3cab38667bef4ce29d783787a2f731d4e12200270f0c32320a");
    assert!(hex_digest(&data[..128]) == "6836cf13bac400e9105071cd6af47084dfacad4e5e302c94bfed24e013afb73e");
    assert!(hex_digest(&data[..129]) == "c12cb024a2e5551cca0e08fce8f1c5e314555cc3fef6329ee994a3db752166ae");
    let mut hash = Sha256::new();
    hash.update(&data[..55]);
    hash.update(b"");
    hash.update(&data[55..56]);
    hash.update(&data[56..63]);
    hash.update(&data[63..64]);
    hash.update(&data[64..]);
    assert!(hash.finish() == digest(&data));
    42
}

fn streaming_vector(input: &[u8], expected: &str) {
    assert!(hex_digest(input) == expected);
    let whole = digest(input);
    let sizes = [1usize, 7usize, 55usize, 56usize, 63usize, 64usize, 65usize];
    let mut index = 0usize;
    while index < sizes.len() {
        let size = sizes[index];
        let mut hash = Sha256::new();
        hash.update(b"");
        let mut at = 0usize;
        while at < input.len() {
            let end = if input.len() - at < size { input.len() } else { at + size };
            hash.update(&input[at..end]);
            hash.update(b"");
            at = end;
        }
        assert!(hash.finish() == whole);
        index += 1;
    }
}

// Exhaustive split/streaming checks are cheap natively; run with --entry streaming_splits.
pub fn streaming_splits() -> i64 {
    streaming_vector(b"", "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    streaming_vector(b"abc", "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    let mut binary = [0u8; 130];
    let mut i = 0usize;
    while i < binary.len() {
        binary[i] = (i * 197 + 131) as u8;
        i += 1;
    }
    let whole = digest(&binary);
    // Every possible split includes empty prefixes, suffixes and buffered/full blocks.
    i = 0;
    while i <= binary.len() {
        let mut hash = Sha256::new();
        hash.update(&binary[..i]);
        hash.update(&binary[i..]);
        assert!(hash.finish() == whole);
        i += 1;
    }
    42
}

// Exercise explicitly with --entry million_a --engine native; the gate's
// interpreted corpus uses the short standard and boundary vectors above.
pub fn million_a() -> i64 {
    let block = [b'a'; 1000];
    let mut hash = Sha256::new();
    let mut i = 0usize;
    while i < 1000 {
        hash.update(&block);
        i += 1;
    }
    let expected = [
        0xcdu8, 0xc7u8, 0x6eu8, 0x5cu8, 0x99u8, 0x14u8, 0xfbu8, 0x92u8,
        0x81u8, 0xa1u8, 0xc7u8, 0xe2u8, 0x84u8, 0xd7u8, 0x3eu8, 0x67u8,
        0xf1u8, 0x80u8, 0x9au8, 0x48u8, 0xa4u8, 0x97u8, 0x20u8, 0x0eu8,
        0x04u8, 0x6du8, 0x39u8, 0xccu8, 0xc7u8, 0x11u8, 0x2cu8, 0xd0u8,
    ];
    assert!(hash.finish() == expected);
    42
}
