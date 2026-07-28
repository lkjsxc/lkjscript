use super::*;

pub(super) fn supported_runtime(operation: RuntimeOp, domain: LoweringDomain) -> bool {
    if domain == LoweringDomain::ResourceIsland && operation == RuntimeOp::StdinHandle {
        return true;
    }
    if domain == LoweringDomain::UniqueIsland
        && matches!(
            operation,
            RuntimeOp::OwnedBufNew
                | RuntimeOp::OwnedBufLen
                | RuntimeOp::OwnedBufRef
                | RuntimeOp::OwnedBufSet
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
            | RuntimeOp::SameObject
            | RuntimeOp::ListEqual
            | RuntimeOp::Cons
            | RuntimeOp::Car
            | RuntimeOp::Cdr
            | RuntimeOp::IsEmptyList
            | RuntimeOp::EmptyStr
            | RuntimeOp::BufNew
            | RuntimeOp::BufLen
            | RuntimeOp::BufRef
            | RuntimeOp::BufSet
            | RuntimeOp::BufClone
            | RuntimeOp::BufFromStr
            | RuntimeOp::BufToStr
            | RuntimeOp::BufSlice
            | RuntimeOp::BufGetU32
            | RuntimeOp::BufSetU32
            | RuntimeOp::StrLen
            | RuntimeOp::StrRef
            | RuntimeOp::StrAppend
            | RuntimeOp::StrSlice
            | RuntimeOp::StrFromByte
            | RuntimeOp::StrFromI64
            | RuntimeOp::StrFromF64
    )
}
