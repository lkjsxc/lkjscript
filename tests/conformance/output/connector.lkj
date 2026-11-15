// expect: output 636f6e6e6563746f72206f6b0a
// witness: {}
fn errno<T>(result: Result<T, i64>, expected: i64) {
    match result {
        Err(error) => assert_eq!(error, expected),
        Ok(_) => panic!("expected an I/O error"),
    }
}
fn read_exact(file: &mut File, bytes: &mut [u8]) {
    let mut at = 0usize;
    while at < bytes.len() {
        let count = file.read(&mut bytes[at..]).unwrap();
        assert!(count > 0);
        at += count;
    }
}
pub fn main(mut world: World) {
    let invalid: Vec<String> = vec![
        "".to_string(), "localhost".to_string(), "127.0.0".to_string(),
        "127.0.0.1.2".to_string(), "127..0.1".to_string(), "256.0.0.1".to_string(),
        "-1.0.0.1".to_string(), "127.0.0.1\0".to_string(),
        "127.0.0.1:80".to_string(), "127.0.0.1.".to_string(),
    ];
    for address in invalid { errno(world.connector(&address, 1u16), 22); }
    assert!(world.connector("255.255.255.255", 65535u16).is_ok());
    errno(world.__lkjscript_connector(&[2u8, 0u8]), 22);
    let mut address = [0u8; 16];
    errno(world.__lkjscript_connector(&address), 22);
    let mut listener = world.listen("127.0.0.1", 0u16).unwrap();
    let port = listener.port().unwrap();
    address[0] = 2;
    address[2] = (port >> 8u64) as u8;
    address[3] = port as u8;
    address[4] = 127;
    address[7] = 1;
    address[15] = 1;
    errno(world.__lkjscript_connector(&address), 22);
    address[15] = 0;
    let mut endpoint = world.__lkjscript_connector(&address).unwrap();
    // The authority snapshots the endpoint; later buffer mutation cannot retarget it.
    address[4] = 1;
    let mut stream = endpoint.connect().unwrap();
    let mut accepted = listener.accept_nodelay().unwrap();
    stream.write_all(b"request").unwrap();
    let mut request = [0u8; 7];
    read_exact(&mut accepted, &mut request);
    assert_eq!(&request[..], b"request");
    accepted.write_all(b"reply").unwrap();
    let mut reply = [0u8; 5];
    read_exact(&mut stream, &mut reply);
    assert_eq!(&reply[..], b"reply");
    drop(accepted);
    assert_eq!(stream.read(&mut reply).unwrap(), 0usize);
    drop(stream);
    let mut numeric = world.connector("127.000.0.1", port).unwrap();
    let stream = numeric.connect().unwrap();
    let accepted = listener.accept().unwrap();
    drop(stream);
    drop(accepted);
    drop(listener);
    errno(numeric.connect(), 111);
    world.stdout().write(b"connector ok\n");
}
