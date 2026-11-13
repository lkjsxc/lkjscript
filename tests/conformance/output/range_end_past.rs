pub fn main(mut world: World) {
    let mut out = world.stdout();
    out.write(&[82u8]);
    let bytes=vec![1u8,2,3];let end=4usize;let value=&bytes[..end];out.write(value);
}
