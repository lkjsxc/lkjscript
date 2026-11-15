// expect: output 736f636b65742070726f63657373207769746e657373206f6b0a
// witness: {}
fn read_exact(file: &mut File, bytes: &mut [u8]) {
    let mut at = 0usize;
    while at < bytes.len() {
        let count = file.read(&mut bytes[at..]).unwrap();
        assert!(count > 0usize);
        at += count;
    }
}
pub fn main(mut world: World) {
    let mut listener = world.listen("127.0.0.1", 0u16).unwrap();
    let mut target = world.connector("127.0.0.1", listener.port().unwrap()).unwrap();
    let mut peer = target.connect().unwrap();
    let accepted = listener.accept().unwrap();
    let mut tools = world.directory("/bin").unwrap();
    let executable = tools.open("sh").unwrap();
    let errors = world.temporary_file().unwrap();
    let result = world.temporary_file().unwrap();
    // Shell builtins avoid a self-executed witness harness's private case selector.
    let arguments = vec![
        "-c".as_bytes().to_vec(),
        "IFS= read -r line && test \"$line\" = ping && printf pong".as_bytes().to_vec(),
    ];
    peer.write_all(b"ping\n").unwrap();
    let mut process = world.process();
    let mut child = process.spawn_file(&executable, &arguments, &accepted, &accepted, &errors, &result).unwrap();
    drop(accepted);
    let mut bytes = [0u8; 4];
    read_exact(&mut peer, &mut bytes);
    assert_eq!(&bytes[..], b"pong");
    assert_eq!(child.wait().unwrap(), 0i64);
    assert_eq!(peer.read(&mut bytes).unwrap(), 0usize);
    world.stdout().write(b"socket process witness ok\n");
}
