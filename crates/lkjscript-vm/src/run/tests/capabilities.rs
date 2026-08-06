use super::*;

fn print_chunk(required: lkjscript_core::CapabilityKind) -> lkjscript_core::ValidatedChunk {
    let mut chunk = Chunk::new();
    chunk.required_capabilities = vec![required];
    chunk.main.arity = 1;
    chunk.main.locals = 1;
    let text = chunk.add_const(Constant::Str("not emitted".into()));
    chunk.main.emit_op_u64(Op::LoadLocal, 0);
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
                host: lkjscript_host::HostEnvironment::default(),
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
fn stdio_and_clock_operations_use_only_granted_providers() {
    let stdio = lkjscript_host::BufferedStdio::default();
    let host = lkjscript_host::HostEnvironment {
        stdio: Some(std::sync::Arc::new(stdio.clone())),
        ..lkjscript_host::HostEnvironment::default()
    };
    let output = Vm::new(
        &print_chunk(lkjscript_core::CapabilityKind::Stdio),
        NullJit,
        crate::ExecutionInputs {
            arguments: Vec::new(),
            capabilities: vec![lkjscript_core::CapabilityKind::Stdio],
            host,
        },
        ExecutionConfig::default(),
    )
    .run();
    assert!(matches!(output, ExecutionOutcome::Returned(_)));
    assert_eq!(stdio.output(), Ok(b"not emitted".to_vec()));
    assert_eq!(stdio.flushes(), Ok(1));

    let mut chunk = Chunk::new();
    chunk.required_capabilities = vec![lkjscript_core::CapabilityKind::Clock];
    chunk.main.arity = 1;
    chunk.main.locals = 1;
    chunk.main.emit_op_u64(Op::LoadLocal, 0);
    chunk.main.emit(Op::SysNowMs);
    chunk.main.emit(Op::Return);
    let clock = validate(chunk);
    let output = Vm::new(
        &clock,
        NullJit,
        crate::ExecutionInputs {
            arguments: Vec::new(),
            capabilities: vec![lkjscript_core::CapabilityKind::Clock],
            host: lkjscript_host::HostEnvironment::portable(),
        },
        ExecutionConfig::default(),
    )
    .run();
    assert!(matches!(
        output,
        ExecutionOutcome::Trapped(ref error)
            if error.as_str().contains("lacks exact structural type metadata")
    ));
}

#[test]
fn matching_capability_without_provider_fails_before_ambient_effect() {
    let outcome = Vm::new(
        &print_chunk(lkjscript_core::CapabilityKind::Stdio),
        NullJit,
        crate::ExecutionInputs {
            arguments: Vec::new(),
            capabilities: vec![lkjscript_core::CapabilityKind::Stdio],
            host: lkjscript_host::HostEnvironment::default(),
        },
        ExecutionConfig::default(),
    )
    .run();
    assert!(
        matches!(
            outcome.primary(),
            ExecutionOutcome::HostFailure(error)
                if error.as_str().contains("stdio capability has no granted provider")
        ),
        "unexpected outcome: {outcome:?}"
    );
}

#[test]
fn bytecode_cannot_use_one_capability_as_another() {
    let mut chunk = Chunk::new();
    chunk.required_capabilities = vec![lkjscript_core::CapabilityKind::Arguments];
    chunk.main.arity = 1;
    chunk.main.locals = 1;
    let text = chunk.add_const(Constant::Str("not emitted".into()));
    chunk.main.emit_op_u64(Op::LoadLocal, 0);
    chunk.main.emit_op_u16(Op::LoadConst, text.0);
    chunk.main.emit(Op::Print);
    chunk.main.emit(Op::Return);
    let error = validate_chunk(chunk, &ValidationLimits::default())
        .expect_err("wrong capability operand")
        .to_string();
    assert!(error.contains("capability stdio"));
}
