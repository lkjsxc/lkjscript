#[test]
fn known_digest() {
    assert_eq!(
        crate::sha256::digest(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        crate::sha256::digest(&[b'a'; 76]),
        "f8b8aba652e5b3cde6bc74bcb7bff15289a222cfdb7759a9809dc08574911f60"
    );
}
