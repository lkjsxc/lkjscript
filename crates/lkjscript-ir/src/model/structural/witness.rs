use super::*;

pub const MAX_MEMORY_WITNESSES: usize = 16_384;
pub const MAX_MEMORY_WITNESS_DEPENDENCIES: usize = 65_536;
pub const MAX_MEMORY_WITNESS_PARAMETERS: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryWitnessDescriptor {
    pub id: MemoryWitnessId,
    pub facts: lkjscript_contracts::ExecutableMemoryWitnessFacts,
    pub ty: SsaType,
    pub dependencies: Vec<MemoryWitnessId>,
    pub representation: Option<StructuralRepresentationId>,
}

impl MemoryWitnessDescriptor {
    pub fn supports(&self, operation: lkjscript_contracts::MemoryWitnessOperation) -> bool {
        self.facts.operations.binary_search(&operation).is_ok()
    }
}
