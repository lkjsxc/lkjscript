mod entry;
mod tree;

#[cfg(test)]
pub(crate) use entry::validate_source_set_for_analysis;
pub(crate) use entry::{
    ensure_source_path_for_compiler, load_for_compiler, load_for_protocol, load_with_metrics,
    rebuild_staged_sources, validate_for_compiler, LoadMetrics,
};
pub use entry::{load, validate};
pub(crate) use tree::ValidatedSourceParts;
pub use tree::ValidatedSourceTree;
