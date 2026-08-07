mod entry;
mod tree;

#[cfg(test)]
pub(crate) use entry::validate_source_set_for_analysis;
pub(crate) use entry::{
    ensure_source_path_for_compiler, load_with_metrics, validate_for_compiler, LoadMetrics,
};
pub(crate) use entry::{load, validate};
pub(crate) use tree::ValidatedSourceParts;
pub(crate) use tree::ValidatedSourceTree;
