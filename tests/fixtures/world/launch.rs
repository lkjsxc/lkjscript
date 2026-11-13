//! Test host: preserve arbitrary non-NUL argv bytes and optionally close stdio.
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::process::CommandExt;
use std::process::Command;
unsafe extern "C" {
    fn close(fd: i32) -> i32;
    fn pipe(descriptors: *mut i32) -> i32;
    fn dup2(old: i32, new: i32) -> i32;
}
fn main() {
    let mut arguments = std::env::args().skip(1);
    let mut executable = arguments.next().unwrap();
    if let Some(fd) = executable.strip_prefix("--close=") {
        let fd: i32 = fd.parse().unwrap();
        assert!((0..=2).contains(&fd));
        unsafe {
            close(fd);
        }
        executable = arguments.next().unwrap();
    } else if let Some(fd) = executable.strip_prefix("--broken-pipe=") {
        let fd: i32 = fd.parse().unwrap();
        assert!((1..=2).contains(&fd));
        let mut descriptors = [0; 2];
        unsafe {
            assert_eq!(pipe(descriptors.as_mut_ptr()), 0);
            assert_eq!(close(descriptors[0]), 0);
            assert_eq!(dup2(descriptors[1], fd), fd);
            assert_eq!(close(descriptors[1]), 0);
        }
        executable = arguments.next().unwrap();
    }
    let mut command = Command::new(executable);
    for argument in arguments {
        assert_eq!(argument.len() % 2, 0);
        let mut bytes = Vec::new();
        for offset in (0..argument.len()).step_by(2) {
            bytes.push(u8::from_str_radix(&argument[offset..offset + 2], 16).unwrap());
        }
        command.arg(OsString::from_vec(bytes));
    }
    panic!("exec failed: {}", command.exec());
}
