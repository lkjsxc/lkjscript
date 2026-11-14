// expect: check
// authority-expect: --grant send; --grant tree=PATH; --grant timer
// authority-expect: stdout_write; clock_read; directory_close
fn emit(output: &mut Stdout) {
    output.write(b"ok\n");
}
fn time(clock: &mut Clock) -> usize {
    clock.nanoseconds().unwrap()
}
pub fn main(mut send: Stdout, tree: Directory, mut timer: Clock) {
    emit(&mut send);
    let _ = time(&mut timer);
    drop(tree);
}
