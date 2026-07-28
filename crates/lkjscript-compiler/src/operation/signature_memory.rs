use crate::operation::instantiation::function;
use crate::operation::*;

pub(in crate::operation) fn memory_signature(operation: Operation) -> Type {
    let system_result =
        |success| crate::types::result_type(success, crate::types::system_error_type());
    match operation {
        Operation::BytesLength => function(vec![Type::Bytes], Type::I64),
        Operation::BytesByteAt => function(vec![Type::Bytes, Type::I64], Type::I64),
        Operation::CopyBytesSlice => function(vec![Type::Bytes, Type::I64, Type::I64], Type::Bytes),
        Operation::CloneBytes => function(vec![Type::Bytes], Type::Bytes),
        Operation::FreezeByteVector => function(vec![Type::ByteVector], Type::Bytes),
        Operation::ThawBytes => function(vec![Type::Bytes], Type::ByteVector),
        Operation::ByteSliceReadU32LittleEndian => {
            function(vec![Type::ByteSlice, Type::I64], Type::I64)
        }
        Operation::ByteSliceMutWriteU32LittleEndian => {
            function(vec![Type::ByteSliceMut, Type::I64, Type::I64], Type::Unit)
        }
        Operation::BufNew => function(vec![Type::I64], Type::Buf),
        Operation::OwnedBufNew => function(vec![Type::I64], Type::ByteVector),
        Operation::OwnedBufLen => function(vec![Type::ByteSlice], Type::I64),
        Operation::OwnedBufRef => function(vec![Type::ByteSlice, Type::I64], Type::I64),
        Operation::OwnedBufSet => {
            function(vec![Type::ByteSliceMut, Type::I64, Type::I64], Type::Unit)
        }
        Operation::BufLen => function(vec![Type::Buf], Type::I64),
        Operation::BufRef | Operation::BufGetU32 => function(vec![Type::Buf, Type::I64], Type::I64),
        Operation::BufSet | Operation::BufSetU32 => {
            function(vec![Type::Buf, Type::I64, Type::I64], Type::Unit)
        }
        Operation::BufClone => function(vec![Type::Buf], Type::Buf),
        Operation::BufFromStr => function(vec![Type::Str], Type::Buf),
        Operation::BufToStr => function(
            vec![Type::Buf],
            crate::types::result_type(Type::Str, crate::types::utf8_error_type()),
        ),
        Operation::PathFromStr => function(vec![Type::Str], system_result(Type::Path)),
        Operation::PathFromBuf => function(vec![Type::Buf], system_result(Type::Path)),
        Operation::PathToBuf => function(vec![Type::Path], Type::Buf),
        Operation::PathToStr => function(
            vec![Type::Path],
            crate::types::result_type(Type::Str, crate::types::utf8_error_type()),
        ),
        Operation::BufSlice => function(
            vec![Type::Buf, Type::I64, Type::I64],
            system_result(Type::Buf),
        ),
        Operation::StrLen => function(vec![Type::Str], Type::I64),
        Operation::StrRef => function(vec![Type::Str, Type::I64], Type::I64),
        Operation::StrAppend => function(vec![Type::Str, Type::Str], Type::Str),
        Operation::StrSlice => function(vec![Type::Str, Type::I64, Type::I64], Type::Str),
        Operation::StrFromByte | Operation::StrFromI64 => function(vec![Type::I64], Type::Str),
        Operation::StrFromF64 => function(vec![Type::F64], Type::Str),
        _ => unreachable!("operation signature family mismatch"),
    }
}
