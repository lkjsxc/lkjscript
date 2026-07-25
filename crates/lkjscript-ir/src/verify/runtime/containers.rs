use crate::{RuntimeOp, SsaType};

pub(super) fn container_signature(
    operation: RuntimeOp,
    parameters: &[SsaType],
    result: &SsaType,
) -> Option<bool> {
    let _exact = |expected: &[SsaType], result_type: &SsaType| {
        parameters == expected && result == result_type
    };
    let valid = match operation {
        RuntimeOp::Ok => {
            matches!((parameters, result), ([value], SsaType::Result(ok, _)) if value == ok.as_ref())
        }
        RuntimeOp::Err => {
            matches!((parameters, result), ([value], SsaType::Result(_, err)) if value == err.as_ref())
        }
        RuntimeOp::IsOk => {
            matches!(parameters, [SsaType::Result(_, _)]) && result == &SsaType::Bool
        }
        RuntimeOp::UnwrapOk => {
            matches!(parameters, [SsaType::Result(ok, _)] if ok.as_ref() == result)
        }
        RuntimeOp::UnwrapErr => {
            matches!(parameters, [SsaType::Result(_, err)] if err.as_ref() == result)
        }
        RuntimeOp::Some => {
            matches!((parameters, result), ([value], SsaType::Option(item)) if value == item.as_ref())
        }
        RuntimeOp::IsSome => matches!(parameters, [SsaType::Option(_)]) && result == &SsaType::Bool,
        RuntimeOp::UnwrapSome => {
            matches!(parameters, [SsaType::Option(item)] if item.as_ref() == result)
        }
        _ => return None,
    };
    Some(valid)
}
