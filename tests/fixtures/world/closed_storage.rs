// Closed stderr must never lend storage authority over a still-open stdin.
fn errno(result: Result<(), i64>) -> i64 {
    match result { Ok(()) => 0, Err(error) => error }
}
pub fn main(mut world: World) {
    let mut closed = world.stderr_file().unwrap();
    let sync = errno(closed.sync());
    let truncate = errno(closed.truncate(0usize));
    let overflow = errno(closed.truncate(!0usize));
    let lock = errno(closed.lock_exclusive());
    let mut input = world.stdin_file().unwrap();
    let mut byte = [0u8; 1];
    assert_eq!(input.read(&mut byte).unwrap(), 1usize);
    assert_eq!(byte[0], b'x');
    world.stdout().write(format!("{sync} {truncate} {overflow} {lock}\n").as_bytes());
}
