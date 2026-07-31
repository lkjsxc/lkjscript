use super::*;

#[test]
fn structural_dispatch_internal_abi_has_exact_direct_words(
) -> Result<(), Box<dyn std::error::Error>> {
    let signature = RuntimeCallSlot::StructuralDispatch
        .internal_abi_signature()
        .ok_or_else(|| std::io::Error::other("structural dispatch ABI"))?;
    assert_eq!(
        signature.parameters(),
        &[
            InternalMachineArgument::InvocationContext,
            InternalMachineArgument::StructuralSiteId,
            InternalMachineArgument::StructuralArgument0,
            InternalMachineArgument::StructuralArgument1,
            InternalMachineArgument::StructuralArgument2,
        ]
    );
    assert_eq!(signature.result(), InternalMachineResult::Integer);
    assert!(RuntimeCallSlot::StructuralDispatch
        .plan_signature()
        .is_none());
    Ok(())
}
