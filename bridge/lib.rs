//! Rust bootstrap library host. The source compiler remains rooted at src/lib.rs.
mod compiler_stack;
mod debug;
#[path = "../src/lib.rs"]
mod language;

pub(crate) use language::facts;
pub use language::{
    ast, check, diagnostics, elaborate, encoding, generics, json, k, k_text, lower, names, resolve,
    source, syntax, wasm,
};

/// Preserve the public compiler boundary while isolating Rust resource setup.
pub mod compiler {
    use crate::ast::{Diagnostic, Span};
    pub use crate::language::compiler::{check_bundle, check_source};
    use crate::source::SourceBundle;

    /// Reserve a native stack from host memory before running the pure pipeline.
    /// This bridge reads resource metadata only. Source/library bytes are never
    /// loaded here; an owned host will replace the Rust thread and this wrapper.
    pub fn compile(bundle: &SourceBundle) -> Result<Vec<u8>, Diagnostic> {
        crate::compiler_stack::run(|| crate::language::compiler::compile(bundle)).map_err(
            |error| {
                let mut problem = Diagnostic::new(Span::default(), "E_HOST", &error.to_string());
                problem.path = bundle.root.clone();
                problem
            },
        )?
    }
}

pub use language::machine;
