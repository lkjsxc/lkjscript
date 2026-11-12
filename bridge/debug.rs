//! Rust bootstrap formatting for Result::unwrap and test assertion failures.
//! The language source graph has no Debug requirement. This bridge disappears
//! when the owned test host prints diagnostics and syntax summaries directly.
use crate::ast::{Diagnostic, Program, Span};
use std::fmt;

impl fmt::Debug for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Diagnostic")
            .field("path", &self.path)
            .field("file", &self.file)
            .field("line", &self.line)
            .field("column", &self.column)
            .field("code", &self.code)
            .field("message", &self.message)
            .finish()
    }
}

impl fmt::Debug for Span {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Span")
            .field("file", &self.file)
            .field("start", &self.start)
            .field("end", &self.end)
            .field("line", &self.line)
            .field("column", &self.column)
            .finish()
    }
}

impl fmt::Debug for Program {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let files: Vec<_> = self.files.iter().map(|file| file.path.as_str()).collect();
        let functions: Vec<_> = self
            .functions
            .iter()
            .map(|function| function.name.as_str())
            .collect();
        formatter
            .debug_struct("Program")
            .field("files", &files)
            .field("functions", &functions)
            .field("resolved", &self.resolved)
            .field("declarations", &self.declarations.len())
            .field("types", &self.types.len())
            .field("expressions", &self.exprs.len())
            .field("patterns", &self.patterns.len())
            .field("blocks", &self.blocks.len())
            .finish_non_exhaustive()
    }
}
