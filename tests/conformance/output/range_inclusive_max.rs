pub fn main(mut world: World) {
    let mut out = world.stdout();
    out.write(&[82u8]);
    let bytes=[1u8,2,3];let end=18446744073709551615usize;let value=&bytes[..=end];out.write(value);
}
