// expect: reject E_BORROW
// pending: Explain that a temporary view in the match scrutinee remains live through its arms; suggest evaluating it in a separate lexical block first.
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
