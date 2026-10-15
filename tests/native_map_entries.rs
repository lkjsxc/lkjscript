//! Standalone entry point retains the explicit component-candidate selection.
#[path = "public_cli/native_map_entries.rs"]
mod native_map_entries;
#[allow(
    dead_code,
    clippy::unwrap_used,
    reason = "shared semantic CLI test support uses panic-on-failure assertions"
)]
#[path = "support/semantic_cli.rs"]
mod semantic_cli;
use native_map_entries::support::copy_executable;

fn binary() -> std::path::PathBuf {
    std::env::var_os("LKJSCRIPT_COMPONENT_CANDIDATE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(env!("CARGO_BIN_EXE_lkjscript")))
}
