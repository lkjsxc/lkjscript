pub fn main(mut state: Directory, mut clock: Clock, mut output: Stdout, mut status: ExitStatus) {
    let mut lock = state.open_append("lock").unwrap();
    output.write(b"waiting\n");
    lock.lock_wait().unwrap();
    let nanos = clock.unix_nanoseconds().unwrap();
    let seconds = clock.unix_seconds().unwrap();
    assert!(seconds >= 0i64);
    assert!(nanos / 1_000_000_000usize <= seconds as usize);
    assert!(seconds as usize <= nanos / 1_000_000_000usize + 2usize);
    output.write(b"locked\n");
    status.set_status(23u8);
}
