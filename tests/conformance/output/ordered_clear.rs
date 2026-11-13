use std::collections::BTreeMap;

struct Mark {
    out: Stdout,
    byte: u8,
}
impl Drop for Mark {
    fn drop(&mut self) {
        self.out.write([self.byte].as_slice());
    }
}
pub fn main(mut world: World) {
    let mut values: BTreeMap<i64, Mark> = BTreeMap::new();
    values.insert(2, Mark { out: world.stdout(), byte: b'b' });
    values.insert(1, Mark { out: world.stdout(), byte: b'a' });
    values.clear();
    values.insert(3, Mark { out: world.stdout(), byte: b'c' });
    drop(values);
    let mut out = world.stdout();
    out.write("!".as_bytes());
}
