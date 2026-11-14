// Expected output: readonly ok\n
pub fn main(mut world: World) {
    let mut original = world.temporary_file().unwrap();
    original.write_all(b"abc").unwrap();
    original.rewind().unwrap();
    let mut byte = [0u8; 1];
    assert_eq!(original.read(&mut byte).unwrap(), 1);
    assert_eq!(byte[0], b'a');
    let mut readonly = original.readonly().unwrap();
    assert_eq!(readonly.read(&mut byte).unwrap(), 1);
    assert_eq!(byte[0], b'a');
    assert_eq!(original.read(&mut byte).unwrap(), 1);
    assert_eq!(byte[0], b'b');
    assert_eq!(readonly.__lkjscript_write(b"changed"), -9);
    let mut second = readonly.__lkjscript_readonly().unwrap();
    assert_eq!(second.read(&mut byte).unwrap(), 1);
    assert_eq!(byte[0], b'a');
    drop(original);
    drop(readonly);
    assert_eq!(second.read(&mut byte).unwrap(), 1);
    assert_eq!(byte[0], b'b');
    world.stdout().write(b"readonly ok\n");
}
