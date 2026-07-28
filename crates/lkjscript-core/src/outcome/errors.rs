use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceLimitKind {
    InstructionFuel,
    StackValues,
    FrameDepth,
    HeapBytes,
    Allocations,
    LogicalAggregateConstructions,
    Handles,
    OutputBytes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trap {
    message: String,
}

impl Trap {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for Trap {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostError {
    message: String,
}

impl HostError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for HostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}
