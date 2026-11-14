// expect: check
// authority-reject: E_ENTRY
pub fn main(output: &mut Stdout) {
    output.write(b"ungrantable\n");
}
