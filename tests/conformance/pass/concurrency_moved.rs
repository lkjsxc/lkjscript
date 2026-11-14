// expect: value 42
// pending: scoped tasks and their ownership and alias checks are not yet implemented
// witness: {}
use std::thread::scope;
pub fn probe() -> i64 {
    let input = Box::new(42i64);
    scope(|scope| {
        let child = scope.spawn(move || *input);
        child.join().unwrap()
    })
}
