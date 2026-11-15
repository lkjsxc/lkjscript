// expect: output 736f636b65742070726f63657373206f6b0a
// The Rust witness harness selects a case through an environment variable;
// spawn_file clears environment, so self-execution cannot select its witness case.
// socket_process_witness.rs retains three-route socket-to-child coverage.
fn read_exact(file: &mut File, bytes: &mut [u8]) {
    let mut at = 0usize;
    while at < bytes.len() {
        let count = file.read(&mut bytes[at..]).unwrap();
        assert!(count > 0usize);
        at += count;
    }
}
pub fn main(mut world: World) {
    if !world.args().is_empty() {
        let mut input = world.stdin_file().unwrap();
        let mut output = world.stdout_file().unwrap();
        let mut bytes = [0u8; 4];
        read_exact(&mut input, &mut bytes);
        assert_eq!(&bytes[..], b"ping");
        output.write_all(b"pong").unwrap();
        return;
    }
    let mut listener = world.listen("127.0.0.1", 0u16).unwrap();
    let mut target = world.connector("127.0.0.1", listener.port().unwrap()).unwrap();
    let mut peer = target.connect().unwrap();
    let accepted = listener.accept().unwrap();
    let executable = world.executable().unwrap();
    let errors = world.temporary_file().unwrap();
    let result = world.temporary_file().unwrap();
    let arguments = vec!["child".as_bytes().to_vec()];
    peer.write_all(b"ping").unwrap();
    let mut process = world.process();
    let mut child = process.spawn_file(&executable, &arguments, &accepted, &accepted, &errors, &result).unwrap();
    drop(accepted);
    let mut bytes = [0u8; 4];
    read_exact(&mut peer, &mut bytes);
    assert_eq!(&bytes[..], b"pong");
    assert_eq!(child.wait().unwrap(), 0i64);
    assert_eq!(peer.read(&mut bytes).unwrap(), 0usize);
    world.stdout().write(b"socket process ok\n");
}
