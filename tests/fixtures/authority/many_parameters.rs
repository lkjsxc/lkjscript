// expect: check
// The last two distinct directories are stack arguments in the native entry ABI.
pub fn main(
    mut first: Stdout,
    mut errors: Stderr,
    mut timer: Clock,
    launch: Process,
    mut input: Stdin,
    mut second: Stdout,
    mut tree: Directory,
    mut other: Directory,
) {
    first.write(b"first|");
    errors.write(b"error\n");
    let _ = timer.nanoseconds().unwrap();
    drop(launch);
    let bytes = input.read_to_end().unwrap();
    second.write(&bytes);
    second.write(b"|");
    let mut left = tree.open("authority-input").unwrap();
    second.write(&left.read_to_end().unwrap());
    second.write(b"|");
    let mut right = other.open("authority-input").unwrap();
    second.write(&right.read_to_end().unwrap());
}
