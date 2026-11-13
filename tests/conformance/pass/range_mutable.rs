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
    let mut values = vec![0i64, 1, 2, 3, 4];
    values[1..4][0] = 19;
    {
        let part = &mut values[1..3];
        part[1] = 23;
    }
    assert!(values[1] + values[2] == 42);
    values[1..3].touch();
    values[1] + values[2]
}
