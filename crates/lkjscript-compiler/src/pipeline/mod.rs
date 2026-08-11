mod common;
mod error;
mod path;
mod source;

pub use common::compile_snapshot;
#[cfg(test)]
pub(crate) use common::{
    compile_snapshot_with_metrics, lowering_invocations, reset_lowering_invocations,
    SnapshotCompileMetrics,
};
pub use error::{PackageCompileError, PackageCompileResult};
pub use path::{
    compile_package_path, compile_package_path_with_metrics, compile_path,
    compile_path_with_metrics, compile_path_with_sources,
};
pub use source::{compile_source, validate_source};
