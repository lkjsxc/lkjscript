use super::*;

#[test]
fn bulk_byte_opcodes_reject_malformed_type_stacks() {
    let mut read = unit_chunk();
    read.main.code = vec![
        Op::Unit as u8,
        Op::Unit as u8,
        Op::SysReadInto as u8,
        Op::Return as u8,
    ];
    assert!(error(read).contains("byte view"));

    let mut from = unit_chunk();
    from.main.code = vec![
        Op::Unit as u8,
        Op::ConvertStringToBytes as u8,
        Op::Return as u8,
    ];
    assert!(error(from).contains("string"));

    let mut random = unit_chunk();
    random.main.code = vec![
        Op::Unit as u8,
        Op::Unit as u8,
        Op::SysRandomFill as u8,
        Op::Return as u8,
    ];
    assert!(error(random).contains("byte view"));

    let mut sha256 = unit_chunk();
    sha256.main.code = vec![Op::Unit as u8, Op::SysSha256 as u8, Op::Return as u8];
    assert!(error(sha256).contains("byte view"));

    let mut fsync = unit_chunk();
    fsync.main.code = vec![Op::Unit as u8, Op::SysFsync as u8, Op::Return as u8];
    assert!(error(fsync).contains("typed resource"));

    let mut aggregate_any = unit_chunk();
    aggregate_any.main.code = vec![
        Op::Unit as u8,
        Op::EmptyList as u8,
        Op::Cons as u8,
        Op::Car as u8,
        Op::SysFsync as u8,
        Op::Return as u8,
    ];
    let message = error(aggregate_any);
    assert!(
        message.contains("typed resource kind mismatch: got any"),
        "{message}"
    );
}

#[test]
fn typed_resource_kinds_reject_cross_domain_bytecode() {
    let mut chunk = unit_chunk();
    chunk.required_capabilities = vec![crate::CapabilityKind::Stdio];
    chunk.main.arity = 1;
    chunk.main.locals = 1;
    chunk.main.code = vec![
        Op::LoadLocal as u8,
        0,
        0,
        Op::StdinHandle as u8,
        Op::SysFsync as u8,
        Op::Return as u8,
    ];
    let message = error(chunk);
    assert!(
        message.contains("typed resource kind mismatch"),
        "{message}"
    );

    let mut escape = unit_chunk();
    escape.required_capabilities = vec![crate::CapabilityKind::Stdio];
    escape.main.arity = 1;
    escape.main.locals = 1;
    escape.main.code = vec![
        Op::LoadLocal as u8,
        0,
        0,
        Op::StdinHandle as u8,
        Op::Return as u8,
    ];
    let message = error(escape);
    assert!(message.contains("cannot escape from main"), "{message}");

    let live_local = resource_parameter_chunk(vec![Op::Unit as u8, Op::Return as u8]);
    let message = error(live_local);
    assert!(message.contains("untransferred owner"), "{message}");

    let duplicate = resource_parameter_chunk(vec![
        Op::LoadLocal as u8,
        0,
        Op::Dup as u8,
        Op::Return as u8,
    ]);
    let message = error(duplicate);
    assert!(message.contains("cannot forge a unique owner"), "{message}");

    let mut borrowed =
        resource_parameter_chunk(vec![Op::LoadLocal as u8, 0, Op::ResourceDrop as u8]);
    borrowed.protos[0].parameter_resource_places[0] = None;
    borrowed.protos[0].unique_places = 0;
    assert!(error(borrowed).contains("borrowed resource cannot be consumed"));
}

fn resource_parameter_chunk(code: Vec<u8>) -> Chunk {
    let mut chunk = unit_chunk();
    let mut proto = Chunk::new().main;
    proto.name = "owned-resource-parameter".into();
    proto.arity = 1;
    proto.locals = 1;
    proto.parameter_resources = vec![Some(crate::ResourceKind::FileReader)];
    proto.parameter_resource_places = vec![Some(0)];
    proto.unique_places = 1;
    proto.code = code;
    chunk.protos.push(proto);
    chunk
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
