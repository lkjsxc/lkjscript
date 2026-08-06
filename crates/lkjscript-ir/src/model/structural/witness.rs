use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryWitnessGroupMember {
    pub witness: MemoryWitnessId,
    pub ordinal: u64,
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
    pub ordinal: u64,
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
