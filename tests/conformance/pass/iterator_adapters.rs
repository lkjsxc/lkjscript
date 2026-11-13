fn enumeration() -> i64 {
    let values = [18i64, 22i64];
    let mut total = 0i64;
    for (index, value) in values.as_slice().iter().enumerate() {
        total = total + index as i64 + *value;
    }
    total + 1
}
fn limiting() -> i64 {
    let values = [1i64, 20i64, 22i64, 9i64];
    let mut total = 0i64;
    for value in values.as_slice().iter().skip(1).take(2) {
        total = total + *value;
    }
    let mut empty = values.as_slice().iter().take(0);
    assert!(empty.next().is_none());
    assert!(empty.next().is_none());
    let mut skipped = values.as_slice().iter().skip(9);
    assert!(skipped.next().is_none());
    assert!(skipped.next().is_none());
    total
}
fn zipped() -> i64 {
    let left = [18i64, 20i64];
    let right = vec![1i64, 3i64, 90i64];
    let mut iterator = left.as_slice().iter().zip(&right);
    let (a, b) = iterator.next().unwrap();
    let (c, d) = iterator.next().unwrap();
    assert!(iterator.next().is_none());
    assert!(iterator.next().is_none());
    drop(iterator);
    *a + *b + *c + *d
}
fn chained() -> i64 {
    let left = [20i64];
    let right = [22i64];
    let mut iterator = left.as_slice().iter().chain(right.as_slice().iter());
    let first = iterator.next().unwrap();
    let second = iterator.next().unwrap();
    assert!(iterator.next().is_none());
    drop(iterator);
    *first + *second
}
pub fn probe() -> i64 {
    assert_eq!(enumeration(), 42);
    assert_eq!(limiting(), 42);
    assert_eq!(zipped(), 42);
    chained()
}
