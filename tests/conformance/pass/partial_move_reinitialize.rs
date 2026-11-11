struct Pair { left: Box<i64>, right: Box<i64> }
pub fn probe() -> i64 {
    let mut pair = Pair { left: Box::new(10i64), right: Box::new(20i64) };
    let first = pair.left;
    pair.left = Box::new(12i64);
    *first + *pair.left + *pair.right
}
