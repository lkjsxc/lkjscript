use super::*;

#[test]
fn internal_runtime_abi_describes_encoder_owned_arguments_exactly(
) -> Result<(), Box<dyn std::error::Error>> {
    let missing = |message| std::io::Error::other(message);
    let reserve = RuntimeCallSlot::ReserveFrame
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
    assert!(RuntimeCallSlot::ReserveFrame.plan_signature().is_none());
    let rejected = RuntimeCallSlot::TakeRejectedEntry
        .internal_abi_signature()
        .ok_or_else(|| missing("missing rejected-entry ABI"))?;
    assert_eq!(
        rejected.parameters(),
        &[InternalMachineArgument::InvocationContext]
    );
    assert_eq!(rejected.result(), InternalMachineResult::Integer);
    assert!(RuntimeCallSlot::TakeRejectedEntry
        .plan_signature()
        .is_none());
    assert_eq!(
        RuntimeCallSlot::RegisterFrame
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
        RuntimeCallSlot::PublishSafepoint
            .internal_abi_signature()
            .ok_or_else(|| missing("missing publish ABI"))?
            .parameters(),
        &[
            InternalMachineArgument::InvocationContext,
            InternalMachineArgument::SafepointId,
        ]
    );
    assert_eq!(
        RuntimeCallSlot::HeapDispatch
            .internal_abi_signature()
            .ok_or_else(|| missing("missing heap dispatch ABI"))?
            .parameters(),
        &[
            InternalMachineArgument::InvocationContext,
            InternalMachineArgument::HeapSiteId,
        ]
    );
    assert_eq!(
        RuntimeCallSlot::UnregisterFrame
            .internal_abi_signature()
            .ok_or_else(|| missing("missing unregister ABI"))?
            .parameters(),
        RuntimeCallSlot::RegisterFrame
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
        builder.runtime_call(entry, RuntimeCallSlot::RegisterFrame, Vec::new()),
        Err(PlanError::EncoderOwnedRuntimeCall)
    );
    Ok(())
}

#[test]
fn failure_cleanup_calls_are_typed_and_independently_verified(
) -> Result<(), Box<dyn std::error::Error>> {
    let valid = failure_cleanup_plan(RuntimeCallSlot::ByteVectorDrop)?;
    valid.verify(BackendLimits::default())?;

    let wrong = failure_cleanup_plan(RuntimeCallSlot::ByteSliceEnd)?;
    assert!(matches!(
        wrong.verify(BackendLimits::default()),
        Err(NativeError::Verification(VerificationError::TypeMismatch(
            "failure cleanup runtime call"
        )))
    ));
    Ok(())
}

fn failure_cleanup_plan(
    slot: RuntimeCallSlot,
) -> Result<MachinePlanBuilder, Box<dyn std::error::Error>> {
    let mut plan = MachinePlanBuilder::new();
    let unique = ValueType::Unique(UniqueType::ByteVector);
    let function = plan.declare_function(
        SourceFunctionId::new(39),
        Signature::new(vec![unique], ValueType::Unit)?,
    )?;
    let mut builder = plan.function_builder(function)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    let owner = builder.parameter(0)?;
    let local = builder.create_local(unique)?;
    builder.write_local(entry, local, owner)?;
    let moved = builder.runtime_call(entry, RuntimeCallSlot::ByteVectorMove, vec![owner])?;
    builder.set_instruction_failure_cleanup(moved, vec![FailureCleanupCall::new(slot, local)])?;
    let unit = builder.unit(entry)?;
    builder.return_value(entry, unit)?;
    plan.define_function(builder.finish())?;
    Ok(plan)
}

#[test]
fn typed_resource_runtime_abi_rejects_wrong_capability_and_resource_kinds(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut capability_plan = MachinePlanBuilder::new();
    let function = capability_plan.declare_function(
        SourceFunctionId::new(40),
        Signature::new(
            vec![ValueType::Capability(CapabilityKind::FileSystem)],
            ValueType::Resource(ResourceKind::InputStream),
        )?,
    )?;
    let mut builder = capability_plan.function_builder(function)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    let capability = builder.parameter(0)?;
    let input = builder.runtime_call(entry, RuntimeCallSlot::StdinHandle, vec![capability])?;
    builder.return_value(entry, input)?;
    capability_plan.define_function(builder.finish())?;
    assert!(matches!(
        capability_plan.verify(BackendLimits::default()),
        Err(NativeError::Verification(VerificationError::TypeMismatch(
            "runtime call"
        )))
    ));

    let mut resource_plan = MachinePlanBuilder::new();
    let callee = resource_plan.declare_function(
        SourceFunctionId::new(41),
        Signature::new(
            vec![ValueType::Resource(ResourceKind::FileReader)],
            ValueType::Unit,
        )?,
    )?;
    let root = resource_plan.declare_function(
        SourceFunctionId::new(42),
        Signature::new(
            vec![ValueType::Resource(ResourceKind::InputStream)],
            ValueType::Unit,
        )?,
    )?;
    let mut callee_builder = resource_plan.function_builder(callee)?;
    let callee_entry = callee_builder.create_block()?;
    callee_builder.set_entry(callee_entry)?;
    let unit = callee_builder.unit(callee_entry)?;
    callee_builder.return_value(callee_entry, unit)?;
    resource_plan.define_function(callee_builder.finish())?;
    let mut root_builder = resource_plan.function_builder(root)?;
    let root_entry = root_builder.create_block()?;
    root_builder.set_entry(root_entry)?;
    let input = root_builder.parameter(0)?;
    let output = root_builder.call(root_entry, callee, vec![input])?;
    root_builder.return_value(root_entry, output)?;
    resource_plan.define_function(root_builder.finish())?;
    assert!(matches!(
        resource_plan.verify(BackendLimits::default()),
        Err(NativeError::Verification(VerificationError::TypeMismatch(
            "compiled call"
        )))
    ));
    Ok(())
}
