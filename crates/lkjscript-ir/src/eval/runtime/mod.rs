use super::*;

mod buffers;
mod scalars;
mod sequences;
mod strings;
mod unsupported;

impl Evaluator<'_> {
    pub(crate) fn runtime(
        &mut self,
        operation: RuntimeOp,
        arguments: Vec<EvalValue>,
    ) -> std::result::Result<EvalValue, Flow> {
        use RuntimeOp as Op;
        match operation {
            Op::Add
            | Op::Subtract
            | Op::Multiply
            | Op::Divide
            | Op::EqualValue
            | Op::SameObject
            | Op::ListEqual
            | Op::F64BitsEqual
            | Op::Less
            | Op::LessEqual
            | Op::Greater
            | Op::GreaterEqual
            | Op::Not
            | Op::BitAnd
            | Op::BitOr
            | Op::BitXor => self.runtime_scalars(operation, arguments),
            Op::Cons
            | Op::Car
            | Op::Cdr
            | Op::IsEmptyList
            | Op::EmptyStr
            | Op::ArgCount
            | Op::Arg => self.runtime_sequences(operation, arguments),
            Op::BufNew
            | Op::OwnedBufNew
            | Op::BufLen
            | Op::OwnedBufLen
            | Op::BufRef
            | Op::OwnedBufRef
            | Op::BufSet
            | Op::OwnedBufSet
            | Op::BufClone
            | Op::BufFromStr
            | Op::BufToStr
            | Op::BufSlice
            | Op::BufGetU32
            | Op::BufSetU32 => self.runtime_buffers(operation, arguments),
            Op::StrLen
            | Op::StrRef
            | Op::StrAppend
            | Op::StrSlice
            | Op::StrFromByte
            | Op::StrFromI64
            | Op::StrFromF64 => self.runtime_strings(operation, arguments),
            _ => self.runtime_unsupported(operation, arguments),
        }
    }
}
