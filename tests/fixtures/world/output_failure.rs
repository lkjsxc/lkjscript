pub fn main(mut world: World) {
    let arguments = world.args();
    let root = String::from_utf8(arguments[0].clone()).unwrap();
    let mut directory = world.directory(&root).unwrap();
    let mut file = directory.create("owned-file").unwrap();
    file.write_all(b"owned\n").unwrap();
    if arguments[1][0] == b'1' {
        let mut output = world.stdout();
        output.write(b"x");
    } else {
        let mut output = world.stderr();
        output.write(b"x");
    }
}
