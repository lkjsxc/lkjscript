trait Substitute {
    fn write(&mut self, bytes: &[u8]) -> i64;
}
impl Substitute for Stdout {
    fn write(&mut self, _bytes: &[u8]) -> i64 {
        42
    }
}
pub fn main(mut world: World) {
    let mut out = world.stdout();
    let answer = out.write(&[42u8]);
    let _unit: () = answer;
}
