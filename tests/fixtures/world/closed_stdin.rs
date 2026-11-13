pub fn main(mut world: World) {
    let args = world.args();
    assert_eq!(args.len(), 1usize);
    let root = String::from_utf8(args[0].clone()).unwrap();
    let mut directory = world.directory(&root).unwrap();
    let mut file = directory.create("closed-input").unwrap();
    let mut input = world.stdin();
    let mut bytes = [0u8; 4];
    match input.read(&mut bytes) {
        Err(code) => assert_eq!(code, 9),
        Ok(_) => panic!("stdin must remain closed"),
    }
    file.write_all(b"still open").unwrap();
    drop(file);
    let mut file = directory.open("closed-input").unwrap();
    assert_eq!(
        String::from_utf8(file.read_to_end().unwrap()).unwrap(),
        "still open"
    );
    let mut out = world.stdout();
    out.write(b"closed ok\n");
}
