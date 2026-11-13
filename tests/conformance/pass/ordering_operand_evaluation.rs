fn scalar(count: &mut i64) -> i64 {
    *count += 1;
    *count
}
fn vector(count: &mut i64) -> Vec<i64> {
    *count += 1;
    vec![*count]
}
pub fn probe() -> i64 {
    let mut calls = 0i64;
    assert!(scalar(&mut calls) < scalar(&mut calls));
    assert_eq!(calls, 2);
    assert!(vector(&mut calls) < vector(&mut calls));
    assert_eq!(calls, 4);
    let mut first = 1i64;
    assert!(
        first < {
            first = 2;
            first
        }
    );
    assert_eq!(first, 2);
    assert!(&false < &true);
    assert!(&true >= &true);
    let left = vec![1i64, 2i64];
    let right = vec![1i64, 3i64];
    assert!(left.as_slice() < right.as_slice());
    assert!(left.as_slice() <= left.as_slice());
    assert!(left.as_slice() == left.as_slice());
    assert!(left.as_slice() != right.as_slice());
    42
}
