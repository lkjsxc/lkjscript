use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceError {
    pub code: &'static str,
    pub detail: String,
}

impl ResourceError {
    pub fn new(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for ResourceError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(output, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for ResourceError {}

pub type ResourceResult<T> = Result<T, ResourceError>;
