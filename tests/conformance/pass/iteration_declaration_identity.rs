enum Option {
    Unrelated,
}
struct Iterator;
mod std {
    pub fn __lkj_next() -> i64 {
        0
    }
}
fn __lkj_into_iter() -> i64 {
    0
}
pub fn probe() -> i64 {
    let values = vec![20i64, 22i64];
    let mut result = 0i64;
    for value in &values {
        result = result + *value;
    }
    result
}
