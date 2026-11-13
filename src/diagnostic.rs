//! Source locations and diagnostics shared by K and source producers.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Span {
    pub file: usize,
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub path: String,
    pub file: usize,
    pub line: usize,
    pub column: usize,
    pub code: String,
    pub message: String,
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
        }
    }
}
