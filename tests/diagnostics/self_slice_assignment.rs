// expect: reject E_BORROW
// pending: Explain that the source slice temporary outlives this assignment; suggest creating the owned copy in a separate lexical block before replacing the vector.
struct State { bytes: Vec<u8> }
impl State { fn update(&mut self) { self.bytes = self.bytes[1usize..].to_vec(); } }
pub fn probe() { let mut state = State { bytes: vec![1u8, 2u8] }; state.update(); }
