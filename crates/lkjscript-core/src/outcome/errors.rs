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
    prior_outcome: Option<String>,
}

impl HostError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            prior_outcome: None,
        }
    }

    pub fn during_cleanup(message: impl Into<String>, prior_outcome: String) -> Self {
        Self {
            message: message.into(),
            prior_outcome: Some(prior_outcome),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.message
    }

    pub fn prior_outcome(&self) -> Option<&str> {
        self.prior_outcome.as_deref()
    }
}

impl fmt::Display for HostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)?;
        if let Some(prior) = &self.prior_outcome {
            write!(formatter, " (prior outcome: {prior})")?;
        }
        Ok(())
    }
}
