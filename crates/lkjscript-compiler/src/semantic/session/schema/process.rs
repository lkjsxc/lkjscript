use std::fmt;

use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionProcessError {
    code: ProcessCode,
    message: String,
    #[serde(skip)]
    budget: Option<Box<lkjscript_core::BudgetError>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
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
            budget: None,
        }
    }

    pub(in crate::semantic::session) fn budget(error: lkjscript_core::BudgetError) -> Self {
        Self {
            code: ProcessCode::FrameTooLarge,
            message: error.to_string(),
            budget: Some(Box::new(error)),
        }
    }

    pub fn budget_error(&self) -> Option<&lkjscript_core::BudgetError> {
        self.budget.as_deref()
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
