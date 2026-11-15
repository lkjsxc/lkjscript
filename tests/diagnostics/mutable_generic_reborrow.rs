// expect: reject E_MOVE
// pending: Explain that this generic mutable argument needs an explicit reborrow `&mut *action` when passed repeatedly, or infer that reborrow.
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
