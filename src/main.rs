//! The complete lkjscript toolchain, written in lkjscript.
pub mod ast;
pub mod backend;
pub mod check;
pub mod diagnostic;
pub mod diagnostics;
pub mod driver;
pub mod driver_host;
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
pub mod runtime;
pub mod runtime_memory;
pub mod source;
pub mod syntax;
pub mod x86;

pub fn main(world: World) {
    driver::main(world);
}
