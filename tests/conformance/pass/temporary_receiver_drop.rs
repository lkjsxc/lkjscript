struct Counter<'a> { count: &'a mut i64 }
impl<'a> Counter<'a> { fn read(&self) -> i64 { 41 } }
impl<'a> Drop for Counter<'a> {
    fn drop(&mut self) { *self.count = *self.count + 1; }
}
pub fn probe() -> i64 {
    let mut count: i64 = 0;
    let value = Counter { count: &mut count }.read();
    count + value
}
