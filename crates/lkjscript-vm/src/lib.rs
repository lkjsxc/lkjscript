//! Stack VM with precise mark-sweep GC and thin host IO builtins.

mod arena;
mod host;
mod host_buf;
mod host_ext;
mod host_term;
mod run;

use lkjscript_core::{Chunk, JitHook, NullJit, Result, Value};

pub use run::Vm;

pub fn run_chunk(chunk: &Chunk) -> Result<Value> {
    run_chunk_with_args(chunk, &[])
}

pub fn run_chunk_with_args(chunk: &Chunk, args: &[String]) -> Result<Value> {
    let mut vm = Vm::new(chunk, NullJit, args.to_vec());
    let out = vm.run();
    host_term::restore_tty();
    out
}

pub fn run_chunk_with_jit<J: JitHook>(chunk: &Chunk, jit: J) -> Result<Value> {
    let mut vm = Vm::new(chunk, jit, Vec::new());
    let out = vm.run();
    host_term::restore_tty();
    out
}
