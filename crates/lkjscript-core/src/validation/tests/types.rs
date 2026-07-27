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
    assert!(error(read).contains("i64"));

    let mut from = unit_chunk();
    from.main.code = vec![Op::Unit as u8, Op::BufFromStr as u8, Op::Return as u8];
    assert!(error(from).contains("string"));

    let mut random = unit_chunk();
    random.main.code = vec![
        Op::Unit as u8,
        Op::Unit as u8,
        Op::Unit as u8,
        Op::Unit as u8,
        Op::SysRandomFill as u8,
        Op::Return as u8,
    ];
    assert!(error(random).contains("i64"));

    let mut sha256 = unit_chunk();
    sha256.main.code = vec![
        Op::Unit as u8,
        Op::Unit as u8,
        Op::Unit as u8,
        Op::SysSha256 as u8,
        Op::Return as u8,
    ];
    assert!(error(sha256).contains("i64"));

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
}

#[test]
fn resource_call_and_return_metadata_is_enforced() {
    let mut call = unit_chunk();
    let prototype = call.add_const(Constant::Proto(0));
    call.protos.push(FunctionProto {
        name: "resource-parameter".into(),
        arity: 1,
        locals: 1,
        parameter_resources: vec![Some(crate::ResourceKind::FileReader)],
        return_resource: None,
        code: vec![Op::LoadLocal as u8, 0, 0, Op::Return as u8],
    });
    call.main.code.clear();
    call.main.emit(Op::Unit);
    call.main.emit_op_u16(Op::LoadConst, prototype.0);
    call.main.emit(Op::MakeClosure);
    call.main.emit_u16(0);
    call.main.emit_op_u8(Op::Call, 1);
    call.main.emit(Op::Return);
    let message = error(call);
    assert!(
        message.contains("call argument does not match"),
        "{message}"
    );

    let mut returned = unit_chunk();
    returned.protos.push(FunctionProto {
        name: "resource-return".into(),
        arity: 0,
        locals: 0,
        parameter_resources: Vec::new(),
        return_resource: Some(crate::ResourceReturnKind::Resource(
            crate::ResourceKind::FileReader,
        )),
        code: vec![Op::Unit as u8, Op::Return as u8],
    });
    let message = error(returned);
    assert!(message.contains("return does not match"), "{message}");
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
