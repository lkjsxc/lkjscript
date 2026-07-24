#![allow(clippy::panic)]

use lkjscript_native::{
    encode, BackendLimits, EncodingConfig, FrameHomeKind, InternalMachineArgument,
    InternalMachineResult, MachinePlanBuilder, NativeError, PlanError, ReferenceType,
    RuntimeCallSlot, Signature, SourceFunctionId, ValueType, VerificationError,
};

#[test]
fn rejects_missing_terminator_and_uninitialized_local() -> Result<(), Box<dyn std::error::Error>> {
    let mut plan = MachinePlanBuilder::new();
    let function = plan.declare_function(
        SourceFunctionId::new(1),
        Signature::new(vec![], ValueType::I64)?,
    )?;
    let mut builder = plan.function_builder(function)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    let _value = builder.i64_const(entry, 1)?;
    plan.define_function(builder.finish())?;
    assert!(matches!(
        plan.verify(BackendLimits::default()),
        Err(NativeError::Verification(
            VerificationError::MissingTerminator(_)
        ))
    ));

    let mut plan = MachinePlanBuilder::new();
    let function = plan.declare_function(
        SourceFunctionId::new(2),
        Signature::new(vec![], ValueType::I64)?,
    )?;
    let mut builder = plan.function_builder(function)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    let local = builder.create_local(ValueType::I64)?;
    let value = builder.read_local(entry, local)?;
    builder.return_value(entry, value)?;
    plan.define_function(builder.finish())?;
    assert!(matches!(
        plan.verify(BackendLimits::default()),
        Err(NativeError::Verification(
            VerificationError::LocalNotInitialized(_)
        ))
    ));
    Ok(())
}

#[test]
fn rejects_type_mismatch_unsupported_signature_and_code_limit(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut plan = MachinePlanBuilder::new();
    let function = plan.declare_function(
        SourceFunctionId::new(3),
        Signature::new(vec![], ValueType::I64)?,
    )?;
    let mut builder = plan.function_builder(function)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    let boolean = builder.bool_const(entry, true)?;
    let integer = builder.i64_const(entry, 1)?;
    let invalid = builder.i64_add(entry, boolean, integer)?;
    builder.return_value(entry, invalid)?;
    plan.define_function(builder.finish())?;
    assert!(matches!(
        plan.verify(BackendLimits::default()),
        Err(NativeError::Verification(VerificationError::TypeMismatch(
            _
        )))
    ));

    let mut plan = MachinePlanBuilder::new();
    let function = plan.declare_function(
        SourceFunctionId::new(4),
        Signature::new(
            vec![ValueType::I64, ValueType::I64, ValueType::I64],
            ValueType::I64,
        )?,
    )?;
    let mut builder = plan.function_builder(function)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    let value = builder.parameter(0)?;
    builder.return_value(entry, value)?;
    plan.define_function(builder.finish())?;
    assert!(matches!(
        plan.verify(BackendLimits::default()),
        Err(NativeError::Verification(
            VerificationError::UnsupportedSignature(_)
        ))
    ));

    let limits = BackendLimits::new(4, 8, 64, 8, 16, 4_096, 1_000);
    let mut plan = MachinePlanBuilder::new();
    let function = plan.declare_function(
        SourceFunctionId::new(5),
        Signature::new(vec![], ValueType::I64)?,
    )?;
    let mut builder = plan.function_builder(function)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    let value = builder.i64_const(entry, 42)?;
    builder.return_value(entry, value)?;
    plan.define_function(builder.finish())?;
    let verified = plan.verify(limits)?;
    assert!(matches!(
        encode(verified, EncodingConfig::default()),
        Err(NativeError::Encode(_))
    ));
    Ok(())
}

