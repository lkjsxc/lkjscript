pub fn main(mut world: World) {
    let mut out = world.stdout();
    out.write(b"low");
    let _ = '0'.to_digit(1u32);
    out.write(b"after");
}
