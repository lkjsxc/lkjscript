use super::{
    StructuralRootTable, StructuralRootTableError, StructuralRootTableLimit,
    StructuralRootTableStats,
};
use crate::structural::StructuralRuntimeId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StructuralRootTableLimits {
    pub max_roots: u32,
    pub max_loans: u32,
    pub max_generation: u32,
}

impl StructuralRootTableLimits {
    pub fn validate(self) -> Result<Self, StructuralRootTableError> {
        if self.max_roots == 0 || self.max_loans == 0 || self.max_generation == 0 {
            return Err(StructuralRootTableError::InvalidLimits);
        }
        Ok(self)
    }
}

impl Default for StructuralRootTableLimits {
    fn default() -> Self {
        Self {
            max_roots: 65_536,
            max_loans: 65_536,
            max_generation: u32::MAX,
        }
    }
}

impl StructuralRootTable {
    pub fn new(
        runtime: StructuralRuntimeId,
        limits: StructuralRootTableLimits,
    ) -> Result<Self, StructuralRootTableError> {
        Ok(Self {
            runtime,
            limits: limits.validate()?,
            roots: Vec::new(),
            free_roots: Vec::new(),
            loans: Vec::new(),
            free_loans: Vec::new(),
            stats: StructuralRootTableStats::default(),
        })
    }

    pub const fn runtime(&self) -> StructuralRuntimeId {
        self.runtime
    }

    pub const fn stats(&self) -> StructuralRootTableStats {
        self.stats
    }

    pub(super) fn root_limit() -> StructuralRootTableError {
        StructuralRootTableError::LimitExceeded(StructuralRootTableLimit::Roots)
    }

    pub(super) fn loan_limit() -> StructuralRootTableError {
        StructuralRootTableError::LimitExceeded(StructuralRootTableLimit::Loans)
    }
}
