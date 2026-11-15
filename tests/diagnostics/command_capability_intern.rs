// expect: output 6f6b0a
// pending: An internal capability interning failure must name the missing capability kind and source span, instead of an opaque R0019 and file index.
fn accepts(_args: &mut Arguments, _environment: &mut Environment, _status: &mut ExitStatus) {}
pub fn main(mut world: World) { world.stdout().write(b"ok\n"); }
