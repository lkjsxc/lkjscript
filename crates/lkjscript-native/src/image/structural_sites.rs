use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuralRuntimeSite {
    pub(super) id: u64,
    pub(super) function: FunctionId,
    pub(super) descriptor: StructuralCallDescriptor,
    pub(super) source: Option<SourceOrigin>,
}

impl StructuralRuntimeSite {
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    #[must_use]
    pub const fn function(&self) -> FunctionId {
        self.function
    }

    #[must_use]
    pub const fn descriptor(&self) -> &StructuralCallDescriptor {
        &self.descriptor
    }

    #[must_use]
    pub const fn source(&self) -> Option<SourceOrigin> {
        self.source
    }
}
