struct Mark {
    out: Stdout,
    byte: u8,
}
impl Drop for Mark {
    fn drop(&mut self) {
        self.out.write(&[self.byte]);
    }
}
fn seed(world: &mut World, byte: u8) -> Mark {
    let mut out = world.stdout();
    out.write(b"M");
    Mark {
        out: world.stdout(),
        byte,
    }
}
pub fn main(mut world: World) {
    let mut out = world.stdout();
    out.write(b"A");
    let empty: [Mark; 0] = [seed(&mut world, 48u8); 0];
    out.write(b"B");
    drop(empty);
    let one = [seed(&mut world, 49u8); 1];
    out.write(b"C");
    drop(one);
    out.write(b"D");
}
