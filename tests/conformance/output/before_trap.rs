pub fn main(mut world: World) {
    let mut out: Stdout = world.stdout();
    out.write("before trap\n".as_bytes());
    assert!(false);
}
