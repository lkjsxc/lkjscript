trait Touch {
    fn touch(&mut self);
}
impl Touch for [i64] {
    fn touch(&mut self) {
        self[0] = 40;
        self[1] = 2;
    }
}
pub fn probe() -> i64 {
    let mut values = [1i64, 2, 3];
    values[..values.len()].touch();
    values[0] + values[1]
}
