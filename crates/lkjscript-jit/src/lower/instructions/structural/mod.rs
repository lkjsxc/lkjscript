use super::*;

type NativeResult = Result<lkjscript_native::ValueId, LoweringError>;

include!("dynamic_witness.rs");
include!("lifecycle.rs");
include!("ownership.rs");
include!("operands.rs");
include!("runtime.rs");
include!("projection.rs");
include!("instruction.rs");
include!("helpers.rs");
