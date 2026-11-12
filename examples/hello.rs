pub fn main(mut world: World) {
    let mut out: Stdout = world.stdout();
    out.write("Hello from lkjscript.\n".as_bytes());
    let bytes: [u8; 4] = [0u8, 255u8, 65u8, 10u8];
    out.write(bytes.as_slice());
}
