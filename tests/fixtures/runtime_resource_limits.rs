pub fn main(mut world: World) {
    let mut directory = world.directory("/proc/self").unwrap();
    let mut file = directory.open("limits").unwrap();
    let bytes = file.read_to_end().unwrap();
    let mut output = world.stdout();
    output.write(&bytes);
}
