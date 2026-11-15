// expect: output 6f6b0a
// Diagnostic: Arguments, Environment and ExitStatus are interned even in an uncalled declaration; regression for the resolved internal trap.
fn accepts(_args: &mut Arguments, _environment: &mut Environment, _status: &mut ExitStatus) {}
pub fn main(mut world: World) { world.stdout().write(b"ok\n"); }
