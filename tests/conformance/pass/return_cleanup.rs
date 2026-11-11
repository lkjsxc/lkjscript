struct CountDrop<'a> { count: &'a mut i64 }
impl<'a> Drop for CountDrop<'a> {
    fn drop(&mut self) { *self.count = *self.count + 1; }
}
fn early(count: &mut i64, leave: bool) -> i64 {
    let _guard = CountDrop { count };
    if leave { return 7; }
    0
}
pub fn probe() -> i64 {
    let mut count: i64 = 0;
    let value = early(&mut count, true);
    count + value
}