#[test]
fn verifies_exact_reference_signatures_and_type_use() -> Result<(), Box<dyn std::error::Error>> {
    let buf = ValueType::Reference(ReferenceType::Buf);
    let string = ValueType::Reference(ReferenceType::Str);

    let mut plan = MachinePlanBuilder::new();
    let function =
        plan.declare_function(SourceFunctionId::new(10), Signature::new(vec![buf], buf)?)?;
    let mut builder = plan.function_builder(function)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    let input = builder.parameter(0)?;
    let one = builder.i64_const(entry, 1)?;
    let invalid = builder.i64_add(entry, input, one)?;
    builder.return_value(entry, invalid)?;
    plan.define_function(builder.finish())?;
    assert!(matches!(
        plan.verify(BackendLimits::default()),
        Err(NativeError::Verification(VerificationError::TypeMismatch(
            _
        )))
    ));

    let mut plan = MachinePlanBuilder::new();
    let function = plan.declare_function(
        SourceFunctionId::new(11),
        Signature::new(vec![string], buf)?,
    )?;
    let mut builder = plan.function_builder(function)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    let input = builder.parameter(0)?;
    builder.return_value(entry, input)?;
    plan.define_function(builder.finish())?;
    assert!(matches!(
        plan.verify(BackendLimits::default()),
        Err(NativeError::Verification(VerificationError::InvalidReturn(
            _
        )))
    ));

    let mut plan = MachinePlanBuilder::new();
    let function = plan.declare_function(
        SourceFunctionId::new(12),
        Signature::new(vec![ValueType::I64], buf)?,
    )?;
    let mut builder = plan.function_builder(function)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    let integer = builder.parameter(0)?;
    let invalid =
        builder.runtime_call(entry, RuntimeCallSlot::CollectReferenceV1, vec![integer])?;
    builder.return_value(entry, invalid)?;
    plan.define_function(builder.finish())?;
    assert!(matches!(
        plan.verify(BackendLimits::default()),
        Err(NativeError::Verification(VerificationError::TypeMismatch(
            _
        )))
    ));
    Ok(())
}

#[test]
fn derives_non_empty_exact_reference_maps_without_dead_slots(
) -> Result<(), Box<dyn std::error::Error>> {
    let buf = ValueType::Reference(ReferenceType::Buf);
    let mut plan = MachinePlanBuilder::new();
    let function = plan.declare_function(
        SourceFunctionId::new(20),
        Signature::new(vec![buf, buf], buf)?,
    )?;
    let mut builder = plan.function_builder(function)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    let live = builder.parameter(0)?;
    let _dead = builder.parameter(1)?;
    let local = builder.create_local(buf)?;
    let _write = builder.write_local(entry, local, live)?;
    let _collected =
        builder.runtime_call(entry, RuntimeCallSlot::CollectReferenceV1, vec![live])?;
    let returned = builder.read_local(entry, local)?;
    builder.return_value(entry, returned)?;
    plan.define_function(builder.finish())?;
    let image = encode(
        plan.verify(BackendLimits::default())?,
        EncodingConfig::default(),
    )?;
    image.validate_integrity()?;
    assert_eq!(image.safepoints().len(), 1);
    let roots = image.safepoints()[0].stack_map().roots();
    assert_eq!(roots.len(), 2);
    assert!(roots
        .iter()
        .all(|root| root.reference_type() == ReferenceType::Buf));
    assert!(roots
        .iter()
        .any(|root| root.kind() == FrameHomeKind::Value(0)));
    assert!(roots
        .iter()
        .any(|root| root.kind() == FrameHomeKind::Local(0)));
    assert!(!roots
        .iter()
        .any(|root| root.kind() == FrameHomeKind::Value(1)));
    assert!(roots.windows(2).all(|pair| pair[0] < pair[1]));
    Ok(())
}

