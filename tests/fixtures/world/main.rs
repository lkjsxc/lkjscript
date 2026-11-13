// Args: arbitrary bytes, a UTF-8 directory path, and a UTF-8 file basename.
// Stdout/file: first argument followed by stdin. Stderr: stdin. Exit: 7.
pub fn main(mut world: World) {
    let args = world.args();
    assert_eq!(args.len(), 3usize);
    let payload = args[0].clone();
    let root = String::from_utf8(args[1].clone()).unwrap();
    let name = String::from_utf8(args[2].clone()).unwrap();

    let mut input = world.stdin();
    let mut head = [0u8; 3];
    let count = input.read(&mut head).unwrap();
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&head[..count]);
    let tail = input.read_to_end().unwrap();
    bytes.extend_from_slice(&tail);

    let mut directory = world.directory(&root).unwrap();
    let mut file = directory.create(&name).unwrap();
    let written = file.write(&payload).unwrap();
    file.write_all(&payload[written..]).unwrap();
    file.write_all(&bytes).unwrap();
    drop(file);

    let mut file = directory.open(&name).unwrap();
    let mut head = [0u8; 4];
    let count = file.read(&mut head).unwrap();
    let mut observed = Vec::new();
    observed.extend_from_slice(&head[..count]);
    let tail = file.read_to_end().unwrap();
    observed.extend_from_slice(&tail);
    let mut expected = payload;
    expected.extend_from_slice(&bytes);
    assert_eq!(observed, expected);

    let mut out = world.stdout();
    out.write(&observed);
    let mut error = world.stderr();
    error.write(&bytes);
    world.set_exit_status(7u8);
}
