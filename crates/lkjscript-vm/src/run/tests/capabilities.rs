use super::*;

fn print_chunk(required: lkjscript_core::CapabilityKind) -> lkjscript_core::ValidatedChunk {
    let mut chunk = Chunk::new();
    chunk.required_capabilities = vec![required];
    chunk.main.arity = 1;
    chunk.main.locals = 1;
    let text = chunk.add_const(Constant::Str("not emitted".into()));
    chunk.main.emit_op_u16(Op::LoadLocal, 0);
    chunk.main.emit_op_u16(Op::LoadConst, text.0);
    chunk.main.emit(Op::Print);
    chunk.main.emit(Op::Return);
    validate(chunk)
}

#[test]
fn missing_and_wrong_grants_fail_before_host_dispatch() {
    let chunk = print_chunk(lkjscript_core::CapabilityKind::Stdio);
    for capabilities in [Vec::new(), vec![lkjscript_core::CapabilityKind::Arguments]] {
        let outcome = Vm::new(
            &chunk,
            NullJit,
            crate::ExecutionInputs {
                arguments: Vec::new(),
                capabilities,
            },
            ExecutionConfig::default(),
        )
        .run();
        assert!(matches!(
            outcome,
            ExecutionOutcome::Trapped(ref trap)
                if trap.as_str().contains("execution capability mismatch")
        ));
    }
}

#[test]
fn bytecode_cannot_use_one_capability_as_another() {
    let mut chunk = Chunk::new();
    chunk.required_capabilities = vec![lkjscript_core::CapabilityKind::Arguments];
    chunk.main.arity = 1;
    chunk.main.locals = 1;
    let text = chunk.add_const(Constant::Str("not emitted".into()));
    chunk.main.emit_op_u16(Op::LoadLocal, 0);
    chunk.main.emit_op_u16(Op::LoadConst, text.0);
    chunk.main.emit(Op::Print);
    chunk.main.emit(Op::Return);
    let error = validate_chunk(chunk, &ValidationLimits::default())
        .expect_err("wrong capability operand")
        .to_string();
    assert!(error.contains("Capability(Stdio)"));
}
