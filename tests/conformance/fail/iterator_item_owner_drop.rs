fn next<I: Iterator>(iterator: &mut I) -> Option<I::Item> {
    iterator.next()
}
pub fn probe() -> i64 {
    let values = vec![42i64];
    let mut iterator = values.iter();
    let first = next(&mut iterator).unwrap();
    drop(iterator);
    drop(values);
    *first
}
