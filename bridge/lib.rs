//! Rust bootstrap library host. The source compiler remains rooted at src/lib.rs.
#[path = "runtime.rs"]
pub mod bootstrap_runtime;
pub mod compiler_stack;
mod debug;
#[path = "../src/lib.rs"]
mod language;

pub(crate) use language::facts;
pub use language::{
    ast, backend, check, diagnostic, diagnostics, elaborate, frontend, generics, ir, ir_text, json,
    lower, names, native, resolve, result_wire, runtime_memory, source, syntax,
};

/// Preserve the public compiler boundary while isolating Rust resource setup.
pub mod compiler {
    use crate::ast::{Diagnostic, Span};
    pub use crate::language::compiler::{check_bundle, check_source};
    use crate::source::SourceBundle;

    /// Reserve a native stack from host memory before running the pure pipeline.
    /// The host provisions an explicit source runtime; the pure pipeline reads
    /// only its declared inputs. An owned host replaces this Rust thread wrapper.
    pub fn compile(bundle: &SourceBundle) -> Result<Vec<u8>, Diagnostic> {
        crate::compiler_stack::run(|| {
            let runtime = crate::bootstrap_runtime::get().map_err(|message| {
                let mut problem = Diagnostic::new(Span::default(), "E_RUNTIME", &message);
                problem.path = bundle.root.clone();
                problem
            })?;
            crate::language::compiler::compile(bundle, runtime)
        })
        .map_err(|error| {
            let mut problem = Diagnostic::new(Span::default(), "E_HOST", &error.to_string());
            problem.path = bundle.root.clone();
            problem
        })?
    }
    /// Compile with caller-declared runtime source, preserving the pure boundary.
    pub fn compile_with_runtime(
        bundle: &SourceBundle,
        runtime: &crate::runtime::Runtime,
    ) -> Result<Vec<u8>, Diagnostic> {
        crate::compiler_stack::run(|| crate::language::compiler::compile(bundle, runtime)).map_err(
            |error| {
                let mut problem = Diagnostic::new(Span::default(), "E_HOST", &error.to_string());
                problem.path = bundle.root.clone();
                problem
            },
        )?
    }
}

pub use language::interpreter;

/// Rust bootstrap preparation of the declared runtime source.
/// The backend itself only seals already checked IR.
pub mod runtime {
    pub use crate::language::runtime::*;
    pub use crate::language::runtime_memory::{Memory, MemoryFault};
    pub fn prepare(
        bundle: &crate::source::SourceBundle,
    ) -> Result<Runtime, crate::ast::Diagnostic> {
        seal(crate::compiler::check_bundle(bundle)?)
    }
}
