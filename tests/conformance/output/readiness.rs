// expect: output 72656164696e657373206f6b0a
// witness: {}
fn errno<T>(result: Result<T, i64>, expected: i64) {
    match result {
        Err(error) => assert_eq!(error, expected),
        Ok(_) => panic!("expected an I/O error"),
    }
}
fn seen(events: &[(usize, u8)], token: usize, flags: u8) -> bool {
    for event in events {
        if event.0 == token && event.1 & flags == flags { return true; }
    }
    false
}
fn wait_for(ready: &mut Readiness, clock: &mut Clock, token: usize, flags: u8) {
    let deadline = clock.nanoseconds().unwrap() + 1000000000usize;
    loop {
        let events = ready.wait(clock, deadline).unwrap();
        if seen(&events, token, flags) { return; }
        assert!(clock.nanoseconds().unwrap() < deadline);
    }
}
pub fn main(mut world: World) {
    let mut clock = world.clock();
    let mut ready = world.readiness().unwrap();
    let mut listener = world.listen("127.0.0.1", 0u16).unwrap();
    listener.set_nonblocking(true).unwrap();
    errno(listener.accept(), 11);
    ready.watch_listener(&mut listener, 9usize).unwrap();
    assert!(ready.wait(&mut clock, 0usize).unwrap().is_empty());
    let mut target = world.connector("127.0.0.1", listener.port().unwrap()).unwrap();
    let mut stream = target.connect().unwrap();
    wait_for(&mut ready, &mut clock, 9usize, 1u8);
    let mut accepted = listener.accept_nodelay().unwrap();
    ready.unwatch_listener(&mut listener).unwrap();
    errno(ready.unwatch_listener(&mut listener), 2);
    accepted.set_nonblocking(true).unwrap();
    let mut bytes = [0u8; 4];
    errno(accepted.read(&mut bytes), 11);
    ready.watch_file(&mut accepted, 10usize, 1u8).unwrap();
    stream.write_all(b"ping").unwrap();
    wait_for(&mut ready, &mut clock, 10usize, 1u8);
    assert!(seen(&ready.wait(&mut clock, 0usize).unwrap(), 10usize, 1u8));
    let mut at = 0usize;
    while at < bytes.len() {
        wait_for(&mut ready, &mut clock, 10usize, 1u8);
        at += accepted.read(&mut bytes[at..]).unwrap();
    }
    assert_eq!(&bytes[..], b"ping");
    ready.watch_file(&mut accepted, 11usize, 2u8).unwrap();
    assert!(seen(&ready.wait(&mut clock, 0usize).unwrap(), 11usize, 2u8));
    errno(ready.watch_file(&mut accepted, 12usize, 4u8), 22);
    assert!(seen(&ready.wait(&mut clock, 0usize).unwrap(), 11usize, 2u8));
    ready.watch_file(&mut accepted, 12usize, 0u8).unwrap();
    assert!(ready.wait(&mut clock, 0usize).unwrap().is_empty());
    ready.unwatch_file(&mut accepted).unwrap();
    errno(ready.unwatch_file(&mut accepted), 2);
    let deadline = clock.nanoseconds().unwrap() + 2000000usize;
    assert!(ready.wait(&mut clock, deadline).unwrap().is_empty());
    assert!(clock.nanoseconds().unwrap() >= deadline);
    let mut regular = world.temporary_file().unwrap();
    regular.set_nonblocking(true).unwrap();
    regular.set_nonblocking(false).unwrap();
    errno(ready.watch_file(&mut regular, 13usize, 1u8), 1);
    ready.watch_file(&mut accepted, 14usize, 1u8).unwrap();
    let alias = accepted.duplicate().unwrap();
    drop(accepted);
    // Closing the registered descriptor keeps its watch while an alias lives.
    stream.write_all(b"alias").unwrap();
    wait_for(&mut ready, &mut clock, 14usize, 1u8);
    drop(alias);
    assert!(ready.wait(&mut clock, 0usize).unwrap().is_empty());
    drop(stream);
    let mut stream = target.connect().unwrap();
    let mut accepted = listener.accept().unwrap();
    ready.watch_file(&mut accepted, 15usize, 1u8).unwrap();
    drop(ready);
    // Destroying the set consumes no watched stream authority.
    stream.write_all(b"live").unwrap();
    accepted.set_nonblocking(false).unwrap();
    let mut at = 0usize;
    while at < bytes.len() { at += accepted.read(&mut bytes[at..]).unwrap(); }
    assert_eq!(&bytes[..], b"live");
    let mut ready = world.readiness().unwrap();
    ready.watch_file(&mut accepted, 16usize, 1u8).unwrap();
    drop(stream);
    wait_for(&mut ready, &mut clock, 16usize, 4u8);
    assert_eq!(accepted.read(&mut bytes).unwrap(), 0usize);
    ready.unwatch_file(&mut accepted).unwrap();
    drop(accepted);
    // A batch is an allocation chunk, never a connection-count limit.
    let mut clients: Vec<File> = Vec::new();
    let mut servers: Vec<File> = Vec::new();
    for index in 0usize..65usize {
        clients.push(target.connect().unwrap());
        let mut server = listener.accept().unwrap();
        ready.watch_file(&mut server, index, 2u8).unwrap();
        servers.push(server);
    }
    let mut count = 0usize;
    while count < 65usize {
        let events = ready.wait(&mut clock, 0usize).unwrap();
        assert!(!events.is_empty());
        for event in events {
            assert!(event.1 & 2u8 != 0u8);
            ready.unwatch_file(&mut servers[event.0]).unwrap();
            count += 1;
        }
    }
    assert!(ready.wait(&mut clock, 0usize).unwrap().is_empty());
    world.stdout().write(b"readiness ok\n");
}
