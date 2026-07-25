use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrError {
    message: String,
}

impl IrError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for IrError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for IrError {}

pub type Result<T> = std::result::Result<T, IrError>;
