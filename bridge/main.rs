//! Rust witness entry over exactly the owned command-line implementation.
#![allow(dead_code, unused_imports)]
#[path = "output.rs"]
mod host;
pub use lkjscript::{
    ast, backend, check, diagnostic, diagnostics, elaborate, frontend, generics, interpreter, ir,
    ir_text, json, names, native, resolve, runtime, runtime_memory, source, syntax, x86,
};
#[path = "../src/embedded.rs"]
mod embedded;
mod driver_host {
    use super::host::{Child, Clock, Directory, File, Process, World};
    include!("../src/driver_host.rs");
}
mod driver {
    use super::host::{Child, Clock, Directory, File, Process, World};
    include!("../src/driver.rs");
}
fn main() {
    host::enter(driver::main);
}
