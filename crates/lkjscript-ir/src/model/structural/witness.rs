use super::*;

pub const MAX_MEMORY_WITNESS_GROUPS: usize = 16_384;
pub const MAX_MEMORY_WITNESSES: usize = 16_384;
pub const MAX_MEMORY_WITNESS_DEPENDENCIES: usize = 65_536;
pub const MAX_MEMORY_WITNESS_PARAMETERS: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryWitnessGroupMember {
    pub witness: MemoryWitnessId,
    pub ordinal: u16,
    pub semantic_identity: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryWitnessGroupDescriptor {
    pub id: MemoryWitnessGroupId,
    pub recursive: bool,
    pub members: Vec<MemoryWitnessGroupMember>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryWitnessDescriptor {
    pub id: MemoryWitnessId,
    pub group: MemoryWitnessGroupId,
    pub ordinal: u16,
    pub facts: lkjscript_contracts::ExecutableMemoryWitnessFacts,
    pub ty: SsaType,
    pub dependencies: Vec<lkjscript_contracts::ExecutableMemoryWitnessDependency>,
    pub representation: Option<StructuralRepresentationId>,
}

impl MemoryWitnessDescriptor {
    pub fn supports(&self, operation: lkjscript_contracts::MemoryWitnessOperation) -> bool {
        self.facts.operations.binary_search(&operation).is_ok()
    }
}
