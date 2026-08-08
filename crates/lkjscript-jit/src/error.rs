use std::fmt;

use crate::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FailureCode {
    UnsupportedType,
    UnsupportedOperation,
    UnsupportedSignature,
    IndirectCall,
    RecursionUnsupported,
    InvalidVerifiedProgram,
    BackendVerification,
    CompileWallTime,
    InstallLimit,
    InstallFailure,
    NativeStackBoundary,
    InvocationFailure,
}

impl FailureCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedType => "unsupported-type",
            Self::UnsupportedOperation => "unsupported-operation",
            Self::UnsupportedSignature => "unsupported-signature",
            Self::IndirectCall => "indirect-call",
            Self::RecursionUnsupported => "recursion-unsupported",
            Self::InvalidVerifiedProgram => "invalid-verified-program",
            Self::BackendVerification => "backend-verification",
            Self::CompileWallTime => "compile-wall-time",
            Self::InstallLimit => "install-limit",
            Self::InstallFailure => "install-failure",
            Self::NativeStackBoundary => "native-stack-boundary",
            Self::InvocationFailure => "invocation-failure",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EngineError {
    code: FailureCode,
    function: Option<FunctionId>,
    detail: String,
}

impl EngineError {
    pub(crate) fn new(
        code: FailureCode,
        function: Option<FunctionId>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            code,
            function,
            detail: detail.into(),
        }
    }

    pub const fn code(&self) -> FailureCode {
        self.code
    }

    pub const fn function(&self) -> Option<FunctionId> {
        self.function
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for EngineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(function) = self.function {
            write!(
                formatter,
                "JIT {:?} in function {}: {}",
                self.code,
                function.raw(),
                self.detail
            )
        } else {
            write!(formatter, "JIT {:?}: {}", self.code, self.detail)
        }
    }
}

impl std::error::Error for EngineError {}

impl From<LoweringError> for EngineError {
    fn from(error: LoweringError) -> Self {
        let code = match error.code() {
            LoweringFailureCode::UnsupportedType => FailureCode::UnsupportedType,
            LoweringFailureCode::UnsupportedOperation => FailureCode::UnsupportedOperation,
            LoweringFailureCode::UnsupportedSignature => FailureCode::UnsupportedSignature,
            LoweringFailureCode::IndirectCall => FailureCode::IndirectCall,
            LoweringFailureCode::RecursiveCallGraph => FailureCode::RecursionUnsupported,
            LoweringFailureCode::InvalidFunction => FailureCode::InvalidVerifiedProgram,
            LoweringFailureCode::Backend => FailureCode::BackendVerification,
        };
        Self::new(code, error.function(), error.detail())
    }
}
