//! Rust bootstrap library host. The source compiler remains rooted at src/lib.rs.
#[path = "runtime.rs"]
pub mod bootstrap_runtime;
mod debug;
#[path = "../src/lib.rs"]
mod language;

pub(crate) use language::facts;
pub use language::{
    ast, backend, check, diagnostic, diagnostics, elaborate, frontend, generics, ir, ir_text, json,
    names, native, resolve, runtime_memory, source, syntax,
};

/// Rust witness provisioning for the pure compiler.
pub mod compiler {
    use crate::ast::{Diagnostic, Span};
    pub use crate::language::compiler::{check_bundle, check_source};
    use crate::source::SourceBundle;
    pub fn compile(bundle: &SourceBundle) -> Result<Vec<u8>, Diagnostic> {
        let runtime = crate::bootstrap_runtime::get().map_err(|message| {
            let mut problem = Diagnostic::new(Span::default(), "E_RUNTIME", &message);
            problem.path = bundle.root.clone();
            problem
        })?;
        crate::language::compiler::compile(bundle, runtime)
    }
    pub fn compile_with_runtime(
        bundle: &SourceBundle,
        runtime: &crate::runtime::Runtime,
    ) -> Result<Vec<u8>, Diagnostic> {
        crate::language::compiler::compile(bundle, runtime)
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

pub use language::x86;
