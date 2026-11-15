pub fn main(mut entropy: Entropy, mut stop: Stop, mut clock: Clock, mut ready: Readiness, mut root: Directory) {
    let mut bytes = [0u8; 32];
    entropy.fill(&mut bytes).unwrap();
    let _seconds = clock.unix_seconds().unwrap();
    let mut key = root.create_private("key").unwrap();
    key.set_private().unwrap();
    key.write_all(&bytes).unwrap();
    ready.watch_stop(&mut stop, 1usize).unwrap();
    let _target = stop.target().unwrap();
    let _requested = stop.requested().unwrap();
    ready.unwatch_stop(&mut stop).unwrap();
}
