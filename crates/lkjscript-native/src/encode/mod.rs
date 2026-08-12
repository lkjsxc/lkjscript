use std::collections::HashSet;

use crate::image::{
    entry_metadata, frame_facts, frame_home, heap_runtime_site, outcome_map_entry, relocation,
    source_map_entry, structural_runtime_site, trap_map_entry, FrameHomeKind, ImageParts,
    InstallableImage, NativeExecutionDomain, OutcomeKind, RelocationKind, RelocationTarget,
};
use crate::plan::{
    BlockId, BoolComparison, F64Comparison, FunctionId, FunctionPlan, I64Comparison, Instruction,
    Operation, RuntimeCallSlot, RuntimeOutcome, Terminator, TrapCode, ValueId, ValueType,
};
use crate::verify::VerifiedMachinePlan;
use crate::{EncodeError, NativeError};

mod calls;
mod control;
mod emission;
mod entry;
mod instructions;
mod layouts;
mod lifecycle;
mod numeric;
mod relocations;

pub use entry::encode;
use layouts::*;
use relocations::*;

const SCRATCH_INTEGER_ARGUMENT_0: u8 = 16;
const SCRATCH_INTEGER_ARGUMENT_1: u8 = 24;
const SCRATCH_INTEGER_ARGUMENT_2: u8 = 32;
const SCRATCH_INTEGER_ARGUMENT_3: u8 = 40;
const SCRATCH_INTEGER_ARGUMENT_4: u8 = 48;
const SCRATCH_FLOAT_ARGUMENT_0: u8 = 56;
const SCRATCH_FLOAT_ARGUMENT_1: u8 = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FixupTarget {
    Block(BlockId),
    Trap(TrapCode),
    StatusReturn,
    UnregisteredStatusReturn,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BranchFixup {
    displacement_offset: usize,
    target: FixupTarget,
}

struct FunctionEncoder<'a> {
    function: &'a FunctionPlan,
    function_ordinal: u64,
    signatures: &'a [(FunctionId, crate::Signature)],
    bytes: &'a mut Vec<u8>,
    relocations: &'a mut Vec<crate::Relocation>,
    heap_runtime_sites: &'a mut Vec<crate::HeapRuntimeSite>,
    structural_runtime_sites: &'a mut Vec<crate::StructuralRuntimeSite>,
    source_map: &'a mut Vec<crate::SourceMapEntry>,
    trap_map: &'a mut Vec<crate::TrapMapEntry>,
    outcome_map: &'a mut Vec<crate::OutcomeMapEntry>,
    runtime_calls: &'a mut HashSet<RuntimeCallSlot>,
    fixups: Vec<BranchFixup>,
    block_offsets: Vec<Option<usize>>,
    trap_offsets: [Option<usize>; 3],
    status_return_offset: Option<usize>,
    unregistered_status_return_offset: Option<usize>,
    frame_bytes: u32,
    maximum_code_bytes: usize,
}
