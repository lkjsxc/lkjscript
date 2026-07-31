use std::collections::HashSet;

use crate::image::{
    entry_metadata, exact_safepoint, frame_facts, frame_home, heap_runtime_site, outcome_map_entry,
    relocation, root_location, root_map_requirement, source_map_entry, structural_runtime_site,
    trap_map_entry, FrameHomeKind, ImageContracts, ImageParts, InstallableImage,
    NativeExecutionDomain, OutcomeKind, RelocationKind, RelocationTarget, RootLocation,
};
use crate::plan::{
    BlockId, BoolComparison, F64Comparison, FunctionId, FunctionPlan, I64Comparison, Instruction,
    Operation, RuntimeCallSlot, RuntimeOutcome, Terminator, TrapCode, ValueId, ValueType,
};
use crate::verify::{CertifiedRoot, FunctionRootRequirements, VerifiedMachinePlan};
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
mod roots;

pub use entry::encode;
use layouts::*;
use relocations::*;
use roots::*;

const SCRATCH_INTEGER_ARGUMENT_0: u8 = 16;
const SCRATCH_INTEGER_ARGUMENT_1: u8 = 24;
const SCRATCH_FLOAT_ARGUMENT_0: u8 = 32;
const SCRATCH_FLOAT_ARGUMENT_1: u8 = 40;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EncodingConfig {
    contracts: ImageContracts,
}

impl EncodingConfig {
    #[must_use]
    pub const fn new(contracts: ImageContracts) -> Self {
        Self { contracts }
    }

    #[must_use]
    pub const fn contracts(self) -> ImageContracts {
        self.contracts
    }
}

impl Default for EncodingConfig {
    fn default() -> Self {
        Self::new(ImageContracts::current())
    }
}

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
    function_ordinal: u32,
    signatures: &'a [(FunctionId, crate::Signature)],
    execution_domain: NativeExecutionDomain,
    collecting_functions: &'a HashSet<FunctionId>,
    bytes: &'a mut Vec<u8>,
    relocations: &'a mut Vec<crate::Relocation>,
    safepoints: &'a mut Vec<crate::Safepoint>,
    root_requirements: &'a mut Vec<crate::image::RootMapRequirement>,
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
    certified_call_roots: &'a FunctionRootRequirements,
    frame_bytes: u32,
    maximum_code_bytes: usize,
}
