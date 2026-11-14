// expect: check
// authority-expect: world:World implicit full authority; world_stdout; stdout_write
pub fn main(mut world: World) {
    world.stdout().write(b"ok\n");
}
