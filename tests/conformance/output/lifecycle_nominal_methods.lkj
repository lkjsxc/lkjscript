// expect: output 6e6f6d696e616c206f6b0a
// witness: {}
struct Ordinary { value: usize }
impl Ordinary {
    fn entropy(&mut self) -> usize { self.value }
    fn fill(&mut self, value: usize) { self.value = value; }
    fn stop(&mut self) -> usize { self.value }
    fn target(&mut self) -> usize { self.value }
    fn requested(&mut self) -> bool { false }
    fn create_private(&mut self, value: usize) { self.value = value; }
}
pub fn main(mut world: World) {
    let mut value = Ordinary { value: 4 };
    assert_eq!(value.entropy(), 4usize);
    value.fill(9usize);
    assert_eq!(value.stop(), 9usize);
    assert_eq!(value.target(), 9usize);
    assert!(!value.requested());
    value.create_private(12usize);
    assert_eq!(value.target(), 12usize);
    world.stdout().write(b"nominal ok\n");
}
