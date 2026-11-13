pub fn main(mut world: World) {
    let mut out = world.stdout();
    out.write(&[82u8]);
    let bytes=vec![1u8,2,3];let start=2usize;let end=1usize;let value=&bytes[start..end];out.write(value);
}
