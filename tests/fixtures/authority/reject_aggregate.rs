// expect: check
// authority-reject: E_ENTRY
struct Granted {
    output: Stdout,
}
pub fn main(granted: Granted) {
    drop(granted);
}
