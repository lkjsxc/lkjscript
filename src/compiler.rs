//! The compiler core consumes declared source bytes and returns artifacts or diagnostics.
//! Filesystem access, process startup and bootstrap input packing belong to the host.
pub use super::frontend::{check_bundle, check_source};
use crate::ast::Diagnostic;
use crate::source::SourceBundle;
use crate::{kc, runtime::Runtime};

/// Compile declared source and a checked runtime to native Linux x86-64 bytes.
/// This entry performs no host I/O and has no hidden compilation service.
pub fn compile(bundle: &SourceBundle, runtime: &Runtime) -> Result<Vec<u8>, Diagnostic> {
    let checked = check_bundle(bundle)?;
    let mut entry = "probe";
    for function in &checked.program().functions {
        if function.name == "main" {
            entry = "main";
            break;
        }
    }
    kc::emit(&checked, runtime, entry).map_err(|mut problem| {
        if problem.path.is_empty() {
            problem.path = bundle.root.clone();
        }
        problem
    })
}
