pub fn main(mut world: World) {
    let mut output = world.stdout();
    let bytes = [b'x'; 8192];
    loop {
        output.write(&bytes);
    }
}
