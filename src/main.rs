//! The complete lkjscript toolchain, written in lkjscript.
pub mod ast;
pub mod authority;
pub mod entry_grants;
pub mod backend;
pub mod check;
pub mod diagnostic;
pub mod diagnostics;
pub mod driver;
pub mod driver_host;
pub mod host_network;
pub mod elaborate;
pub mod embedded;
pub(crate) mod facts;
pub mod frontend;
pub mod generics;
pub mod interpreter;
pub mod ir;
pub mod ir_text;
pub mod json;
pub mod names;
pub mod native;
pub mod resolve;
pub mod reuse;
pub mod runtime;
pub mod runtime_memory;
pub use std::sha256;
pub mod source;
pub mod store;
pub mod syntax;
pub mod test_runner;
pub mod test_generators;
pub mod x86;

pub fn main(world: World) {
    driver::main(world);
}
