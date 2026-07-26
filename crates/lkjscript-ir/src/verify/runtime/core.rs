use crate::verify::*;
use crate::{RuntimeOp, SsaType};

pub(super) fn core_signature(
    operation: RuntimeOp,
    parameters: &[SsaType],
    result: &SsaType,
) -> Option<bool> {
    let exact = |expected: &[SsaType], result_type: &SsaType| {
        parameters == expected && result == result_type
    };
    let valid = match operation {
        RuntimeOp::Add | RuntimeOp::Subtract | RuntimeOp::Multiply | RuntimeOp::Divide => {
            parameters.len() == 2
                && parameters.iter().all(is_numeric)
                && result
                    == if parameters.iter().any(|ty| ty == &SsaType::F64) {
                        &SsaType::F64
                    } else {
                        &SsaType::I64
                    }
        }
        RuntimeOp::EqualValue => {
            parameters.len() == 2
                && parameters[0] == parameters[1]
                && supports_value_equality(&parameters[0])
                && result == &SsaType::Bool
        }
        RuntimeOp::SameObject => {
            parameters.len() == 2
                && parameters[0] == parameters[1]
                && matches!(parameters[0], SsaType::Buf | SsaType::Handle)
                && result == &SsaType::Bool
        }
        RuntimeOp::ListEqual => {
            parameters.len() == 2
                && parameters[0] == parameters[1]
                && matches!(&parameters[0], SsaType::List(item) if supports_value_equality(item))
                && result == &SsaType::Bool
        }
        RuntimeOp::F64BitsEqual => exact(&[SsaType::F64, SsaType::F64], &SsaType::Bool),
        RuntimeOp::Less | RuntimeOp::LessEqual | RuntimeOp::Greater | RuntimeOp::GreaterEqual => {
            parameters.len() == 2 && parameters.iter().all(is_numeric) && result == &SsaType::Bool
        }
        RuntimeOp::Not => exact(&[SsaType::Bool], &SsaType::Bool),
        RuntimeOp::BitAnd | RuntimeOp::BitOr | RuntimeOp::BitXor => {
            exact(&[SsaType::I64, SsaType::I64], &SsaType::I64)
        }
        RuntimeOp::Cons => {
            matches!(parameters, [item, SsaType::List(tail)] if item == tail.as_ref())
                && result == &parameters[1]
        }
        RuntimeOp::Car => {
            matches!(parameters, [SsaType::List(item)] if item.as_ref() == result)
        }
        RuntimeOp::Cdr => matches!(parameters, [SsaType::List(_)]) && result == &parameters[0],
        RuntimeOp::IsEmptyList => {
            matches!(parameters, [SsaType::List(_)]) && result == &SsaType::Bool
        }
        RuntimeOp::Print | RuntimeOp::WriteStr => exact(&[SsaType::Str], &SsaType::Unit),
        RuntimeOp::Flush => exact(&[], &SsaType::Unit),
        RuntimeOp::ReadByte => exact(&[], &SsaType::I64),
        RuntimeOp::WriteByte => exact(&[SsaType::I64], &SsaType::Unit),
        RuntimeOp::EmptyStr => exact(&[], &SsaType::Str),
        RuntimeOp::ArgCount => exact(&[], &SsaType::I64),
        RuntimeOp::Arg => exact(
            &[SsaType::I64],
            &crate::prelude_contract::option(SsaType::Str),
        ),
        RuntimeOp::BufNew => exact(&[SsaType::I64], &SsaType::Buf),
        RuntimeOp::OwnedBufNew => exact(&[SsaType::I64], &SsaType::Owned(Box::new(SsaType::Buf))),
        RuntimeOp::OwnedBufLen => exact(&[SsaType::Ref(Box::new(SsaType::Buf))], &SsaType::I64),
        RuntimeOp::OwnedBufRef => exact(
            &[SsaType::Ref(Box::new(SsaType::Buf)), SsaType::I64],
            &SsaType::I64,
        ),
        RuntimeOp::OwnedBufSet => exact(
            &[
                SsaType::RefMut(Box::new(SsaType::Buf)),
                SsaType::I64,
                SsaType::I64,
            ],
            &SsaType::Unit,
        ),
        RuntimeOp::BufLen => exact(&[SsaType::Buf], &SsaType::I64),
        RuntimeOp::BufRef | RuntimeOp::BufGetU32 => {
            exact(&[SsaType::Buf, SsaType::I64], &SsaType::I64)
        }
        RuntimeOp::BufSet | RuntimeOp::BufSetU32 => {
            exact(&[SsaType::Buf, SsaType::I64, SsaType::I64], &SsaType::Unit)
        }
        RuntimeOp::BufClone | RuntimeOp::BufFromStr => match operation {
            RuntimeOp::BufClone => exact(&[SsaType::Buf], &SsaType::Buf),
            RuntimeOp::BufFromStr => exact(&[SsaType::Str], &SsaType::Buf),
            _ => false,
        },
        RuntimeOp::BufToStr => exact(
            &[SsaType::Buf],
            &crate::prelude_contract::result(
                SsaType::Str,
                SsaType::Enum {
                    id: crate::EnumId::new(crate::prelude_contract::UTF8_ERROR_ID),
                    arguments: Vec::new(),
                },
            ),
        ),
        RuntimeOp::BufSlice => exact(
            &[SsaType::Buf, SsaType::I64, SsaType::I64],
            &system_result(SsaType::Buf),
        ),
        RuntimeOp::StrLen => exact(&[SsaType::Str], &SsaType::I64),
        RuntimeOp::StrRef => exact(&[SsaType::Str, SsaType::I64], &SsaType::I64),
        RuntimeOp::StrAppend => exact(&[SsaType::Str, SsaType::Str], &SsaType::Str),
        RuntimeOp::StrSlice => exact(&[SsaType::Str, SsaType::I64, SsaType::I64], &SsaType::Str),
        RuntimeOp::StrFromByte | RuntimeOp::StrFromI64 => exact(&[SsaType::I64], &SsaType::Str),
        RuntimeOp::StrFromF64 => exact(&[SsaType::F64], &SsaType::Str),
        _ => return None,
    };
    Some(valid)
}
