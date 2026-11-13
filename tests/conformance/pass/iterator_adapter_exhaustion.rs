struct Pulse {
    position: usize,
}
impl Iterator for Pulse {
    type Item = usize;
    fn next(&mut self) -> Option<usize> {
        let position = self.position;
        self.position = position + 1;
        if position == 0 || position == 2 {
            Some(position)
        } else {
            None
        }
    }
}
fn pair(value: Option<(usize, usize)>, first: usize, second: usize) {
    let value = value.unwrap();
    assert_eq!(value.0, first);
    assert_eq!(value.1, second);
}
pub fn probe() -> i64 {
    let mut take = Pulse { position: 0 }.take(3);
    assert_eq!(take.next().unwrap(), 0usize);
    assert!(take.next().is_none());
    assert_eq!(take.next().unwrap(), 2usize);
    assert!(take.next().is_none());
    assert!(take.next().is_none());
    let mut skip = Pulse { position: 0 }.skip(1);
    assert!(skip.next().is_none());
    assert_eq!(skip.next().unwrap(), 2usize);
    let mut enumerate = Pulse { position: 0 }.enumerate();
    pair(enumerate.next(), 0usize, 0usize);
    assert!(enumerate.next().is_none());
    pair(enumerate.next(), 1usize, 2usize);
    let mut zip = Pulse { position: 0 }.zip(Pulse { position: 2 }.take(3));
    pair(zip.next(), 0usize, 2usize);
    assert!(zip.next().is_none());
    assert!(zip.next().is_none());
    let mut chain = Pulse { position: 0 }.chain(Pulse { position: 0 });
    assert_eq!(chain.next().unwrap(), 0usize);
    assert_eq!(chain.next().unwrap(), 0usize);
    assert!(chain.next().is_none());
    assert_eq!(chain.next().unwrap(), 2usize);
    42
}
