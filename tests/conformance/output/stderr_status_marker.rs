pub fn main(mut world: World) {
    let mut stderr = world.stderr();
    stderr.write(b"LKJSCRIPT_RUST_TRAP R0005\n");
    world.set_exit_status(101u8);
}
