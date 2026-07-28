use super::*;

pub(in crate::executable) unsafe fn invoke_typed(
    address: *mut c_void,
    result: ValueType,
    arguments: &[MachineArgument],
    state: *mut c_void,
) -> Result<RawReturn, InvocationError> {
    let state = state.cast::<NativeCallState>();
    macro_rules! call {
            ($type:ty $(, $argument:expr)*) => {{
                // SAFETY: The caller validates that `address` is a sealed entry
                // with the exact closed SysV signature selected by this match.
                let function: $type = unsafe { std::mem::transmute(address) };
                function(state $(, $argument)*)
            }};
        }

    match (arguments, result) {
        ([], ValueType::I64 | ValueType::Bool | ValueType::Reference(_)) => Ok(RawReturn::Integer(
            call!(extern "C" fn(*mut NativeCallState) -> u64),
        )),
        ([], ValueType::F64) => Ok(RawReturn::Float(call!(
            extern "C" fn(*mut NativeCallState) -> f64
        ))),
        ([], ValueType::Unit) => {
            call!(extern "C" fn(*mut NativeCallState));
            Ok(RawReturn::Unit)
        }
        (
            [MachineArgument::Integer(first)],
            ValueType::I64 | ValueType::Bool | ValueType::Reference(_),
        ) => Ok(RawReturn::Integer(call!(
            extern "C" fn(*mut NativeCallState, u64) -> u64,
            *first
        ))),
        ([MachineArgument::Integer(first)], ValueType::F64) => Ok(RawReturn::Float(call!(
            extern "C" fn(*mut NativeCallState, u64) -> f64,
            *first
        ))),
        ([MachineArgument::Integer(first)], ValueType::Unit) => {
            call!(extern "C" fn(*mut NativeCallState, u64), *first);
            Ok(RawReturn::Unit)
        }
        (
            [MachineArgument::Float(first)],
            ValueType::I64 | ValueType::Bool | ValueType::Reference(_),
        ) => Ok(RawReturn::Integer(call!(
            extern "C" fn(*mut NativeCallState, f64) -> u64,
            *first
        ))),
        ([MachineArgument::Float(first)], ValueType::F64) => Ok(RawReturn::Float(call!(
            extern "C" fn(*mut NativeCallState, f64) -> f64,
            *first
        ))),
        ([MachineArgument::Float(first)], ValueType::Unit) => {
            call!(extern "C" fn(*mut NativeCallState, f64), *first);
            Ok(RawReturn::Unit)
        }
        (
            [MachineArgument::Integer(first), MachineArgument::Integer(second)],
            ValueType::I64 | ValueType::Bool | ValueType::Reference(_),
        ) => Ok(RawReturn::Integer(call!(
            extern "C" fn(*mut NativeCallState, u64, u64) -> u64,
            *first,
            *second
        ))),
        ([MachineArgument::Integer(first), MachineArgument::Integer(second)], ValueType::F64) => {
            Ok(RawReturn::Float(call!(
                extern "C" fn(*mut NativeCallState, u64, u64) -> f64,
                *first,
                *second
            )))
        }
        ([MachineArgument::Integer(first), MachineArgument::Integer(second)], ValueType::Unit) => {
            call!(
                extern "C" fn(*mut NativeCallState, u64, u64),
                *first,
                *second
            );
            Ok(RawReturn::Unit)
        }
        (
            [MachineArgument::Integer(first), MachineArgument::Float(second)],
            ValueType::I64 | ValueType::Bool | ValueType::Reference(_),
        ) => Ok(RawReturn::Integer(call!(
            extern "C" fn(*mut NativeCallState, u64, f64) -> u64,
            *first,
            *second
        ))),
        ([MachineArgument::Integer(first), MachineArgument::Float(second)], ValueType::F64) => {
            Ok(RawReturn::Float(call!(
                extern "C" fn(*mut NativeCallState, u64, f64) -> f64,
                *first,
                *second
            )))
        }
        ([MachineArgument::Integer(first), MachineArgument::Float(second)], ValueType::Unit) => {
            call!(
                extern "C" fn(*mut NativeCallState, u64, f64),
                *first,
                *second
            );
            Ok(RawReturn::Unit)
        }
        (
            [MachineArgument::Float(first), MachineArgument::Integer(second)],
            ValueType::I64 | ValueType::Bool | ValueType::Reference(_),
        ) => Ok(RawReturn::Integer(call!(
            extern "C" fn(*mut NativeCallState, f64, u64) -> u64,
            *first,
            *second
        ))),
        ([MachineArgument::Float(first), MachineArgument::Integer(second)], ValueType::F64) => {
            Ok(RawReturn::Float(call!(
                extern "C" fn(*mut NativeCallState, f64, u64) -> f64,
                *first,
                *second
            )))
        }
        ([MachineArgument::Float(first), MachineArgument::Integer(second)], ValueType::Unit) => {
            call!(
                extern "C" fn(*mut NativeCallState, f64, u64),
                *first,
                *second
            );
            Ok(RawReturn::Unit)
        }
        (
            [MachineArgument::Float(first), MachineArgument::Float(second)],
            ValueType::I64 | ValueType::Bool | ValueType::Reference(_),
        ) => Ok(RawReturn::Integer(call!(
            extern "C" fn(*mut NativeCallState, f64, f64) -> u64,
            *first,
            *second
        ))),
        ([MachineArgument::Float(first), MachineArgument::Float(second)], ValueType::F64) => {
            Ok(RawReturn::Float(call!(
                extern "C" fn(*mut NativeCallState, f64, f64) -> f64,
                *first,
                *second
            )))
        }
        ([MachineArgument::Float(first), MachineArgument::Float(second)], ValueType::Unit) => {
            call!(
                extern "C" fn(*mut NativeCallState, f64, f64),
                *first,
                *second
            );
            Ok(RawReturn::Unit)
        }
        _ => Err(InvocationError::UnsupportedSignature),
    }
}
