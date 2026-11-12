fn write(out: &mut Stdout, bytes: &[u8]) { out.write(bytes); }
pub fn main(mut world: World) {
    let writer: Stdout = world.stdout();
    let mut out: Stdout = writer;
    out.write("Hello, 世界!\n".as_bytes());
    out.write("".as_bytes());
    let bytes: [u8; 4] = [0u8, 255u8, 13u8, 10u8];
    write(&mut out, bytes.as_slice());
    let mut values: Vec<u8> = Vec::new();
    let mut i: usize = 0;
    while i < 256 { values.push(i as u8); i = i + 1; }
    out.write(values.as_slice());
}
