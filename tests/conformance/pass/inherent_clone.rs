#[derive(Copy, Clone)]
struct Thing { value: i64 }
impl Thing {
    fn clone(&self) -> Thing { Thing { value: 42 } }
}
pub fn probe() -> i64 { Thing { value: 1 }.clone().value }
