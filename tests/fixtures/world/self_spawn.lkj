// Expected output: 42\n
pub fn main(mut world: World) {
    let arguments = world.args();
    if !arguments.is_empty() {
        if arguments[0] == "child".as_bytes().to_vec() {
            let mut output = world.stdout();
            output.write(b"42\n");
            return;
        }
        if arguments[0] == "unchecked".as_bytes().to_vec() {
            match world.executable() {
                Err(errno) => assert_eq!(errno, 38),
                Ok(file) => {
                    drop(file);
                    panic!("unchecked IR obtained a native executable");
                }
            }
            let mut output = world.stdout();
            output.write(b"unchecked ok\n");
            return;
        }
    }
    let mut first = world.executable().unwrap();
    let mut byte = [0u8; 1];
    assert_eq!(first.read(&mut byte).unwrap(), 1);
    assert_eq!(byte[0], 127u8);
    let mut executable = world.executable().unwrap();
    assert_eq!(executable.read(&mut byte).unwrap(), 1);
    assert_eq!(byte[0], 127u8);
    assert_eq!(first.read(&mut byte).unwrap(), 1);
    assert_eq!(byte[0], 69u8);
    match first.write(b"x") {
        Err(errno) => assert_eq!(errno, 9),
        Ok(_) => panic!("executable handle was writable"),
    }
    drop(first);
    let input = world.temporary_file().unwrap();
    let mut output = world.temporary_file().unwrap();
    let mut errors = world.temporary_file().unwrap();
    let result = world.temporary_file().unwrap();
    let mut process = world.process();
    let arguments = vec!["child".as_bytes().to_vec()];
    let mut child = process.spawn_file(&executable, &arguments, &input, &output, &errors, &result).unwrap();
    assert_eq!(child.wait().unwrap(), 0);
    output.rewind().unwrap();
    errors.rewind().unwrap();
    assert!(errors.read_to_end().unwrap().is_empty());
    let bytes = output.read_to_end().unwrap();
    assert_eq!(bytes, "42\n".as_bytes().to_vec());
    let mut stdout = world.stdout();
    stdout.write(&bytes);
}
