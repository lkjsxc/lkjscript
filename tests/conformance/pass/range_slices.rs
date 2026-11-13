fn window<'a>(items: &'a [i64]) -> &'a [i64] {
    &items[1..3]
}
pub fn probe() -> i64 {
    let values = vec![1i64, 19, 23, 4, 5];
    let first = &values[1..=2];
    assert!(first.len() == 2 && first[1] == 23);
    let second = window(&values[..]);
    assert!(second.len() == 2);
    assert!(values[..2].len() == 2 && values[3..].len() == 2);
    assert!(values[..=0].len() == 1 && values[..].len() == 5);
    assert!(values[5..=4].len() == 0);
    let nested = &values[1..4][..2];
    nested[0] + nested[1]
}
