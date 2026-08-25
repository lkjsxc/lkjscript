use std::fmt;

#[derive(Debug)]
pub(crate) struct DevError {
    kind: &'static str,
    message: String,
}

impl DevError {
    pub(crate) fn usage(message: impl Into<String>) -> Self {
        Self {
            kind: "usage",
            message: message.into(),
        }
    }

    pub(crate) fn infrastructure(message: impl Into<String>) -> Self {
        Self {
            kind: "infrastructure",
            message: message.into(),
        }
    }

    pub(crate) fn corrupt(message: impl Into<String>) -> Self {
        Self {
            kind: "corrupt",
            message: message.into(),
        }
    }

    pub(crate) fn kind(&self) -> &'static str {
        self.kind
    }

    pub(crate) fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for DevError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.kind, self.message)
    }
}

impl std::error::Error for DevError {}

impl From<std::io::Error> for DevError {
    fn from(error: std::io::Error) -> Self {
        Self::infrastructure(error.to_string())
    }
}

impl From<serde_json::Error> for DevError {
    fn from(error: serde_json::Error) -> Self {
        Self::corrupt(error.to_string())
    }
}
