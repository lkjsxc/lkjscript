// A closed standard stream remains closed through derived Files and spawning.
pub fn main(mut world: World) {
    let arguments = world.args();
    let which = (arguments[0][0] - b'0') as usize;
    let mut input = world.stdin_file().unwrap();
    let mut output = world.stdout_file().unwrap();
    let mut errors = world.stderr_file().unwrap();
    let result = world.temporary_file().unwrap();
    let mut closed = if which == 0 { input.duplicate().unwrap() }
        else if which == 1 { output.duplicate().unwrap() }
        else { errors.duplicate().unwrap() };
    let mut bytes = [0u8; 1];
    match closed.read(&mut bytes) {
        Err(code) => assert_eq!(code, 9),
        Ok(_) => panic!("derived stream must remain closed"),
    }
    assert!(closed.write_all(b"x").is_err());
    assert!(closed.rewind().is_err());
    assert!(closed.set_executable().is_err());
    drop(closed);
    let mut directory = world.directory("/bin").unwrap();
    let executable = directory.open("sh").unwrap();
    let script = format!("test ! -e /proc/self/fd/{which}");
    let arguments = vec!["-c".as_bytes().to_vec(), script.as_bytes().to_vec()];
    let mut process = world.process();
    let mut child = process.spawn_file(&executable, &arguments, &input, &output, &errors, &result).unwrap();
    assert_eq!(child.wait().unwrap(), 0);
}
