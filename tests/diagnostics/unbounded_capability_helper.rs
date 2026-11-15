// expect: reject E_UNSUPPORTED
// Diagnostic: Raw argument operations name the affine Arguments receiver and concrete helper rewrite; an unbounded T grants no capability.
fn command_args<T>(arguments: &mut T) -> usize { arguments.__lkjscript_arg_count() }
fn main() {}
