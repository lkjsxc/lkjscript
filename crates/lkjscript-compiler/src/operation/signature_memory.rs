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
        Operation::ByteVectorNew => function(vec![Type::I64], Type::ByteVector),
        Operation::ByteSliceLength => function(vec![Type::ByteSlice], Type::I64),
        Operation::ByteSliceByteAt => function(vec![Type::ByteSlice, Type::I64], Type::I64),
        Operation::ByteSliceMutSetByte => {
            function(vec![Type::ByteSliceMut, Type::I64, Type::I64], Type::Unit)
        }
        Operation::ConvertStringToBytes => function(vec![Type::Str], Type::Bytes),
        Operation::ConvertBytesToString => function(
            vec![Type::Bytes],
            crate::types::result_type(Type::Str, crate::types::utf8_error_type()),
        ),
        Operation::PathFromStr => function(vec![Type::Str], system_result(Type::Path)),
        Operation::PathFromBytes => function(vec![Type::Bytes], system_result(Type::Path)),
        Operation::PathToBytes => function(vec![Type::Path], Type::Bytes),
        Operation::PathToStr => function(
            vec![Type::Path],
            crate::types::result_type(Type::Str, crate::types::utf8_error_type()),
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
