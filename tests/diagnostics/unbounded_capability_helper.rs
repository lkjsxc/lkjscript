// expect: reject E_UNSUPPORTED
// pending: Name the required affine Arguments receiver for __lkjscript_arg_count; an opaque $rigid type and implementation count do not explain this capability-only operation.
fn command_args<T>(arguments: &mut T) -> usize { arguments.__lkjscript_arg_count() }
fn main() {}
