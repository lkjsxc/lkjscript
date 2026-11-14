//! rustc's view of the source library. I/O uses Rust's canonical Result for `?`;
//! all other modules, including the source Result implementation, stay unchanged.
//! Capability methods come from the real oracle host, never stub declarations.
use lkjscript::source::SourceInput;
use std::path::{Path, PathBuf};

pub fn stage(directory: &Path, inputs: &[SourceInput]) -> PathBuf {
    std::fs::create_dir(directory).unwrap();
    for input in inputs {
        let relative = input.path.strip_prefix("$std/").unwrap();
        let mut text = input.text.clone();
        if relative == "lib.rs" {
            text.push_str("\n#[allow(dead_code, unexpected_cfgs)]\nmod host;\n");
        } else if relative == "io.rs" {
            assert_eq!(text.matches("use crate::Result;").count(), 1);
            text = text.replacen(
                "use crate::Result;",
                "use crate::host::{Child, Clock, Directory, File, Process, Stdin, World};\nuse std::result::Result;",
                1,
            );
        }
        // Preserve every provisioned module and its algorithm bodies. Only name
        // bindings differ where Rust supplies types intrinsic to lkjscript.
        let destination = directory.join(relative);
        std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
        std::fs::write(destination, text).unwrap();
    }
    std::fs::write(
        directory.join("host.rs"),
        include_str!("../../bridge/output.rs"),
    )
    .unwrap();
    directory.join("lib.rs")
}
