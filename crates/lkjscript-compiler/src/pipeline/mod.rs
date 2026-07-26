mod common;
mod path;
mod source;

pub use path::{
    compile_path, compile_path_with_ledger, compile_path_with_metrics,
    compile_path_with_metrics_and_ledger, compile_path_with_profile,
    compile_path_with_profile_and_metrics, compile_path_with_sources,
    compile_path_with_sources_and_ledger, compile_path_with_sources_and_profile,
};
pub use source::{
    compile_source, compile_source_with_ledger, compile_source_with_profile, validate_source,
    validate_source_with_ledger, validate_source_with_profile,
};

use std::path::Path;

use lkjscript_core::{Limits, Result};

pub fn validate_source_tree(root: &Path, limits: &Limits) -> Result<()> {
    crate::source::validate_source_directory_tree(root, limits.max_dir_children)
        .map_err(crate::source::SourceDiagnostic::into_core)
}
