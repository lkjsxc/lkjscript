use super::{MemoryWitnessId, StructuralRepresentationId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct MemoryWitnessGroupId([u8; 32]);

impl MemoryWitnessGroupId {
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }
    pub fn is_resolved(self) -> bool {
        self.0 != [0; 32]
    }
}

pub const MAX_MEMORY_WITNESS_GROUPS: usize = 16_384;
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
pub struct InstalledMemoryWitnessGroupMember {
    pub witness: MemoryWitnessId,
    pub ordinal: u16,
    pub semantic_identity: [u8; 32],
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct InstalledMemoryWitnessGroup {
    pub id: MemoryWitnessGroupId,
    pub recursive: bool,
    pub members: Vec<InstalledMemoryWitnessGroupMember>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct InstalledMemoryWitness {
    pub id: MemoryWitnessId,
    pub group: MemoryWitnessGroupId,
    pub ordinal: u16,
    pub facts: lkjscript_contracts::ExecutableMemoryWitnessFacts,
    pub dependencies: Vec<lkjscript_contracts::ExecutableMemoryWitnessDependency>,
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
    pub offset: u64,
    pub callee: u32,
    pub bindings: Vec<MemoryWitnessBinding>,
}
