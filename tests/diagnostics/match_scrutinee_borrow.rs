// expect: reject E_BORROW
// rewrite: Explain the scrutinee view lifetime; evaluate its owned result in a lexical block before entering the match arms.
struct Request { path: String }
fn parsed(bytes: &[u8]) -> Result<Request, u8> {
    if bytes.is_empty() { Err(0u8) } else { Ok(Request { path: "x".to_string() }) }
}
struct State { bytes: Vec<u8> }
impl State {
    fn clear(&mut self) { self.bytes.clear(); }
    fn update(&mut self) {
        let request = match parsed(&self.bytes[..]) {
            Ok(request) => request,
            Err(_) => return self.clear(),
        };
        drop(request);
    }
}
pub fn probe() { let mut state = State { bytes: Vec::new() }; state.update(); }
