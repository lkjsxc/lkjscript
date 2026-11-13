pub fn main(mut world: World) {
    let mut out = world.stdout();
    out.write(&[82u8]);
    let bytes=[1u8,2,3];let start=4usize;let value=&bytes[start..];out.write(value);
}
