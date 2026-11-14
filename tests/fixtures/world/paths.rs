// One argument names the granted directory. No path in this program escapes it.
fn directory_error(result: Result<Directory, i64>) -> i64 {
    match result {
        Ok(value) => {
            drop(value);
            0
        }
        Err(code) => code,
    }
}
fn file_error(result: Result<File, i64>) -> i64 {
    match result {
        Ok(value) => {
            drop(value);
            0
        }
        Err(code) => code,
    }
}
pub fn main(mut world: World) {
    let args = world.args();
    assert_eq!(args.len(), 1usize);
    let root = String::from_utf8(args[0].clone()).unwrap();
    assert_eq!(directory_error(world.__lkjscript_directory(b"")), 22);
    assert_eq!(
        directory_error(world.__lkjscript_directory(b"missing-final-zero")),
        22
    );
    assert_eq!(directory_error(world.__lkjscript_directory(b"a\0b\0")), 22);
    assert_eq!(directory_error(world.directory("a\0b")), 22);
    let mut directory = world.directory(&root).unwrap();
    assert_eq!(file_error(directory.__lkjscript_open(b"", false)), 22);
    assert_eq!(file_error(directory.__lkjscript_open(b"a\0b\0", false)), 22);
    assert_eq!(file_error(directory.open("a\0b")), 22);
    assert_eq!(file_error(directory.open("../outside")), 18);
    assert_eq!(file_error(directory.open("/outside")), 18);
    assert_eq!(file_error(directory.open("missing")), 2);
    assert_eq!(file_error(directory.open("outward")), 18);
    let mut inward = directory.open("inward").unwrap();
    let content = inward.read_to_end().unwrap();
    assert_eq!(String::from_utf8(content).unwrap(), "inside");
    let mut out = world.stdout();
    out.write(b"paths ok\n");
}
