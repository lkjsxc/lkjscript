pub fn main(mut world: World) {
    let mut stdout = world.stdout();
    stdout.write(b"\0\xff");
    let mut stderr = world.stderr();
    stderr.write(b"\xff\0\x80");
    world.set_exit_status(23u8);
}
