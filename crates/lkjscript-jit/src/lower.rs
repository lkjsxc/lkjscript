use std::collections::{HashMap, HashSet};
use std::fmt;

use lkjscript_ir::{
    Block, CallTarget, Constant, Function, FunctionId, Instruction, InstructionKind, RuntimeOp,
    SsaType, StructuredOutcome, Terminator, ValueId, VerifiedProgram,
};
use lkjscript_native::{
    AllocationClass, BackendLimits, BoolComparison, F64Comparison, FunctionBuilder,
    HeapCallDescriptor, HeapOperation, I64Comparison, InstallableImage, LayoutIdentity, LoanType,
    LocalId, MachinePlanBuilder, NativeError, ReferenceType, RuntimeCallSlot, RuntimeOutcome,
    Signature, SourceFunctionId, SourceOrigin, StoreClass, TrapCode, UniqueType, ValueType,
};

mod enum_types;
mod enums;
mod functions;
mod group;
mod instructions;
mod layout;
mod numeric;
mod preflight;
mod reachability;
mod runtime_calls;
mod scalar_helpers;
mod terminators;
#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;
mod types;
mod values;

use enum_types::*;
use enums::*;
use functions::*;
pub(crate) use group::lower_baseline_group;
use instructions::*;
use layout::*;
use numeric::*;
use preflight::*;
pub(crate) use reachability::reachable_group;
use runtime_calls::*;
use scalar_helpers::*;
use terminators::*;
use types::*;
use values::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoweringFailureCode {
    UnsupportedType,
    UnsupportedOperation,
    UnsupportedSignature,
    IndirectCall,
    RecursiveCallGraph,
    InvalidFunction,
    Backend,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoweringError {
    code: LoweringFailureCode,
    function: Option<FunctionId>,
    detail: String,
}

impl LoweringError {
    fn new(
        code: LoweringFailureCode,
        function: Option<FunctionId>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            code,
            function,
            detail: detail.into(),
        }
    }

    fn backend(error: impl fmt::Display) -> Self {
        Self::new(LoweringFailureCode::Backend, None, error.to_string())
    }

    pub const fn code(&self) -> LoweringFailureCode {
        self.code
    }

    pub const fn function(&self) -> Option<FunctionId> {
        self.function
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for LoweringError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(function) = self.function {
            write!(formatter, "function {}: {}", function.raw(), self.detail)
        } else {
            formatter.write_str(&self.detail)
        }
    }
}

impl std::error::Error for LoweringError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LoweringDomain {
    ResourceIsland,
    UniqueIsland,
    StructuralIsland,
    Legacy,
}

pub(crate) struct LoweredGroup {
    pub(crate) image: InstallableImage,
    pub(crate) functions: Vec<FunctionId>,
    pub(crate) native_functions: Vec<(FunctionId, lkjscript_native::FunctionId)>,
    pub(crate) explicit_traps: Vec<(u64, String)>,
}
#[derive(Default)]
struct LayoutInterner {
    identities: HashMap<SsaType, LayoutIdentity>,
    semantics: HashMap<SsaType, u64>,
    region_products: HashMap<lkjscript_ir::ProductId, [u8; 32]>,
    structural: StructuralCatalog,
    witness_slots: HashMap<
        (
            lkjscript_ir::MemoryWitnessId,
            lkjscript_native::StructuralStorageRoute,
        ),
        u64,
    >,
    next: u64,
}

#[derive(Clone, Copy)]
struct EdgeBlocks {
    branch: Option<lkjscript_native::BlockId>,
    when_true: Option<lkjscript_native::BlockId>,
    when_false: Option<lkjscript_native::BlockId>,
}

#[derive(Clone, Copy)]
struct RuntimeLoweringContext<'a> {
    block: lkjscript_native::BlockId,
    locals: &'a [LocalId],
    value_types: &'a [ValueType],
    result_type: ValueType,
}

#[derive(Clone, Copy)]
struct TerminatorContext<'a> {
    native_block: lkjscript_native::BlockId,
    edges: EdgeBlocks,
    blocks: &'a [lkjscript_native::BlockId],
    locals: &'a [LocalId],
    value_types: &'a [ValueType],
    layouts: &'a LayoutInterner,
}
