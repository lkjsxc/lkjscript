#[derive(Clone, Copy)]
struct Point { x: i64, y: i64 }
pub fn probe() -> i64 {
    let point = Point { x: 20, y: 22 };
    let other = point;
    point.x + other.y
}
