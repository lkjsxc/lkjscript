// expect: value 42
// All bodies are checked and source/IR native bytes must agree, including accept.
fn serve(mut listener: Listener) {
    let mut stream = listener.accept().unwrap();
    stream.write_all(b"ok").unwrap();
}
pub fn probe() -> i64 { 42 }
