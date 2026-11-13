struct Handles {
    first: File,
    second: File,
}

pub fn main(mut world: World) {
    let args = world.args();
    assert_eq!(args.len(), 1usize);
    let root = String::from_utf8(args[0].clone()).unwrap();
    let mut directory = world.directory(&root).unwrap();
    let mut iteration = 0usize;
    while iteration < 50usize {
        {
            let mut direct = directory.create("direct").unwrap();
            direct.write_all(b"direct").unwrap();
        }
        {
            let mut files: Vec<File> = Vec::new();
            files.push(directory.create("vector-a").unwrap());
            files.push(directory.create("vector-b").unwrap());
            let mut taken = files.pop().unwrap();
            taken.write_all(b"taken").unwrap();
        }
        {
            let mut boxed = Box::new(directory.create("boxed").unwrap());
            (*boxed).write_all(b"boxed").unwrap();
        }
        {
            let mut pair = Handles {
                first: directory.create("pair-a").unwrap(),
                second: directory.create("pair-b").unwrap(),
            };
            let mut moved = pair.first;
            pair.first = directory.create("replacement").unwrap();
            moved.write_all(b"moved").unwrap();
            drop(moved);
            pair.second.write_all(b"stayed").unwrap();
            pair.first.write_all(b"replacement").unwrap();
        }
        iteration = iteration + 1;
    }
    let mut out = world.stdout();
    out.write(b"cleanup ok\n");
}
