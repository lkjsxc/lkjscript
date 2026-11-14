// expect: trap R0004
// stdout: 52
// witness: {}
pub fn main(mut world: World) {
    let mut out = world.stdout();
    out.write(&[82u8]);
    panic!("ordinary user panic");
}
