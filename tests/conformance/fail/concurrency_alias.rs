// expect: reject E_BORROW
// pending: scoped tasks and their ownership and alias checks are not yet implemented
// witness: {"rustAccept":false}
use std::thread::scope;
pub fn probe() -> i64 {
    let mut value: i64 = 0;
    scope(|scope| {
        scope.spawn(|| { value = 20; });
        scope.spawn(|| { value = 22; });
    });
    value
}
