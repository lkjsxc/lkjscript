fn next(count: &mut usize) -> usize {
    let current = *count;
    *count = current + 1;
    current
}
pub fn probe() -> i64 {
    let mut count: usize = 0;
    let values: [i64; 8] = [10, 20, 30, 40, 50, 60, 70, 80];
    let value = values[next(&mut count)] + 2;
    value * 100 + count as i64
}
