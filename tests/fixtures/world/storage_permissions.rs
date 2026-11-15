// The runner grants a mode0300 directory and closes stdin before this entry.
fn errno<T>(result: Result<T, i64>, expected: i64) {
    match result {
        Err(error) => assert_eq!(error, expected),
        Ok(_) => panic!("expected an I/O error"),
    }
}
pub fn main(mut world: World) {
    let arguments = world.args();
    let path = String::from_utf8(arguments[0].clone()).unwrap();
    let mut directory = world.directory(&path).unwrap();
    let mut file = directory.create_new("record").unwrap();
    file.write_all(b"first").unwrap();
    file.sync().unwrap();
    drop(file);
    errno(directory.create_new("record"), 17);
    let mut append = directory.open_append("record").unwrap();
    assert_eq!(&append.read_to_end().unwrap()[..], b"first");
    append.write_all(b"second").unwrap();
    append.sync().unwrap();
    append.rewind().unwrap();
    assert_eq!(&append.read_to_end().unwrap()[..], b"firstsecond");
    drop(append);
    let mut target = world.directory(&path).unwrap();
    assert_eq!(directory.__lkjscript_rename(b"record\0", &mut target, b"moved\0"), 0i64);
    assert_eq!(directory.__lkjscript_remove(b"moved\0", false), 0i64);
    assert_eq!(directory.__lkjscript_create_directory(b"child\0"), 0i64);
    let mut child = directory.open_directory("child").unwrap();
    assert!(child.list().unwrap().is_empty());
    child.sync().unwrap();
    drop(child);
    assert_eq!(directory.__lkjscript_remove(b"child\0", true), 0i64);
    errno(directory.sync(), 13);
    errno(directory.list(), 13);
    let mut closed = world.stdin_file().unwrap();
    errno(closed.sync(), 9);
    world.stdout().write(b"storage permissions ok\n");
}
