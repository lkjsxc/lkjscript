struct Pair { left: i64, right: i64 }
pub fn probe() -> i64 {
    let mut pair = Pair { left: 0, right: 0 };
    let left = &mut pair.left;
    let right = &mut pair.right;
    *left = 20;
    *right = 22;
    pair.left + pair.right
}
