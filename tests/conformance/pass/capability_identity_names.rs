struct World {
    value: i64,
}
struct Stdin {
    value: i64,
}
struct Stderr {
    value: i64,
}
struct Directory {
    value: i64,
}
struct File {
    value: i64,
}
impl World {
    fn stdin(&mut self) -> Stdin {
        Stdin { value: self.value }
    }
    fn stderr(&mut self) -> Stderr {
        Stderr { value: self.value }
    }
    fn directory(&mut self, _path: &str) -> Directory {
        Directory { value: self.value }
    }
}
impl Stdin {
    fn __lkjscript_read(&mut self, bytes: &mut [u8]) -> i64 {
        bytes[0] = self.value as u8;
        1
    }
}
impl Stderr {
    fn write(&mut self, _bytes: &[u8]) -> i64 {
        self.value
    }
}
impl Directory {
    fn __lkjscript_open(&mut self, _path: &[u8], _create: bool) -> File {
        File { value: self.value }
    }
}
impl File {
    fn __lkjscript_write(&mut self, bytes: &[u8]) -> i64 {
        self.value + bytes.len() as i64
    }
}
pub fn probe() -> i64 {
    let mut world = World { value: 40 };
    let mut input = world.stdin();
    let mut bytes = [0u8; 2];
    assert_eq!(input.__lkjscript_read(&mut bytes), 1);
    assert_eq!(bytes[0], 40u8);
    let mut error = world.stderr();
    assert_eq!(error.write(&bytes), 40);
    let mut directory = world.directory("ordinary data");
    let mut file = directory.__lkjscript_open(&bytes, false);
    file.__lkjscript_write(&bytes)
}
