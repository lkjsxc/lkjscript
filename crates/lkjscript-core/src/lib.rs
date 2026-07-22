//! Pure language core: values, bytecode, limits, and JIT hook surface.

mod chunk;
mod error;
mod jit;
mod limits;
mod opcode;
mod value;

pub use chunk::{Chunk, ConstId, Constant, FunctionProto};
pub use error::{Error, Result};
pub use jit::{JitHook, NullJit};
pub use limits::{
    Limits, MAX_CHILDREN, MAX_DIR_CHILDREN, MAX_LIST_EQUAL_STEPS, MAX_NEST_DEPTH,
    MAX_TOKENS_PER_FILE, MAX_TOPLEVEL_FORMS,
};

pub use opcode::Op;
pub use value::{HeapObj, Value, MAX_SMALL_I64, MIN_SMALL_I64};
