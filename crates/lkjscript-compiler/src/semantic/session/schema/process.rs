use std::fmt;

use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionProcessError {
    code: ProcessCode,
    message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(in crate::semantic::session) enum ProcessCode {
    PartialHeader,
    PartialPayload,
    FrameTooLarge,
    LengthOverflow,
    InvalidJson,
    InputFailure,
    OutputFailure,
}

impl SessionProcessError {
    pub(in crate::semantic::session) fn new(code: ProcessCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub(in crate::semantic::session) fn output(error: std::io::Error) -> Self {
        Self::new(
            ProcessCode::OutputFailure,
            format!("session output: {error}"),
        )
    }
}

impl fmt::Display for SessionProcessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        serde_json::to_string(self)
            .map_err(|_| fmt::Error)
            .and_then(|encoded| formatter.write_str(&encoded))
    }
}

impl std::error::Error for SessionProcessError {}
