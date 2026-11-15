// expect: reject E_MOVE
// rewrite: Pass the mutable parameter as &mut *action on each generic call so the loop does not move its original reference.
trait Action { fn apply(&mut self); }
struct Counter { value: usize }
impl Action for Counter { fn apply(&mut self) { self.value += 1usize; } }
struct Runner {}
impl Runner {
    fn tick<T: Action>(&mut self, action: &mut T) { action.apply(); }
    fn repeat<T: Action>(&mut self, action: &mut T) {
        for _step in 0usize..2usize { self.tick(action); }
    }
}
pub fn probe() -> usize { let mut runner = Runner {}; let mut count = Counter { value: 0usize }; runner.repeat(&mut count); count.value }
