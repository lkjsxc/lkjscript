// expect: trap R0004
// stdout:
// stderr: 6265666f7265ff80
// witness: {}
pub fn main(mut world: World) {
    let mut errors = world.stderr();
    errors.write(b"before\xff\x80");
    panic!("trap\nRtail");
}
