//! Rust bootstrap library host. The source compiler remains rooted at src/lib.rs.
#[path = "runtime.rs"]
pub mod bootstrap_runtime;
pub mod compiler_stack;
mod debug;
#[path = "../src/lib.rs"]
mod language;

pub(crate) use language::facts;
pub use language::{
    ast, check, diagnostics, elaborate, encoding, generics, json, k, k_text, lower, names, native,
    resolve, runtime, source, syntax,
};

/// Bootstrap convenience; the pure encoder only consumes an explicit lowered program.
pub mod wasm {
    pub use crate::language::wasm::encode;
    pub fn emit(checked: &crate::k::CheckedProgram) -> Result<Vec<u8>, String> {
        encode(&crate::lower::lower(
            checked,
            crate::bootstrap_runtime::get()?,
        )?)
    }
}

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

pub use language::machine;
