use super::*;

pub(super) fn plan_signature(slot: RuntimeCallSlot) -> Option<Signature> {
    let unique = ValueType::Unique(UniqueType::ByteVector);
    let shared = ValueType::Loan(LoanType::ByteSlice);
    let exclusive = ValueType::Loan(LoanType::ByteSliceMut);
    match slot {
        RuntimeCallSlot::IdentityI64 => signature(vec![ValueType::I64], ValueType::I64),
        RuntimeCallSlot::Poll => signature(Vec::new(), ValueType::Unit),
        RuntimeCallSlot::EnterFunction => signature(vec![ValueType::I64], ValueType::Unit),
        RuntimeCallSlot::StdinHandle => signature(
            vec![ValueType::Capability(
                lkjscript_contracts::CapabilityKind::Stdio,
            )],
            ValueType::Resource(lkjscript_contracts::ResourceKind::InputStream),
        ),
        RuntimeCallSlot::ByteVectorNew => signature(vec![ValueType::I64], unique),
        RuntimeCallSlot::ByteVectorMove => signature(vec![unique], unique),
        RuntimeCallSlot::ByteVectorBorrowShared => signature(vec![unique], shared),
        RuntimeCallSlot::ByteVectorBorrowExclusive => signature(vec![unique], exclusive),
        RuntimeCallSlot::ByteSliceLength => signature(vec![shared], ValueType::I64),
        RuntimeCallSlot::ByteSliceByteAt => signature(vec![shared, ValueType::I64], ValueType::I64),
        RuntimeCallSlot::ByteSliceMutSetByte => signature(
            vec![exclusive, ValueType::I64, ValueType::I64],
            ValueType::Unit,
        ),
        RuntimeCallSlot::ByteSliceEnd => signature(vec![shared], ValueType::Unit),
        RuntimeCallSlot::ByteSliceMutEnd => signature(vec![exclusive], ValueType::Unit),
        RuntimeCallSlot::ByteVectorDrop => signature(vec![unique], ValueType::Unit),
        RuntimeCallSlot::CollectReference => signature(
            vec![ValueType::Reference(ReferenceType::Buf)],
            ValueType::Reference(ReferenceType::Buf),
        ),
        RuntimeCallSlot::HeapDispatch
        | RuntimeCallSlot::ReserveFrame
        | RuntimeCallSlot::RegisterFrame
        | RuntimeCallSlot::PublishSafepoint
        | RuntimeCallSlot::UnregisterFrame => None,
    }
}

fn signature(parameters: Vec<ValueType>, result: ValueType) -> Option<Signature> {
    Some(Signature { parameters, result })
}
