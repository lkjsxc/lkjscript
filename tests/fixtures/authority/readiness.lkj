pub fn main(mut ready: Readiness, mut clock: Clock, mut incoming: Listener) {
    incoming.set_nonblocking(true).unwrap();
    ready.watch_listener(&mut incoming, 0usize).unwrap();
    let _events = ready.wait(&mut clock, 0usize);
    ready.unwatch_listener(&mut incoming).unwrap();
}
