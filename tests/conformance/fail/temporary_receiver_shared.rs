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
pub fn probe() -> i64 { let value = make(); shared(&value).next().unwrap() }
