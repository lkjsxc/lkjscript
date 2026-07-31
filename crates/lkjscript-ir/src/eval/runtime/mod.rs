use super::*;

mod byte_data;
mod bytes;
mod paths;
mod resources;
mod scalars;
mod sequences;
mod strings;
mod unsupported;

impl Evaluator<'_> {
    pub(crate) fn runtime(
        &mut self,
        operation: RuntimeOp,
        arguments: Vec<&EvalValue>,
        result_type: &crate::SsaType,
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
            | Op::Arg => self.runtime_sequences(operation, arguments, result_type),
            Op::ByteVectorNew
            | Op::ByteSliceLength
            | Op::ByteSliceByteAt
            | Op::ByteSliceMutSetByte
            | Op::ByteSliceReadU32Le
            | Op::ByteSliceMutWriteU32Le
            | Op::ConvertStringToBytes
            | Op::ConvertBytesToString => self.runtime_byte_data(operation, arguments, result_type),
            Op::BytesLength
            | Op::BytesByteAt
            | Op::CopyBytesSlice
            | Op::CloneBytes
            | Op::FreezeByteVector
            | Op::ThawBytes => self.runtime_bytes(operation, arguments),
            Op::PathFromStr | Op::PathFromBytes | Op::PathToBytes | Op::PathToStr => {
                self.runtime_paths(operation, arguments, result_type)
            }
            Op::StdinHandle
            | Op::SysIsatty
            | Op::SysClose
            | Op::SysOpenRead
            | Op::SysOpenWrite
            | Op::SysOpenAppend
            | Op::SysOpenCreateNew
            | Op::SysOpenDir
            | Op::SysSqliteOpen
            | Op::SysSqliteClose
            | Op::SysSqlitePrepare
            | Op::SysSqliteFinalize => self.runtime_resources(operation, arguments, result_type),
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
