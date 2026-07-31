use super::*;

pub(super) fn verify_runtime_slot(slot: RuntimeCallSlot) -> Result<(), VerificationError> {
    if !slot.plan_callable() {
        return Err(VerificationError::TypeMismatch(
            "encoder-owned runtime call",
        ));
    }
    let signature = slot
        .plan_signature()
        .ok_or(VerificationError::TypeMismatch(
            "encoder-owned runtime call",
        ))?;
    match slot {
        RuntimeCallSlot::CollectReference
            if signature.parameters()
                == [ValueType::Reference(crate::ReferenceType::Product(
                    crate::LayoutIdentity::product(0),
                ))]
                && signature.result()
                    == ValueType::Reference(crate::ReferenceType::Product(
                        crate::LayoutIdentity::product(0),
                    )) => {}
        RuntimeCallSlot::CollectReference => {
            return Err(VerificationError::TypeMismatch(
                "collecting runtime-call signature",
            ));
        }
        RuntimeCallSlot::IdentityI64
        | RuntimeCallSlot::Poll
        | RuntimeCallSlot::EnterFunction
        | RuntimeCallSlot::StdinHandle
        | RuntimeCallSlot::ByteVectorNew
        | RuntimeCallSlot::ByteVectorMove
        | RuntimeCallSlot::ByteVectorBorrowShared
        | RuntimeCallSlot::ByteVectorBorrowExclusive
        | RuntimeCallSlot::ByteSliceLength
        | RuntimeCallSlot::ByteSliceByteAt
        | RuntimeCallSlot::ByteSliceReadU32Le
        | RuntimeCallSlot::ByteSliceMutSetByte
        | RuntimeCallSlot::ByteSliceMutWriteU32Le
        | RuntimeCallSlot::ByteSliceEnd
        | RuntimeCallSlot::ByteSliceMutEnd
        | RuntimeCallSlot::ByteVectorDrop
        | RuntimeCallSlot::StaticBytesLength
        | RuntimeCallSlot::StaticBytesByteAt
        | RuntimeCallSlot::StaticBytesClone
        | RuntimeCallSlot::StaticBytesCopySlice
        | RuntimeCallSlot::StaticBytesThaw
        | RuntimeCallSlot::BytesMove
        | RuntimeCallSlot::BytesBorrowShared
        | RuntimeCallSlot::BytesLength
        | RuntimeCallSlot::BytesByteAt
        | RuntimeCallSlot::BytesClone
        | RuntimeCallSlot::BytesCopySlice
        | RuntimeCallSlot::BytesEndBorrow
        | RuntimeCallSlot::BytesDrop
        | RuntimeCallSlot::FreezeByteVector
        | RuntimeCallSlot::ThawBytes => {}
        RuntimeCallSlot::HeapDispatch
        | RuntimeCallSlot::StructuralDispatch
        | RuntimeCallSlot::TakeRejectedEntry
        | RuntimeCallSlot::ReserveFrame
        | RuntimeCallSlot::RegisterFrame
        | RuntimeCallSlot::PublishSafepoint
        | RuntimeCallSlot::UnregisterFrame => {
            return Err(VerificationError::TypeMismatch(
                "encoder-owned runtime call",
            ));
        }
    }
    Ok(())
}
