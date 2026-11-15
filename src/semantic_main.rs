//! Native semantic gate over the compiler's actual source modules.
pub mod ast;
pub mod backend;
pub mod check;
pub mod diagnostic;
pub mod diagnostics;
pub mod elaborate;
pub mod embedded;
pub mod entry_grants;
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
mod semantic_tests;
pub mod source;
pub mod syntax;
pub mod x86;

pub fn main(mut world: World) {
    let args = world.args();
    let shard = if args.is_empty() {
        0usize
    } else {
        String::from_utf8(args[0].clone())
            .unwrap()
            .as_str()
            .parse::<usize>()
            .unwrap()
    };
    let workers = if args.len() < 2 {
        1usize
    } else {
        String::from_utf8(args[1].clone())
            .unwrap()
            .as_str()
            .parse::<usize>()
            .unwrap()
    };
    assert!(workers > 0 && workers <= 8 && shard < workers);
    let (count, errors) = semantic_tests::run(&mut world, shard, workers, args.len() > 2);
    if errors.is_empty() {
        let mut output = world.stdout();
        output.write(format!("ok semantics {count} cases\n").as_bytes());
    } else {
        let mut output = world.stderr();
        for error in errors {
            output.write(error.as_bytes());
            output.write(b"\n");
        }
        world.set_exit_status(1u8);
    }
}
