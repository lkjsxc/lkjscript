pub fn main(mut world: World) {
    let args = world.args();
    let closed = args[0][0] - b'0';
    let mut listener = world.listen("127.0.0.1", 0u16).unwrap();
    assert!(listener.port().unwrap() != 0);
    let mut stream = world.__lkjscript_file(closed + 1).unwrap();
    if closed == 0 {
        let mut bytes = [0u8; 1];
        assert_eq!(stream.__lkjscript_read(&mut bytes), -9);
    } else {
        assert_eq!(stream.__lkjscript_write(b"x"), -9);
    }
    drop(listener);
    let mut listener = world.listen("127.0.0.1", 0u16).unwrap();
    assert!(listener.port().unwrap() != 0);
}
