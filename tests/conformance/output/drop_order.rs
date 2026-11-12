struct Mark<'a> { out: &'a mut Stdout, byte: u8 }
impl<'a> Drop for Mark<'a> {
    fn drop(&mut self) {
        let bytes: [u8; 1] = [self.byte];
        self.out.write(bytes.as_slice());
    }
}
pub fn main(mut world: World) {
    let mut out: Stdout = world.stdout();
    out.write("a".as_bytes());
    { let mark: Mark<'_> = Mark { out: &mut out, byte: 98u8 }; }
    out.write("c\n".as_bytes());
}