#[test]
fn rejects_adversarial_wide_root_certificates_before_metadata_allocation(
) -> Result<(), Box<dyn std::error::Error>> {
    let buf = ValueType::Reference(ReferenceType::Buf);
    let mut plan = MachinePlanBuilder::new();
    let sink = plan.declare_function(
        SourceFunctionId::new(30),
        Signature::new(vec![buf, buf], ValueType::Unit)?,
    )?;
    let wide = plan.declare_function(SourceFunctionId::new(31), Signature::new(vec![buf], buf)?)?;

    let mut sink_builder = plan.function_builder(sink)?;
    let sink_entry = sink_builder.create_block()?;
    sink_builder.set_entry(sink_entry)?;
    let unit = sink_builder.unit(sink_entry)?;
    sink_builder.return_value(sink_entry, unit)?;
    plan.define_function(sink_builder.finish())?;

    let mut builder = plan.function_builder(wide)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    let input = builder.parameter(0)?;
    let mut locals = Vec::new();
    for _ in 0..128 {
        let local = builder.create_local(buf)?;
        let _write = builder.write_local(entry, local, input)?;
        locals.push(local);
    }
    let collected =
        builder.runtime_call(entry, RuntimeCallSlot::CollectReferenceV1, vec![input])?;
    for pair in locals.chunks_exact(2) {
        let first = builder.read_local(entry, pair[0])?;
        let second = builder.read_local(entry, pair[1])?;
        let _call = builder.call(entry, sink, vec![first, second])?;
    }
    builder.return_value(entry, collected)?;
    plan.define_function(builder.finish())?;

    let limits = BackendLimits::new(2, 2, 512, 128, 1024 * 1024, 1024, 1_000_000);
    assert!(matches!(
        plan.verify(limits),
        Err(NativeError::Verification(VerificationError::LimitExceeded(
            "stack-map root metadata"
        )))
    ));
    Ok(())
}

#[test]
fn internal_runtime_abi_describes_encoder_owned_arguments_exactly(
) -> Result<(), Box<dyn std::error::Error>> {
    let missing = |message| std::io::Error::other(message);
    let reserve = RuntimeCallSlot::ReserveFrameV1
        .internal_abi_signature()
        .ok_or_else(|| missing("missing reserve ABI"))?;
    assert_eq!(
        reserve.parameters(),
        &[
            InternalMachineArgument::InvocationContext,
            InternalMachineArgument::FunctionOrdinal,
            InternalMachineArgument::FrameBytes,
            InternalMachineArgument::FramePointer,
        ]
    );
    assert_eq!(reserve.result(), InternalMachineResult::InvocationContext);
    assert!(RuntimeCallSlot::ReserveFrameV1.plan_signature().is_none());
    assert_eq!(
        RuntimeCallSlot::RegisterFrameV1
            .internal_abi_signature()
            .ok_or_else(|| missing("missing register ABI"))?
            .parameters(),
        &[
            InternalMachineArgument::InvocationContext,
            InternalMachineArgument::FunctionOrdinal,
            InternalMachineArgument::FramePointer,
        ]
    );
    assert_eq!(
        RuntimeCallSlot::PublishSafepointV1
            .internal_abi_signature()
            .ok_or_else(|| missing("missing publish ABI"))?
            .parameters(),
        &[
            InternalMachineArgument::InvocationContext,
            InternalMachineArgument::SafepointId,
        ]
    );
    assert_eq!(
        RuntimeCallSlot::UnregisterFrameV1
            .internal_abi_signature()
            .ok_or_else(|| missing("missing unregister ABI"))?
            .parameters(),
        RuntimeCallSlot::RegisterFrameV1
            .internal_abi_signature()
            .ok_or_else(|| missing("missing register ABI"))?
            .parameters()
    );

    let mut plan = MachinePlanBuilder::new();
    let function = plan.declare_function(
        SourceFunctionId::new(32),
        Signature::new(Vec::new(), ValueType::Unit)?,
    )?;
    let mut builder = plan.function_builder(function)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    assert_eq!(
        builder.runtime_call(entry, RuntimeCallSlot::RegisterFrameV1, Vec::new()),
        Err(PlanError::EncoderOwnedRuntimeCall)
    );
    Ok(())
}
