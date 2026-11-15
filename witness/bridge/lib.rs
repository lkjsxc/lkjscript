//! Rust bootstrap library host. The source compiler remains rooted at src/lib.lkj.
#[path = "runtime.rs"]
pub mod bootstrap_runtime;
mod debug;
#[path = "output.rs"]
pub mod host;
include!(concat!(env!("OUT_DIR"), "/source-json-module.rs"));
include!(concat!(env!("OUT_DIR"), "/language-module.rs"));

pub(crate) use language::facts;
pub use language::{
    ast, authority, child_image, entry_grants, backend, check, diagnostic, diagnostics, elaborate, frontend, generics, ir, ir_text, ir_binary, json,
    names, native, optimizer, resolve, reuse, runtime_memory, runtime_diagnostics, sha256, source, syntax,
    meaning, semantic, semantic_facts, structure_edit, structure_write, structure_replay, structure_refactor,
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

/// The source library trims bytes 9 through 13 and 32, including vertical tab.
pub fn trim_source_ascii(value: &str) -> &str {
    let bytes = value.as_bytes();
    let mut start = 0usize;
    let mut end = bytes.len();
    while start < end {
        let byte = bytes[start];
        if byte != 32u8 && (byte < 9u8 || byte > 13u8) { break; }
        start += 1usize;
    }
    while end > start {
        let byte = bytes[end - 1usize];
        if byte != 32u8 && (byte < 9u8 || byte > 13u8) { break; }
        end -= 1usize;
    }
    &value[start..end]
}
