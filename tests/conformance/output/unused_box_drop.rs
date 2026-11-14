struct Mark { out: Stdout, byte: u8 }
impl Drop for Mark {
    fn drop(&mut self) { self.out.write([self.byte].as_slice()); }
}
pub fn main(mut world: World) {
    { let unused = Box::new(Mark { out: world.stdout(), byte: 77u8 }); }
    drop(Box::new(Mark { out: world.stdout(), byte: 81u8 }));
}
