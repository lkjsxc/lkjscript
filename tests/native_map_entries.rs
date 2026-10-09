//! Standalone entry point retains the explicit component-candidate selection.
#[path = "public_cli/native_map_entries.rs"]
mod native_map_entries;

fn binary() -> std::path::PathBuf {
    std::env::var_os("LKJSCRIPT_COMPONENT_CANDIDATE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(env!("CARGO_BIN_EXE_lkjscript")))
}
