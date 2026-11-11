struct Pair { left: i64, right: i64 }
fn left(pair: &Pair) -> &i64 { &pair.left }
pub fn probe() -> i64 {
    let pair = Pair { left: 42, right: 0 };
    *left(&pair)
}
