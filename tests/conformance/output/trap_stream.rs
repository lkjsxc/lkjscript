pub fn main(mut world: World) {
    let mut errors = world.stderr();
    errors.write(b"before\xff\x80");
    panic!("trap\nRtail");
}
