// expect: reject E_BORROW
// rewrite: Explain the source view lifetime; create the owned copy in a lexical block before replacing the vector.
struct State { bytes: Vec<u8> }
impl State { fn update(&mut self) { self.bytes = self.bytes[1usize..].to_vec(); } }
pub fn probe() { let mut state = State { bytes: vec![1u8, 2u8] }; state.update(); }
