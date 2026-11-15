// expect: reject E_BORROW
// pending: Explain that the receiver is borrowed before its arguments; suggest computing `self.started` in a local before the call.
struct State { started: usize }
impl State {
    fn set(&mut self, value: usize) { self.started = value; }
    fn update(&mut self) { self.set(self.started); }
}
pub fn probe() { let mut state = State { started: 1usize }; state.update(); }
