use super::*;

/// One generic heap-dispatch site whose arguments and result are copied only
/// through verified generated-frame homes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HeapRuntimeSite {
    pub(super) id: u32,
    pub(super) function: FunctionId,
    pub(super) safepoint: u32,
    pub(super) descriptor: HeapCallDescriptor,
    pub(super) arguments: Vec<FrameHome>,
    pub(super) result: FrameHome,
    pub(super) source: Option<SourceOrigin>,
}

impl HeapRuntimeSite {
    #[must_use]
    pub const fn id(&self) -> u32 {
        self.id
    }

    #[must_use]
    pub const fn function(&self) -> FunctionId {
        self.function
    }

    #[must_use]
    pub const fn safepoint(&self) -> u32 {
        self.safepoint
    }

    #[must_use]
    pub const fn descriptor(&self) -> &HeapCallDescriptor {
        &self.descriptor
    }

    #[must_use]
    pub fn arguments(&self) -> &[FrameHome] {
        &self.arguments
    }

    #[must_use]
    pub const fn result(&self) -> FrameHome {
        self.result
    }

    #[must_use]
    pub const fn source(&self) -> Option<SourceOrigin> {
        self.source
    }
}
