// expect: trap R0005
// stdout: 6265666f726520747261700a
// witness: {}
pub fn main(mut world: World) {
    let mut out: Stdout = world.stdout();
    out.write("before trap\n".as_bytes());
    assert!(false);
}
