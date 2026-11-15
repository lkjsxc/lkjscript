// expect: reject E_TRAIT
// witness: {"rustAccept":false,"rustCode":"E0592","world":true}
type Held = File;
impl Held {
    fn truncate(&mut self, _length: usize) -> Result<(), i64> { Ok(()) }
}
pub fn main(mut world: World) {
    let mut file = world.temporary_file().unwrap();
    file.truncate(0usize).unwrap();
}
