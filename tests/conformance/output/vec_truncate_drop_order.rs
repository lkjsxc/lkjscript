struct Mark {
    out: Stdout,
    byte: u8,
}
impl Drop for Mark {
    fn drop(&mut self) {
        self.out.write(&[self.byte]);
    }
}
pub fn main(mut world: World) {
    let mut empty: Vec<Mark> = Vec::new();
    empty.truncate(0);
    empty.truncate(5);
    assert!(empty.len() == 0);
    let mut values: Vec<Mark> = Vec::new();
    let mut byte = 1u8;
    while byte <= 5u8 {
        values.push(Mark {
            out: world.stdout(),
            byte,
        });
        byte += 1u8;
    }
    let mut out = world.stdout();
    values.truncate(8);
    assert!(values.len() == 5);
    out.write(&[100u8]);
    values.truncate(2);
    assert!(values.len() == 2);
    assert!(values[0].byte == 1u8 && values[1].byte == 2u8);
    out.write(&[101u8]);
    values.truncate(2);
    values.truncate(0);
    assert!(values.len() == 0);
    out.write(&[102u8]);
    values.truncate(0);
}
