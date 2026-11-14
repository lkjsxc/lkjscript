// expect: check
// authority-expect: recursive appears once in the reachable-functions section
// authority-expect: stdout_write
fn recursive(output: &mut Stdout, remaining: usize) {
    if remaining == 0 {
        return;
    }
    output.write(b"x");
    recursive(output, remaining - 1);
}
pub fn main(mut send: Stdout) {
    recursive(&mut send, 2);
}
