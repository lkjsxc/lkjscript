// expect: output 646572697665640a
// witness: {}
fn writer(mut world: World) -> Stdout { world.stdout() }
pub fn main(world: World) {
    let mut out: Stdout = writer(world);
    out.write("derived\n".as_bytes());
}
