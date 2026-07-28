use std::os::unix::ffi::OsStrExt;

use super::*;

#[test]
fn wrong_kind_is_rejected_before_socket_effects() -> std::io::Result<()> {
    let file = TempFile::new()?;
    let path = file.0.as_os_str().as_bytes();
    let mut table = ResourceTable::default();
    let handle = table
        .sys_open_read(path)
        .expect("open temporary file as handle");
    assert!(table.sys_listen(handle, 1).is_err());
    assert_eq!(table.metrics().resources_opened(), 1);
    assert_eq!(table.metrics().resources_closed(), 0);
    table
        .close(handle)
        .expect("close file after kind rejection");
    Ok(())
}
#[test]
fn socket_ranges_are_checked_before_os_calls() {
    let mut table = ResourceTable::default();
    let socket = table.sys_socket().expect("create test socket");
    assert!(table.sys_bind(socket, -1).is_err());
    assert!(table.sys_bind(socket, 65_536).is_err());
    assert!(table.sys_listen(socket, -1).is_err());
    assert!(table.close(socket).is_ok());
}
