pub fn main(mut world: World) {
    let mut out = world.stdout();
    out.write(&[82u8]);
    let text="λ🦀";let start=9usize;let value=&text[start..];out.write(value.as_bytes());
}
