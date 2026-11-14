// expect: output
// stderr: 4c4b4a5343524950545f525553545f545241502052303030350a
// status: 101
// witness: {}
pub fn main(mut world: World) {
    let mut stderr = world.stderr();
    stderr.write(b"LKJSCRIPT_RUST_TRAP R0005\n");
    world.set_exit_status(101u8);
}
