struct Triple<T>(T,T,T);
pub fn probe() -> i64 {
    let pair = (20i64,22i64);
    let (left,right) = &pair;
    assert!(*left + *right == 42);
    let mut value = (19i64,99i64,21i64);
    { let (first, .., last) = &mut value; *first = *first+1; *last = *last+1; }
    let nested = Some(Triple(1i64,2i64,3i64));
    let sum = match &nested { Some(Triple(first,..,last)) => *first+*last, None=>0 };
    assert!(sum == 4);
    value.0 + value.2
}
