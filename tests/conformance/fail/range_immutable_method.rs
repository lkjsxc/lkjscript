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
    let values = [1i64, 2, 3];
    values[..2].touch();
    42
}
