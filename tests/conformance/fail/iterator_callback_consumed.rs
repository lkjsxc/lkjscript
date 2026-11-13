pub fn probe() -> i64 {
    let values = [1i64, 2];
    let captured = Box::new(42i64);
    let mut iterator = values.as_slice().iter().map(move |_| {
        drop(captured);
        42i64
    });
    iterator.next().unwrap()
}
