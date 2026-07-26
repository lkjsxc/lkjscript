use super::*;

#[test]
fn bulk_byte_opcodes_reject_malformed_type_stacks() {
    let mut read = unit_chunk();
    read.main.code = vec![
        Op::Unit as u8,
        Op::Unit as u8,
        Op::Unit as u8,
        Op::Unit as u8,
        Op::SysReadInto as u8,
        Op::Return as u8,
    ];
    assert!(error(read).contains("I64"));

    let mut from = unit_chunk();
    from.main.code = vec![Op::Unit as u8, Op::BufFromStr as u8, Op::Return as u8];
    assert!(error(from).contains("Str"));

    let mut random = unit_chunk();
    random.main.code = vec![
        Op::Unit as u8,
        Op::Unit as u8,
        Op::Unit as u8,
        Op::Unit as u8,
        Op::SysRandomFill as u8,
        Op::Return as u8,
    ];
    assert!(error(random).contains("I64"));

    let mut sha256 = unit_chunk();
    sha256.main.code = vec![
        Op::Unit as u8,
        Op::Unit as u8,
        Op::Unit as u8,
        Op::SysSha256 as u8,
        Op::Return as u8,
    ];
    assert!(error(sha256).contains("I64"));

    let mut fsync = unit_chunk();
    fsync.main.code = vec![Op::Unit as u8, Op::SysFsync as u8, Op::Return as u8];
    assert!(error(fsync).contains("Handle"));
}

#[test]
fn random_and_small_byte_chunks_never_panic() {
    let mut seed = 0x9e37_79b9_u32;
    for length in 0..=32_usize {
        for _ in 0..128 {
            let mut chunk = Chunk::new();
            for _ in 0..length {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                chunk.main.code.push(seed.to_le_bytes()[0]);
            }
            let _result = validate_chunk(chunk, &ValidationLimits::default());
        }
    }
}
