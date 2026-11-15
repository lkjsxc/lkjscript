// expect: output 73746f72616765206f6b0a
// witness: {}
fn errno<T>(result: Result<T, i64>, expected: i64) {
    match result {
        Err(error) => assert_eq!(error, expected),
        Ok(_) => panic!("expected an I/O error"),
    }
}
// Decode one Linux directory record. The 32-byte read buffer fits exactly one
// entry for these short fixture names. Padding and opaque d_off cookies are not
// part of the entry identity and need not repeat after a rewind.
fn directory_entry(bytes: &[u8]) -> (u64, u8, Vec<u8>) {
    assert!(bytes.len() >= 20usize);
    let size = bytes[16usize] as usize + (bytes[17usize] as usize) * 256usize;
    assert_eq!(size, bytes.len());
    let mut inode = 0u64;
    for index in 0usize..8usize {
        inode |= (bytes[index] as u64) << ((index * 8usize) as u64);
    }
    let mut end = 19usize;
    while end < size && bytes[end] != 0u8 { end += 1usize; }
    assert!(end > 19usize && end < size);
    (inode, bytes[18usize], bytes[19usize..end].to_vec())
}
pub fn main(mut world: World) {
    let mut clock = world.clock();
    let name = format!("lkjscript-storage-{}", clock.nanoseconds().unwrap());
    let mut temporary = world.directory("/tmp").unwrap();
    temporary.create_directory(&name).unwrap();
    let original = format!("/tmp/{name}");
    let moved = format!("{name}-moved");
    let mut directory = world.directory(&original).unwrap();
    temporary.rename(&name, &moved).unwrap();
    temporary.create_directory(&name).unwrap();
    let mut replacement_root = temporary.open_directory(&name).unwrap();
    assert!(replacement_root.list().unwrap().is_empty());
    let mut file = directory.create_new("journal").unwrap();
    errno(directory.create_new("journal"), 17);
    file.write_all(b"abc").unwrap();
    file.sync().unwrap();
    directory.sync().unwrap();
    let mut alias = file.duplicate().unwrap();
    file.rewind().unwrap();
    let mut byte = [0u8; 1];
    assert_eq!(alias.read(&mut byte).unwrap(), 1usize);
    assert_eq!(byte[0], b'a');
    assert_eq!(file.read(&mut byte).unwrap(), 1usize);
    assert_eq!(byte[0], b'b');
    drop(alias);
    let mut append = directory.open_append("journal").unwrap();
    assert_eq!(append.read(&mut byte).unwrap(), 1usize);
    assert_eq!(byte[0], b'a');
    append.write_all(b"def").unwrap();
    let mut alias = append.duplicate().unwrap();
    append.rewind().unwrap();
    assert_eq!(alias.read(&mut byte).unwrap(), 1usize);
    assert_eq!(byte[0], b'a');
    alias.write_all(b"g").unwrap();
    assert_eq!(append.read(&mut byte).unwrap(), 0usize);
    append.truncate(3usize).unwrap();
    assert_eq!(alias.read(&mut byte).unwrap(), 0usize);
    append.write_all(b"z").unwrap();
    append.truncate(6usize).unwrap();
    append.sync().unwrap();
    errno(append.truncate(!0usize), 22);
    let mut reader = append.readonly().unwrap();
    reader.sync().unwrap();
    assert_eq!(&reader.read_to_end().unwrap()[..], b"abcz\0\0");
    errno(reader.truncate(0usize), 22);
    drop(reader);
    drop(alias);

    let mut first = directory.open_append(".lock").unwrap();
    first.lock_exclusive().unwrap();
    first.lock_exclusive().unwrap();
    let mut held = first.duplicate().unwrap();
    let mut contender = directory.open_append(".lock").unwrap();
    errno(contender.lock_exclusive(), 11);
    drop(first);
    errno(contender.lock_exclusive(), 11);
    held.lock_exclusive().unwrap();
    drop(held);
    contender.lock_exclusive().unwrap();
    drop(contender);

    append.sync().unwrap();
    let mut previous = append.readonly().unwrap();
    assert_eq!(&previous.read_to_end().unwrap()[..], b"abcz\0\0");
    let mut replacement = directory.create_new("replacement").unwrap();
    replacement.write_all(b"new record").unwrap();
    replacement.sync().unwrap();
    drop(replacement);
    directory.rename("replacement", "journal").unwrap();
    directory.sync().unwrap();
    let mut current = directory.open("journal").unwrap();
    assert_eq!(&current.read_to_end().unwrap()[..], b"new record");
    previous.rewind().unwrap();
    assert_eq!(&previous.read_to_end().unwrap()[..], b"abcz\0\0");
    drop(current);
    drop(previous);
    drop(append);
    drop(file);

    errno(directory.create_new("../escape"), 18);
    errno(directory.open_append("/outside"), 18);
    errno(directory.create_new("a\0b"), 22);
    errno(directory.open_append("a\0b"), 22);
    errno(directory.__lkjscript_create_new(b"missing-zero"), 22);
    errno(directory.__lkjscript_open_append(b"a\0b\0"), 22);
    // An opened read-directory retains its kernel cursor until an explicit rewind.
    let mut entries = directory.open_directory(".").unwrap();
    let mut buffer = [0u8; 32];
    let first_count = entries.__lkjscript_read(&mut buffer);
    assert!(first_count > 0i64);
    let first = directory_entry(&buffer[..first_count as usize]);
    let second_count = entries.__lkjscript_read(&mut buffer);
    assert!(second_count > 0i64);
    let second = directory_entry(&buffer[..second_count as usize]);
    assert!(first.2 != second.2);
    assert_eq!(entries.__lkjscript_rewind(), 0i64);
    let repeated_count = entries.__lkjscript_read(&mut buffer);
    assert!(repeated_count > 0i64);
    let repeated = directory_entry(&buffer[..repeated_count as usize]);
    assert_eq!(repeated.0, first.0);
    assert_eq!(repeated.1, first.1);
    assert_eq!(repeated.2, first.2);
    drop(entries);
    assert!(replacement_root.list().unwrap().is_empty());
    directory.remove("journal").unwrap();
    directory.remove(".lock").unwrap();
    directory.sync().unwrap();
    drop(directory);
    drop(replacement_root);
    temporary.remove_directory(&name).unwrap();
    temporary.remove_directory(&moved).unwrap();
    world.stdout().write(b"storage ok\n");
}
