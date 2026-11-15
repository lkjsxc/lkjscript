//! Source locations and diagnostics shared by IR and source producers.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Span {
    pub file: usize,
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

/// One replacement in the diagnostic's original UTF-8 source file.
/// A diagnostic's edits form one ordered, nonoverlapping atomic set.
#[derive(Clone, PartialEq, Eq)]
pub struct Edit {
    pub start: usize,
    pub end: usize,
    pub replacement: String,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub path: String,
    pub file: usize,
    pub line: usize,
    pub column: usize,
    pub code: String,
    pub message: String,
    pub edits: Vec<Edit>,
}

impl Diagnostic {
    pub fn new(span: Span, code: &str, message: &str) -> Diagnostic {
        Diagnostic {
            path: String::new(),
            file: span.file,
            line: span.line,
            column: span.column,
            code: code.to_string(),
            message: message.to_string(),
            edits: Vec::new(),
        }
    }
}
