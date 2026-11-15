// expect: trap R0004
// stdout: 68696768
// witness: {}
pub fn main(mut world: World) {
    let mut out = world.stdout();
    out.write(b"high");
    let _ = '0'.to_digit(37u32);
    out.write(b"after");
}
