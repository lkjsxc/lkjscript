use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceMapEntry {
    pub(super) function: FunctionId,
    pub(super) code_start: u32,
    pub(super) code_end: u32,
    pub(super) source: Option<SourceOrigin>,
}

impl SourceMapEntry {
    #[must_use]
    pub const fn function(self) -> FunctionId {
        self.function
    }

    #[must_use]
    pub const fn code_start(self) -> u32 {
        self.code_start
    }

    #[must_use]
    pub const fn code_end(self) -> u32 {
        self.code_end
    }

    #[must_use]
    pub const fn source(self) -> Option<SourceOrigin> {
        self.source
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TrapMapEntry {
    pub(super) function: FunctionId,
    pub(super) code_offset: u32,
    pub(super) trap: TrapCode,
    pub(super) site: Option<u32>,
}

impl TrapMapEntry {
    #[must_use]
    pub const fn function(self) -> FunctionId {
        self.function
    }

    #[must_use]
    pub const fn code_offset(self) -> u32 {
        self.code_offset
    }

    #[must_use]
    pub const fn trap(self) -> TrapCode {
        self.trap
    }

    #[must_use]
    pub const fn site(self) -> Option<u32> {
        self.site
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutcomeKind {
    Return,
    Trap(TrapCode),
    Exit,
    DeadlineExceeded,
    ResourceLimitExceeded,
    HostFailure,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OutcomeMapEntry {
    pub(super) function: FunctionId,
    pub(super) code_offset: u32,
    pub(super) outcome: OutcomeKind,
}

impl OutcomeMapEntry {
    #[must_use]
    pub const fn function(self) -> FunctionId {
        self.function
    }

    #[must_use]
    pub const fn code_offset(self) -> u32 {
        self.code_offset
    }

    #[must_use]
    pub const fn outcome(self) -> OutcomeKind {
        self.outcome
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CodeAccounting {
    pub(super) code_bytes: u64,
    pub(super) metadata_bytes: u64,
    pub(super) work_units: u64,
}

impl CodeAccounting {
    #[must_use]
    pub const fn code_bytes(self) -> u64 {
        self.code_bytes
    }

    #[must_use]
    pub const fn metadata_bytes(self) -> u64 {
        self.metadata_bytes
    }

    #[must_use]
    pub const fn work_units(self) -> u64 {
        self.work_units
    }
}
