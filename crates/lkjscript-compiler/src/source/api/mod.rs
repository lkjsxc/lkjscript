mod entry;
mod tree;

#[cfg(test)]
pub(crate) use entry::validate_source_set_for_analysis;
pub(crate) use entry::{
    ensure_source_path_for_compiler, load_for_compiler_with_budget, load_for_protocol,
    load_with_metrics_and_budget, rebuild_staged_sources, validate_for_compiler_with_budget,
    LoadMetrics,
};
pub use entry::{load, validate};
pub use tree::ValidatedSourceTree;
