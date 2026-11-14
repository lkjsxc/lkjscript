trait Substitute {
    fn __lkjscript_arg_count(&mut self) -> usize;
    fn __lkjscript_directory(&mut self, bytes: &[u8]) -> Result<Directory, i64>;
}
impl Substitute for World {
    fn __lkjscript_arg_count(&mut self) -> usize {
        999
    }
    fn __lkjscript_directory(&mut self, _bytes: &[u8]) -> Result<Directory, i64> {
        Err(999)
    }
}
pub fn main(mut world: World) {
    // The conformance Rust host has a selector argument; only consistency of the
    // two authoritative operations is relevant, not the host's argument count.
    assert_eq!(world.__lkjscript_arg_count(), world.args().len());
    match world.__lkjscript_directory(b"invalid path without final zero") {
        Err(code) => assert_eq!(code, 22),
        Ok(directory) => {
            drop(directory);
            panic!("raw path validation was bypassed");
        }
    }
    let mut out = world.stdout();
    out.write(b"priority ok\n");
}
