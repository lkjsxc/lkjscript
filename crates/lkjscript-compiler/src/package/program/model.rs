use lkjscript_contracts::{
    PackageProvenanceKind, PreparedProgramDescriptor, PreparedProgramIdentity,
};

#[derive(Clone)]
pub(crate) struct PreparationProvenance {
    pub kind: PackageProvenanceKind,
    pub package_content: [u8; 32],
    pub package_root: [u8; 32],
    pub entry: [u8; 32],
    pub module_memory_closure: [u8; 32],
    pub witness_closure: [u8; 32],
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct PreparedProgram {
    pub(super) descriptor: PreparedProgramDescriptor,
    pub(super) identity: PreparedProgramIdentity,
}

impl std::fmt::Debug for PreparedProgram {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        output.write_str("PreparedProgram(<redacted>)")
    }
}

impl PreparedProgram {
    pub const fn descriptor(self) -> PreparedProgramDescriptor {
        self.descriptor
    }

    pub const fn identity(self) -> PreparedProgramIdentity {
        self.identity
    }
}
