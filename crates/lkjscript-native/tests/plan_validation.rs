#![allow(clippy::panic)]

use lkjscript_native::{
    encode, BackendLimits, EncodingConfig, FrameHomeKind, MachinePlanBuilder, NativeError,
    ReferenceType, RuntimeCallSlot, Signature, SourceFunctionId, ValueType, VerificationError,
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
