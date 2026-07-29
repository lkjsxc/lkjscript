use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinuxHostError {
    pub code: &'static str,
    pub detail: String,
}

impl LinuxHostError {
    pub(crate) fn new(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }

    pub(crate) fn io(code: &'static str, path: &std::path::Path, error: &std::io::Error) -> Self {
        Self::new(code, format!("{}: {error}", path.display()))
    }
}

impl fmt::Display for LinuxHostError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(output, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for LinuxHostError {}

impl From<lkjscript_resource::ResourceError> for LinuxHostError {
    fn from(error: lkjscript_resource::ResourceError) -> Self {
        Self::new(error.code, error.detail)
    }
}

pub(crate) type HostResult<T> = Result<T, LinuxHostError>;
