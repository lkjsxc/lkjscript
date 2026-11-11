fn expand<T>(value: T, depth: i64) -> i64 {
    if depth == 0 { 0 } else { expand::<Vec<T>>(vec![value], depth - 1) }
}
pub fn probe() -> i64 { expand::<i64>(42, 1) }
