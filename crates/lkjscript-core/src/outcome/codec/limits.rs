use super::StructuralSnapshotLimits;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExecutionOutcomeCodecLimits {
    pub max_wire_bytes: usize,
    pub structural: StructuralSnapshotLimits,
}

impl ExecutionOutcomeCodecLimits {
    pub const fn new(max_wire_bytes: usize, structural: StructuralSnapshotLimits) -> Self {
        Self {
            max_wire_bytes,
            structural,
        }
    }

    pub(super) fn validate(self) -> Result<Self> {
        if self.max_wire_bytes == 0 {
            return Err(Error::msg("execution outcome wire bound must be nonzero"));
        }
        self.structural.validate()?;
        Ok(self)
    }
}

impl From<usize> for ExecutionOutcomeCodecLimits {
    fn from(max_wire_bytes: usize) -> Self {
        Self::new(max_wire_bytes, StructuralSnapshotLimits::DEFAULT)
    }
}
