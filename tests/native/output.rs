pub fn main(mut world: World) {
    let mut output: Stdout = world.stdout();
    output.write("".as_bytes());
    // A zero-length write does not inspect the output descriptor.
    output.write("native\n".as_bytes());
}
