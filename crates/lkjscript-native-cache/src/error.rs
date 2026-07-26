use std::fmt;

#[derive(Debug)]
pub struct CacheError(String);

impl CacheError {
    pub(crate) fn message(message: impl Into<String>) -> Self {
        Self(message.into())
    }

    pub(crate) fn host(operation: &str, error: impl fmt::Display) -> Self {
        Self(format!("native cache {operation}: {error}"))
    }
}

impl fmt::Display for CacheError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for CacheError {}
