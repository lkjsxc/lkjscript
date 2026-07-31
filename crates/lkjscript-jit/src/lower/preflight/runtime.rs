use super::*;

pub(super) fn supported_runtime(operation: RuntimeOp, domain: LoweringDomain) -> bool {
    if domain == LoweringDomain::StructuralIsland
        && matches!(
            operation,
            RuntimeOp::EmptyStr | RuntimeOp::StrFromI64 | RuntimeOp::StrLen
        )
    {
        return true;
    }
    if domain == LoweringDomain::ResourceIsland && operation == RuntimeOp::StdinHandle {
        return true;
    }
    if domain == LoweringDomain::UniqueIsland
        && matches!(
            operation,
            RuntimeOp::ByteVectorNew
                | RuntimeOp::ByteSliceLength
                | RuntimeOp::ByteSliceByteAt
                | RuntimeOp::ByteSliceMutSetByte
                | RuntimeOp::ByteSliceReadU32Le
                | RuntimeOp::ByteSliceMutWriteU32Le
                | RuntimeOp::BytesLength
                | RuntimeOp::BytesByteAt
                | RuntimeOp::CopyBytesSlice
                | RuntimeOp::CloneBytes
                | RuntimeOp::FreezeByteVector
                | RuntimeOp::ThawBytes
        )
    {
        return true;
    }
    matches!(
        operation,
        RuntimeOp::Add
            | RuntimeOp::Subtract
            | RuntimeOp::Multiply
            | RuntimeOp::Divide
            | RuntimeOp::EqualValue
            | RuntimeOp::F64BitsEqual
            | RuntimeOp::Less
            | RuntimeOp::LessEqual
            | RuntimeOp::Greater
            | RuntimeOp::GreaterEqual
            | RuntimeOp::Not
            | RuntimeOp::BitAnd
            | RuntimeOp::BitOr
            | RuntimeOp::BitXor
            | RuntimeOp::ListEqual
            | RuntimeOp::Cons
            | RuntimeOp::Car
            | RuntimeOp::Cdr
            | RuntimeOp::IsEmptyList
    )
}
