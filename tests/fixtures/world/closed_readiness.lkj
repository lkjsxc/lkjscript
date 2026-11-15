// A closed stderr File is a sentinel, never authority over the still-open stdin.
fn errno(result: Result<(), i64>) -> i64 {
    match result { Ok(()) => 0, Err(error) => error }
}
pub fn main(mut world: World) {
    let mut closed = world.stderr_file().unwrap();
    let mut ready = world.readiness().unwrap();
    let enable = errno(closed.set_nonblocking(true));
    let disable = errno(closed.set_nonblocking(false));
    let watch = errno(ready.watch_file(&mut closed, 1usize, 1u8));
    let unwatch = errno(ready.unwatch_file(&mut closed));
    // Invalid interest is rejected before touching any handle on both routes.
    let invalid = errno(ready.watch_file(&mut closed, 2usize, 4u8));
    let mut input = world.stdin_file().unwrap();
    let mut byte = [0u8; 1];
    assert_eq!(input.read(&mut byte).unwrap(), 1usize);
    assert_eq!(byte[0], b'x');
    world.stdout().write(format!("{enable} {disable} {watch} {unwatch} {invalid}\n").as_bytes());

}
