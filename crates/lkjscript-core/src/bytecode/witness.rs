use super::{MemoryWitnessId, StructuralRepresentationId};

pub const MAX_MEMORY_WITNESSES: usize = 16_384;
pub const MAX_MEMORY_WITNESS_DEPENDENCIES: usize = 65_536;
pub const MAX_MEMORY_WITNESS_PARAMETERS: usize = 16;
pub const MAX_CALL_WITNESS_SITES: usize = 65_536;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MemoryWitnessValueKind {
    Unit,
    Bool,
    I64,
    F64,
    List,
    Structural(StructuralRepresentationId),
    Unsupported,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct InstalledMemoryWitness {
    pub id: MemoryWitnessId,
    pub facts: lkjscript_contracts::ExecutableMemoryWitnessFacts,
    pub dependencies: Vec<MemoryWitnessId>,
    pub value_kind: MemoryWitnessValueKind,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MemoryWitnessParameter {
    pub parameter: u16,
    pub operations: Vec<lkjscript_contracts::MemoryWitnessOperation>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct MemoryWitnessBinding {
    pub parameter: u16,
    pub witness: u16,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CallWitnessSite {
    pub offset: u32,
    pub callee: u32,
    pub bindings: Vec<MemoryWitnessBinding>,
}
