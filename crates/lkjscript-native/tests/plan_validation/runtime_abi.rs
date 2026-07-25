use super::*;

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
        RuntimeCallSlot::HeapDispatchV1
            .internal_abi_signature()
            .ok_or_else(|| missing("missing heap dispatch ABI"))?
            .parameters(),
        &[
            InternalMachineArgument::InvocationContext,
            InternalMachineArgument::HeapSiteId,
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
