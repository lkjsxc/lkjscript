struct Pair<'a, 'b> { left: &'a i64, right: &'b i64 }
fn swap<'a, 'b>(left: &'a i64, right: &'b i64) -> Pair<'a, 'b> {
    Pair { left: right, right: left }
}
pub fn probe() -> i64 {
    let first: i64 = 20;
    let second: i64 = 22;
    let result = swap(&first, &second);
    *result.left + *result.right
}
