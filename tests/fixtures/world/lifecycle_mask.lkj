// A syscall-only child observes its own inherited mask, never another process.
fn put(bytes: &mut [u8], offset: usize, value: u64, width: usize) {
    for index in 0usize..width { bytes[offset + index] = (value >> (index * 8)) as u8; }
}
fn image() -> Vec<u8> {
    let code = b"\x48\x83\xec\x10\xb8\x0e\x00\x00\x00\x31\xff\x31\xf6\x48\x89\xe2\x41\xba\x08\x00\x00\x00\x0f\x05\xb8\x01\x00\x00\x00\xbf\x01\x00\x00\x00\x48\x89\xe6\xba\x08\x00\x00\x00\x0f\x05\xb8\x3c\x00\x00\x00\x31\xff\x0f\x05";
    let mut bytes = vec![0u8; 176];
    bytes[0] = 127; bytes[1] = b'E'; bytes[2] = b'L'; bytes[3] = b'F';
    bytes[4] = 2; bytes[5] = 1; bytes[6] = 1;
    put(&mut bytes, 16, 2, 2); put(&mut bytes, 18, 62, 2); put(&mut bytes, 20, 1, 4);
    put(&mut bytes, 24, 0x400000 + 176, 8); put(&mut bytes, 32, 64, 8);
    put(&mut bytes, 52, 64, 2); put(&mut bytes, 54, 56, 2); put(&mut bytes, 56, 2, 2);
    put(&mut bytes, 64, 1, 4); put(&mut bytes, 68, 5, 4);
    put(&mut bytes, 80, 0x400000, 8); put(&mut bytes, 88, 0x400000, 8);
    let size = (176 + code.len()) as u64;
    put(&mut bytes, 96, size, 8); put(&mut bytes, 104, size, 8); put(&mut bytes, 112, 4096, 8);
    put(&mut bytes, 120, 0x6474e551, 4); put(&mut bytes, 124, 6, 4); put(&mut bytes, 168, 16, 8);
    bytes.extend_from_slice(code);
    bytes
}
fn child_mask(process: &mut Process, executable: &File, empty: &File, output: &mut File) -> u64 {
    output.truncate(0usize).unwrap();
    output.rewind().unwrap();
    let arguments: Vec<Vec<u8>> = Vec::new();
    let mut child = process.spawn_file(executable, &arguments, empty, &*output, empty, empty).unwrap();
    assert_eq!(child.wait().unwrap(), 0i64);
    output.rewind().unwrap();
    let bytes = output.read_to_end().unwrap();
    assert_eq!(bytes.len(), 8usize);
    let mut mask = 0u64;
    for index in 0usize..8usize { mask |= (bytes[index] as u64) << (index * 8); }
    mask
}
pub fn main(mut world: World) {
    let mut executable = world.temporary_file().unwrap();
    executable.write_all(&image()).unwrap();
    executable.set_executable().unwrap();
    let empty = world.temporary_file().unwrap();
    let mut output = world.temporary_file().unwrap();
    let mut process = world.process();
    let before = child_mask(&mut process, &executable, &empty, &mut output);
    if !world.args().is_empty() {
        let mut dev = world.directory("/dev").unwrap();
        let mut held = Vec::new();
        loop {
            match dev.open("null") {
                Ok(file) => held.push(file),
                Err(24) => break,
                Err(_) => panic!("descriptor exhaustion failed"),
            }
        }
        assert!(!held.is_empty());
        match world.stop() { Err(24) => {}, _ => panic!("Stop must refuse without a descriptor") }
        drop(held);
        let failed = child_mask(&mut process, &executable, &empty, &mut output);
        assert_eq!(before, failed);
    }
    let stop = world.stop().unwrap();
    let during = child_mask(&mut process, &executable, &empty, &mut output);
    assert_eq!(before, during);
    drop(stop);
    let after = child_mask(&mut process, &executable, &empty, &mut output);
    assert_eq!(before, after);
    world.stdout().write(format!("mask {}\n", before & 0x4200u64).as_bytes());
}
