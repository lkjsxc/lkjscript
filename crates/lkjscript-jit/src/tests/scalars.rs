use super::*;
use crate::scalar::owned_scalar;

#[test]
fn automatic_stack_representation_decline_is_typed_before_native_entry() {
    let program = terminal_program(
        Terminator::Outcome {
            outcome: StructuredOutcome::DeadlineExceeded,
            detail: None,
        },
        EffectSet::PURE,
    );
    let main = program.program().main;
    let links = lkjscript_ir::BytecodeLinkMetadata {
        main,
        functions: vec![lkjscript_ir::FunctionBytecodeLink {
            function: main,
            prototype: Some(0),
            is_main: true,
            blocks: Vec::new(),
            instructions: Vec::new(),
        }],
    };
    let mut session = JitSession::new_auto(&program, &links, JitConfig::default());
    session
        .compile_group(main)
        .expect("compile automatic scalar");
    let object = session.objects.first_mut().expect("automatic code object");
    let requirement = object
        .automatic_stack_requirements
        .iter_mut()
        .find(|(function, _)| *function == main)
        .expect("automatic stack requirement");
    requirement.1 = usize::MAX;

    let error = session
        .invoke_scalar(main, &[], &ExecutionPolicy::unrestricted())
        .expect_err("unrepresentable automatic stack requirement must decline");
    assert_eq!(error.code(), FailureCode::NativeStackBoundary);
    assert!(session.objects[0].invalidated);
    assert_eq!(session.native_entries, 0);
    let record = &session.functions[main.index().expect("main index")];
    assert_eq!(record.state(), TierState::Disabled);
    assert_eq!(
        record.last_failure(),
        Some(FailureCode::NativeStackBoundary)
    );

    let after_entry = entered_invocation_error(
        main,
        EnteredInvocationError::NativeStackViolation(
            lkjscript_executable::NativeStackError::GuardReached,
        ),
    );
    assert_eq!(
        after_entry.code(),
        FailureCode::NativeStackBoundaryAfterEntry
    );
}

#[test]
fn detached_native_scalars_retain_exact_payload_without_snapshot_objects() {
    for value in [i64::MIN, -1, 0, 1, i64::MAX] {
        let owned = owned_scalar(NativeValue::I64(value)).expect("owned I64");
        assert_eq!(owned.as_i64(), Some(value));
        assert_eq!(owned.snapshot_object_count(), 0);
    }
    for bits in [
        0_u64,
        1_u64 << 63,
        0x7ff0_0000_0000_0000,
        0x7ff8_0000_0000_0042,
        0xfff8_dead_beef_cafe,
    ] {
        let owned = owned_scalar(NativeValue::F64Bits(bits)).expect("owned F64");
        assert_eq!(owned.as_f64_bits(), Some(bits));
        assert_eq!(owned.snapshot_object_count(), 0);
    }
}
