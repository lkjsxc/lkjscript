//! Rust bootstrap provisioning of the explicitly declared source runtime.
//! The pure compiler receives Runtime as an input and never loads these bytes.
use crate::runtime::{self, Runtime};
use crate::source::{SourceBundle, SourceInput};
use std::sync::OnceLock;

pub fn get() -> Result<&'static Runtime, String> {
    static RUNTIME: OnceLock<Result<Runtime, String>> = OnceLock::new();
    RUNTIME
        .get_or_init(|| {
            let bundle = SourceBundle {
                root: "$runtime/runtime.rs".to_string(),
                library: None,
                files: vec![SourceInput {
                    path: "$runtime/runtime.rs".to_string(),
                    text: include_str!("../library/runtime.rs").to_string(),
                }],
            };
            runtime::prepare(&bundle).map_err(|error| {
                format!(
                    "{}:{}:{}: {} {}",
                    error.path, error.line, error.column, error.code, error.message
                )
            })
        })
        .as_ref()
        .map_err(Clone::clone)
}

/// Reservations are host resource inputs, shared by every emitted native image.
pub fn resources() -> Result<crate::native::Resources, String> {
    let extent = crate::compiler_stack::reservation().map_err(|error| error.to_string())? as u64;
    let extent = extent & !65535;
    Ok(crate::native::Resources {
        stack_bytes: extent,
        heap_bytes: extent,
    })
}
