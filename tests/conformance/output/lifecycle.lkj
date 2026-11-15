// expect: output 6c6966656379636c65206f6b0a
// witness: {}
fn send_stop(process: &mut Process, shell: &File, empty: &File, target: u32) {
    // Minimal images provide kill as a shell builtin without a /bin/kill file.
    let arguments = vec!["-c".as_bytes().to_vec(), format!("kill -TERM {target}").into_bytes()];
    let mut child = process.spawn_file(shell, &arguments, empty, empty, empty, empty).unwrap();
    assert_eq!(child.wait().unwrap(), 0i64);
}
fn errno<T>(result: Result<T, i64>, expected: i64) {
    match result { Err(error) => assert_eq!(error, expected), Ok(_) => panic!("expected errno") }
}
pub fn main(mut world: World) {
    let mut entropy = world.entropy();
    let mut empty_bytes = [0u8; 0];
    entropy.fill(&mut empty_bytes).unwrap();
    let mut bytes = vec![0u8; 65539];
    bytes[0] = 17;
    bytes[65538] = 29;
    entropy.fill(&mut bytes[1..65538]).unwrap();
    assert_eq!(bytes[0], 17u8);
    assert_eq!(bytes[65538], 29u8);
    let mut changed = false;
    for byte in &bytes[1..65538] { if *byte != 0 { changed = true; } }
    assert!(changed);
    let mut clock = world.clock();
    assert!(clock.unix_seconds().unwrap() > 0i64);

    let mut first = world.stop().unwrap();
    let mut second = world.stop().unwrap();
    let target = first.target().unwrap();
    assert!(target > 0u32);
    assert_eq!(target, second.target().unwrap());
    assert!(!first.requested().unwrap());
    assert!(!second.requested().unwrap());
    let mut ready = world.readiness().unwrap();
    ready.watch_stop(&mut first, 1usize).unwrap();
    ready.watch_stop(&mut first, 2usize).unwrap();
    ready.watch_stop(&mut second, 3usize).unwrap();
    assert!(ready.wait(&mut clock, 0usize).unwrap().is_empty());
    let mut bin = world.directory("/bin").unwrap();
    let shell = bin.open("sh").unwrap();
    let empty = world.temporary_file().unwrap();
    let mut process = world.process();
    send_stop(&mut process, &shell, &empty, target);
    let deadline = clock.nanoseconds().unwrap() + 1000000000usize;
    let events = ready.wait(&mut clock, deadline).unwrap();
    assert!(!events.is_empty());
    for event in &events { assert!(event.0 == 2usize || event.0 == 3usize); assert_eq!(event.1 & 1u8, 1u8); }
    assert!(first.requested().unwrap());
    assert!(first.requested().unwrap());
    assert!(!second.requested().unwrap()); // The process signal queue is not broadcast.
    assert!(ready.wait(&mut clock, 0usize).unwrap().is_empty());
    ready.unwatch_stop(&mut first).unwrap();
    errno(ready.unwatch_stop(&mut first), 2);
    drop(first);
    send_stop(&mut process, &shell, &empty, target);
    assert!(second.requested().unwrap());
    drop(second);
    assert!(ready.wait(&mut clock, 0usize).unwrap().is_empty());

    // Dropping the last Stop does not release the execution's SIGTERM claim.
    send_stop(&mut process, &shell, &empty, target);
    let mut final_stop = world.stop().unwrap();
    assert!(final_stop.requested().unwrap());
    drop(ready);
    assert!(final_stop.requested().unwrap());
    // A child without Stop recovers the pre-claim unblocked SIGTERM bit.
    // Some distributions put sleep outside /bin through a multicall symlink.
    // Execute its ordinary name, as process_capture does for cat, through owned sh.
    let arguments = vec!["-c".as_bytes().to_vec(), "exec sleep 30".as_bytes().to_vec()];
    let mut child = process.spawn_file(&shell, &arguments, &empty, &empty, &empty, &empty).unwrap();
    child.request_stop().unwrap();
    assert_eq!(child.wait().unwrap(), 143i64);
    errno(child.request_stop(), 10);
    drop(final_stop);
    world.stdout().write(b"lifecycle ok\n");
}
