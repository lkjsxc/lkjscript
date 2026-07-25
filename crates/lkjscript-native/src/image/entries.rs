use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntryMetadata {
    pub(super) function: FunctionId,
    pub(super) source_function: SourceFunctionId,
    pub(super) signature: Signature,
    pub(super) offset: u32,
    pub(super) end: u32,
}

impl EntryMetadata {
    #[must_use]
    pub const fn function(&self) -> FunctionId {
        self.function
    }

    #[must_use]
    pub const fn source_function(&self) -> SourceFunctionId {
        self.source_function
    }

    #[must_use]
    pub fn signature(&self) -> &Signature {
        &self.signature
    }

    #[must_use]
    pub const fn offset(&self) -> u32 {
        self.offset
    }

    #[must_use]
    pub const fn end(&self) -> u32 {
        self.end
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelocationKind {
    Absolute64,
}

impl RelocationKind {
    #[must_use]
    pub const fn width(self) -> usize {
        match self {
            Self::Absolute64 => 8,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelocationTarget {
    Function(FunctionId),
    Runtime(RuntimeCallSlot),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Relocation {
    pub(super) offset: u32,
    pub(super) kind: RelocationKind,
    pub(super) target: RelocationTarget,
}

impl Relocation {
    #[must_use]
    pub const fn offset(self) -> u32 {
        self.offset
    }

    #[must_use]
    pub const fn kind(self) -> RelocationKind {
        self.kind
    }

    #[must_use]
    pub const fn target(self) -> RelocationTarget {
        self.target
    }
}
