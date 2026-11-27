//! Rust witness entry over exactly the owned command-line implementation.
#![allow(dead_code, unused_imports)]
pub use lkjscript::{host, proof_domain, seal_header};
pub use lkjscript::{
    ast, authority, child_image, entry_grants, backend, check, diagnostic, diagnostics, frontend, generics, interpreter, ir,
    ir_text, ir_binary, json, names, optimizer, resolve, reuse, runtime, runtime_memory, runtime_diagnostics, sha256, x86,
    semantic, semantic_facts, structure_write, structure_replay, structure_refactor, source_json, trim_source_ascii,
};
// The source compiler is one crate; preserve its crate-local helper access here.
mod elaborate {
    pub use lkjscript::elaborate::*;
    pub use lkjscript::infer_resolved_report_with_source_paths;
}
mod native {
    pub use lkjscript::native::*;
    pub mod records { pub use lkjscript::command_records::*; }
    pub use lkjscript::native_messages as messages;
}
mod structure_edit {
    pub use lkjscript::structure_edit::*;
    pub use lkjscript::structural_error as error;
}
mod source {
    pub use lkjscript::load_report_with_providers;
    pub use lkjscript::source::*;
}
mod syntax {
    pub use lkjscript::macro_provider_name;
    pub use lkjscript::syntax::*;
}
mod partial_check {
    pub use lkjscript::partial_check::*;
    pub use lkjscript::retained_body_failure;
}
include!(concat!(env!("OUT_DIR"), "/embedded-module.rs"));
include!(concat!(env!("OUT_DIR"), "/meaning-module.rs"));
include!(concat!(env!("OUT_DIR"), "/semantic_store-module.rs"));
include!(concat!(env!("OUT_DIR"), "/semantic_query-module.rs"));
include!(concat!(env!("OUT_DIR"), "/units-module.rs"));
include!(concat!(env!("OUT_DIR"), "/residual-module.rs"));
mod host_network {
    use super::host::{Child, File, World};
    include!(concat!(env!("OUT_DIR"), "/source/src/host_network.rs"));
}
mod admitted_image {
    use super::host::{File, World};
    include!(concat!(env!("OUT_DIR"), "/source/src/admitted_image.rs"));
}
mod driver_host {
    use super::host::{Child, Clock, Directory, File, Listener, Process, World};
    include!(concat!(env!("OUT_DIR"), "/source/src/driver_host.rs"));
}
mod store {
    use super::host::{Clock, Directory, File, World};
    include!(concat!(env!("OUT_DIR"), "/source/src/store.rs"));
}
mod program_directory {
    use super::host::World;
    include!(concat!(env!("OUT_DIR"), "/source/src/program_directory.rs"));
}
mod driver {
    use super::host::{Child, Clock, Directory, File, Listener, Process, World};
    include!(concat!(env!("OUT_DIR"), "/source/src/driver.rs"));
}
mod test_runner {
    use super::host::{Child, Clock, Directory, File, Listener, Process, World};
    include!(concat!(env!("OUT_DIR"), "/source/src/test_runner.rs"));
}
mod test_generators {
    use super::host::{Child, Clock, Directory, File, Listener, Process, World};
    include!(concat!(env!("OUT_DIR"), "/source/src/test_generators.rs"));
}
fn main() {
    host::enter(driver::main);
}
