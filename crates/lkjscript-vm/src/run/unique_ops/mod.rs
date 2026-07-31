use super::*;

#[path = "bytes.rs"]
mod bytes_ops;
#[path = "support.rs"]
mod support;
mod word;
use support::*;

pub(super) fn handles(op: u8) -> bool {
    bytes_ops::handles(op)
        || word::handles(op)
        || matches!(
            Op::from_byte(op),
            Some(
                Op::ByteVectorNew
                    | Op::ByteVectorPlaceInit
                    | Op::ByteVectorMove
                    | Op::ByteVectorBorrow
                    | Op::ByteVectorBorrowMut
                    | Op::StoreUniqueLocal
                    | Op::StoreViewLocal
                    | Op::TakeUniqueLocal
                    | Op::LoadViewLocal
                    | Op::ByteVectorDropPlace
                    | Op::ByteVectorPlaceEnd
                    | Op::ByteSliceLen
                    | Op::ByteSliceRef
                    | Op::ByteSliceMutSet
                    | Op::EndBorrowLocal
            )
        )
}

include!("dispatch.rs");
