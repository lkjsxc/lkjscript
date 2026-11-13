pub fn main(mut world: World) {
    let mut out = world.stdout();
    out.write(&[82u8]);
    let text="λ🦀";let start=4usize;let end=2usize;let value=&text[start..end];out.write(value.as_bytes());
}
