#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::analyze) enum NameUse {
    Symbol,
    Call,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::analyze) enum AnalysisDiagnostic {
    UnknownName {
        usage: NameUse,
        name: String,
    },
    CallArity {
        name: String,
        expected: usize,
        actual: usize,
    },
    TypeMismatch {
        context: String,
        expected: String,
        actual: String,
    },
}

impl AnalysisDiagnostic {
    pub(in crate::analyze) fn render_human(&self) -> String {
        match self {
            Self::UnknownName { usage, name } => {
                let kind = match usage {
                    NameUse::Symbol => "symbol",
                    NameUse::Call => "call",
                };
                format!("unknown {kind} {name}")
            }
            Self::CallArity {
                name,
                expected,
                actual,
            } => format!("{name}: expected {expected} args, got {actual}"),
            Self::TypeMismatch {
                context,
                expected,
                actual,
            } => format!("{context} {actual} not assignable to {expected}"),
        }
    }
}
