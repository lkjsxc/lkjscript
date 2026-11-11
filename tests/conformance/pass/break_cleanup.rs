struct CountDrop<'a> { count: &'a mut i64 }
impl<'a> Drop for CountDrop<'a> {
    fn drop(&mut self) { *self.count = *self.count + 1; }
}
pub fn probe() -> i64 {
    let mut count: i64 = 0;
    loop {
        let _guard = CountDrop { count: &mut count };
        break;
    }
    count
}
