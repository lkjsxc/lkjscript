struct Mark {
    out: Stdout,
    tag: u8,
}
impl Drop for Mark {
    fn drop(&mut self) {
        self.out.write([self.tag].as_slice());
    }
}
fn mark(world: &mut World) -> Mark {
    let mut out = world.stdout();
    out.write([65u8].as_slice());
    Mark { out, tag: 97u8 }
}
fn peek(value: &Mark) {}
pub fn main(mut world: World) {
    let mut count: i64 = 0;
    let owned = mark(&mut world);
    let counter = &mut count;
    let mut update = move || {
        peek(&owned);
        *counter = *counter + 1;
    };
    update();
    let mut out = world.stdout();
    out.write([(count as u8) + 48u8].as_slice());
}
