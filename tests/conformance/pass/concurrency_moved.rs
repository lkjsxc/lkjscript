use std::thread::scope;
pub fn probe() -> i64 {
    let input = Box::new(42i64);
    scope(|scope| {
        let child = scope.spawn(move || *input);
        child.join().unwrap()
    })
}
