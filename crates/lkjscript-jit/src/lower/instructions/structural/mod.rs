use super::*;

type NativeResult = Result<lkjscript_native::ValueId, LoweringError>;

include!("lifecycle.rs");
include!("ownership.rs");
include!("operands.rs");
include!("runtime.rs");
include!("instruction.rs");
include!("helpers.rs");
