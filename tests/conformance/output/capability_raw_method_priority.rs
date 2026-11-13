trait Substitute {
    fn __lkj_arg_count(&mut self) -> usize;
    fn __lkj_directory(&mut self, bytes: &[u8]) -> Result<Directory, i64>;
}
impl Substitute for World {
    fn __lkj_arg_count(&mut self) -> usize {
        999
    }
    fn __lkj_directory(&mut self, _bytes: &[u8]) -> Result<Directory, i64> {
        Err(999)
    }
}
pub fn main(mut world: World) {
    // The conformance Rust host has a selector argument; only consistency of the
    // two authoritative operations is relevant, not the host's argument count.
    assert_eq!(world.__lkj_arg_count(), world.args().len());
    match world.__lkj_directory(b"invalid path without final zero") {
        Err(code) => assert_eq!(code, 22),
        Ok(directory) => {
            drop(directory);
            panic!("raw path validation was bypassed");
        }
    }
    let mut out = world.stdout();
    out.write(b"priority ok\n");
}
