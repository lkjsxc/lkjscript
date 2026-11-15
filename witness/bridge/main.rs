//! Rust witness entry over exactly the owned command-line implementation.
#![allow(dead_code, unused_imports)]
#[path = "output.rs"]
mod host;
pub use lkjscript::{
    ast, authority, entry_grants, backend, check, diagnostic, diagnostics, elaborate, frontend, generics, interpreter, ir,
    ir_text, ir_binary, json, names, native, optimizer, resolve, reuse, runtime, runtime_memory, runtime_diagnostics, sha256, source, syntax, x86,
};
include!(concat!(env!("OUT_DIR"), "/embedded-module.rs"));
mod host_network {
    use super::host::{Child, File, World};
    include!(concat!(env!("OUT_DIR"), "/source/src/host_network.rs"));
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
