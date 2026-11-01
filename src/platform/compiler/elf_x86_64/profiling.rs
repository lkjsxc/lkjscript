//! Opt-in test-only address observation; never changes generated instructions.
use super::*;
use std::io::Write;

pub(super) fn record(emitter: &Emitter<'_>) -> Result<()> {
    let Some(path) = std::env::var_os("LKJSCRIPT_NATIVE_LAYOUT_OUTPUT") else {
        return Ok(());
    };
    let mut functions = Vec::new();
    for (index, label) in emitter.functions.iter().enumerate() {
        if let Some(label) = label {
            let function = emitter
                .program
                .functions
                .get(index)
                .ok_or_else(|| error("native_elf_profile", "observed function is absent"))?;
            functions.push(serde_json::json!({"index":index,
                "declaration":function.declaration.declaration.to_string(),
                "package":function.declaration.package.to_string(),
                "address":emitter.a.label_address(*label)?}));
        }
    }
    let report = serde_json::json!({"schema":"lkjscript.native-address-observation.v1",
        "functions":functions, "allocator":emitter.a.label_address(emitter.runtime.alloc)?,
        "integer":emitter.a.label_address(emitter.runtime.integer)?,
        "boolean":emitter.a.label_address(emitter.runtime.boolean)?});
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| super::error("native_elf_profile", format!("profile output: {error}")))?;
    serde_json::to_writer_pretty(&mut file, &report).map_err(|error| {
        super::error("native_elf_profile", format!("profile encoding: {error}"))
    })?;
    file.flush()
        .map_err(|error| super::error("native_elf_profile", format!("profile flush: {error}")))?;
    Ok(())
}
