// expect: output 73746f726167652070726f63657373206f6b0a
// The Rust witness harness selects a case through an environment variable;
// spawn_file clears environment, so self-execution cannot select its witness case.
pub fn main(mut world: World) {
    if !world.args().is_empty() {
        world.stdout().write(b"child");
        return;
    }
    let mut clock = world.clock();
    let name = format!("lkjscript-storage-process-{}", clock.nanoseconds().unwrap());
    let mut temporary = world.directory("/tmp").unwrap();
    temporary.create_directory(&name).unwrap();
    let mut directory = temporary.open_directory(&name).unwrap();
    let mut append = directory.open_append("record").unwrap();
    append.write_all(b"base").unwrap();
    append.rewind().unwrap();
    let executable = world.executable().unwrap();
    let empty = world.temporary_file().unwrap();
    let arguments = vec!["child".as_bytes().to_vec()];
    let mut process = world.process();
    let mut child = process.spawn_file(&executable, &arguments, &empty, &append, &empty, &empty).unwrap();
    assert_eq!(child.wait().unwrap(), 0i64);
    let mut byte = [0u8; 1];
    assert_eq!(append.read(&mut byte).unwrap(), 0usize);
    append.sync().unwrap();
    let mut reader = append.readonly().unwrap();
    assert_eq!(&reader.read_to_end().unwrap()[..], b"basechild");
    drop(reader);
    drop(append);
    directory.remove("record").unwrap();
    directory.sync().unwrap();
    drop(directory);
    temporary.remove_directory(&name).unwrap();
    world.stdout().write(b"storage process ok\n");
}
