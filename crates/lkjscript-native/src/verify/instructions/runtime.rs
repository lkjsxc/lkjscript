use super::*;

pub(super) fn verify_runtime_slot(slot: RuntimeCallSlot) -> Result<(), VerificationError> {
    if !slot.plan_callable() {
        return Err(VerificationError::TypeMismatch(
            "encoder-owned runtime call",
        ));
    }
    match slot {
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
        | RuntimeCallSlot::UnregisterFrame => {
            return Err(VerificationError::TypeMismatch(
                "encoder-owned runtime call",
            ));
        }
    }
    Ok(())
}
