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
struct Capture<'a> {
    owned: Mark,
    counter: &'a mut i64,
}
fn invoke(value: &mut Capture<'_>) {
    peek(&value.owned);
    *value.counter = *value.counter + 1;
}
pub fn main(mut world: World) {
    let mut count: i64 = 0;
    let owned = mark(&mut world);
    let counter = &mut count;
    let mut update = Capture { owned, counter };
    invoke(&mut update);
    drop(update.counter);
    let mut out = world.stdout();
    out.write([(count as u8) + 48u8].as_slice());
}
