struct One { value: i64 }
impl Iterator for One {
    type Item = i64;
    fn next(&mut self) -> Option<i64> {
        let value = self.value;
        self.value = self.value + 1;
        Some(value)
    }
}
struct Pair { first: One, second: One }
fn make() -> One { One { value: 42 } }
fn pair() -> Pair { Pair { first: make(), second: make() } }
fn shared(value: &One) -> &One { value }
fn exclusive(value: &mut One) -> &mut One { value }
pub fn probe() -> i64 {
    if make().next().unwrap() != 42 { return 1; }
    if ({ make() }).next().unwrap() != 42 { return 2; }
    if pair().first.next().unwrap() != 42 { return 3; }
    if [make(), make()][0usize].next().unwrap() != 42 { return 4; }
    let mut value = make();
    let reference = &mut value;
    if reference.next().unwrap() != 42 { return 5; }
    if exclusive(&mut value).next().unwrap() != 43 { return 6; }
    let values = vec![20i64, 22];
    if !values.iter().any(|item| *item == 22) { return 7; }
    42
}
