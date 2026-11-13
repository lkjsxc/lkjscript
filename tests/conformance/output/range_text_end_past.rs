pub fn main(mut world: World) {
    let mut out = world.stdout();
    out.write(&[82u8]);
    let text="λ🦀".to_string();let end=9usize;let value=&text[..end];out.write(value.as_bytes());
}
