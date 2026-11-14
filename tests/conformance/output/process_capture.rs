fn copy(world: &mut World, program: &File, bytes: &[u8], output: &File, errors: &File) {
    let mut input = world.temporary_file().unwrap();
    input.write_all(bytes).unwrap();
    input.rewind().unwrap();
    let result = world.temporary_file().unwrap();
    let arguments: Vec<Vec<u8>> = Vec::new();
    let mut process = world.process();
    let mut child = process
        .spawn_file(program, &arguments, &input, output, errors, &result)
        .unwrap();
    assert_eq!(child.wait().unwrap(), 0i64);
}

pub fn main(mut world: World) {
    let mut tools = world.directory("/usr/bin").unwrap();
    let program = tools.open("cat").unwrap();
    let output = world.stdout_file().unwrap().duplicate().unwrap();
    let errors = world.stderr_file().unwrap().duplicate().unwrap();
    world.stdout().write(b"before|");
    world.stderr().write(b"before|");
    copy(&mut world, &program, b"child-out", &output, &errors);
    copy(&mut world, &program, b"child-err", &errors, &errors);
    world.stdout().write(b"|after");
    world.stderr().write(b"|after");
}
