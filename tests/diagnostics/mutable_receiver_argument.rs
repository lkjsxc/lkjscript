// expect: reject E_BORROW
// rewrite: Explain receiver-before-argument borrowing; read the scalar field into a fresh local before the mutable call.
struct State { started: usize }
impl State {
    fn set(&mut self, value: usize) { self.started = value; }
    fn update(&mut self) { self.set(self.started); }
}
pub fn probe() { let mut state = State { started: 1usize }; state.update(); }
