pub fn main(mut world: World) {
    let mut directory = world.directory("/proc/self").unwrap();
    let mut file = directory.open("limits").unwrap();
    let bytes = file.read_to_end().unwrap();
    let mut output = world.stdout();
    output.write(&bytes);
    let mut status = directory.open("status").unwrap();
    let status_bytes = status.read_to_end().unwrap();
    output.write(&status_bytes);
}
