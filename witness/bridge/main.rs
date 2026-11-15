//! Rust witness entry over exactly the owned command-line implementation.
#![allow(dead_code, unused_imports)]
#[path = "output.rs"]
mod host;
pub use lkjscript::{
    ast, authority, entry_grants, backend, check, diagnostic, diagnostics, elaborate, frontend, generics, interpreter, ir,
    ir_text, json, names, native, resolve, runtime, runtime_memory, source, syntax, x86,
};
#[path = "../../src/embedded.rs"]
mod embedded;
mod host_network {
    use super::host::{Child, File, World};
    include!("../../src/host_network.rs");
}
mod driver_host {
    use super::host::{Child, Clock, Directory, File, Listener, Process, World};
    include!("../../src/driver_host.rs");
}
mod driver {
    use super::host::{Child, Clock, Directory, File, Listener, Process, World};
    include!("../../src/driver.rs");
}
mod test_runner {
    use super::host::{Child, Clock, Directory, File, Listener, Process, World};
    include!("../../src/test_runner.rs");
}
mod test_generators {
    use super::host::{Child, Clock, Directory, File, Listener, Process, World};
    include!("../../src/test_generators.rs");
}
fn main() {
    host::enter(driver::main);
}
