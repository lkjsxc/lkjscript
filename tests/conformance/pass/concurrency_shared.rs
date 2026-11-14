// expect: value 42
// pending: scoped tasks and their ownership and alias checks are not yet implemented
// witness: {}
use std::thread::scope;
pub fn probe() -> i64 {
    let input: i64 = 21;
    scope(|scope| {
        let first = scope.spawn(|| input);
        let second = scope.spawn(|| input);
        first.join().unwrap() + second.join().unwrap()
    })
}
