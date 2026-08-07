#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExecutionOutcomeCodecLimits {
    /// Explicit process-transport policy. Local VM returns do not use it.
    pub max_wire_bytes: usize,
}

impl ExecutionOutcomeCodecLimits {
    pub const fn new(max_wire_bytes: usize) -> Self {
        Self { max_wire_bytes }
    }
}

impl From<usize> for ExecutionOutcomeCodecLimits {
    fn from(max_wire_bytes: usize) -> Self {
        Self::new(max_wire_bytes)
    }
}
