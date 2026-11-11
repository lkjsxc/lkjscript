use std::thread::scope;
pub fn probe() -> i64 {
    let input: i64 = 21;
    scope(|scope| {
        let first = scope.spawn(|| input);
        let second = scope.spawn(|| input);
        first.join().unwrap() + second.join().unwrap()
    })
}
