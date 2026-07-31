use super::*;

pub(super) fn plan_signature(slot: RuntimeCallSlot) -> Option<Signature> {
    let unique = ValueType::Unique(UniqueType::ByteVector);
    let bytes = ValueType::Unique(UniqueType::Bytes);
    let bytes_loan = ValueType::Loan(LoanType::Bytes);
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
        RuntimeCallSlot::ByteSliceByteAt | RuntimeCallSlot::ByteSliceReadU32Le => {
            signature(vec![shared, ValueType::I64], ValueType::I64)
        }
        RuntimeCallSlot::ByteSliceMutSetByte | RuntimeCallSlot::ByteSliceMutWriteU32Le => {
            signature(
                vec![exclusive, ValueType::I64, ValueType::I64],
                ValueType::Unit,
            )
        }
        RuntimeCallSlot::ByteSliceEnd => signature(vec![shared], ValueType::Unit),
        RuntimeCallSlot::ByteSliceMutEnd => signature(vec![exclusive], ValueType::Unit),
        RuntimeCallSlot::ByteVectorDrop => signature(vec![unique], ValueType::Unit),
        RuntimeCallSlot::StaticBytesLength => {
            signature(vec![ValueType::StaticBytes], ValueType::I64)
        }
        RuntimeCallSlot::StaticBytesByteAt => {
            signature(vec![ValueType::StaticBytes, ValueType::I64], ValueType::I64)
        }
        RuntimeCallSlot::StaticBytesClone => signature(vec![ValueType::StaticBytes], bytes),
        RuntimeCallSlot::StaticBytesCopySlice => signature(
            vec![ValueType::StaticBytes, ValueType::I64, ValueType::I64],
            bytes,
        ),
        RuntimeCallSlot::StaticBytesThaw => signature(vec![ValueType::StaticBytes], unique),
        RuntimeCallSlot::BytesMove => signature(vec![bytes], bytes),
        RuntimeCallSlot::BytesBorrowShared => signature(vec![bytes], bytes_loan),
        RuntimeCallSlot::BytesLength => signature(vec![bytes_loan], ValueType::I64),
        RuntimeCallSlot::BytesByteAt => signature(vec![bytes_loan, ValueType::I64], ValueType::I64),
        RuntimeCallSlot::BytesClone => signature(vec![bytes_loan], bytes),
        RuntimeCallSlot::BytesCopySlice => {
            signature(vec![bytes_loan, ValueType::I64, ValueType::I64], bytes)
        }
        RuntimeCallSlot::BytesEndBorrow => signature(vec![bytes_loan], ValueType::Unit),
        RuntimeCallSlot::BytesDrop => signature(vec![bytes], ValueType::Unit),
        RuntimeCallSlot::FreezeByteVector => signature(vec![unique], bytes),
        RuntimeCallSlot::ThawBytes => signature(vec![bytes], unique),
        RuntimeCallSlot::HeapDispatch
        | RuntimeCallSlot::StructuralDispatch
        | RuntimeCallSlot::TakeRejectedEntry
        | RuntimeCallSlot::ReserveFrame
        | RuntimeCallSlot::RegisterFrame
        | RuntimeCallSlot::UnregisterFrame => None,
    }
}

fn signature(parameters: Vec<ValueType>, result: ValueType) -> Option<Signature> {
    Some(Signature { parameters, result })
}
