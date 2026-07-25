use super::*;

#[test]
fn file_handles_cannot_be_used_as_sockets() -> std::io::Result<()> {
    let file = TempFile::new()?;
    let path = file.0.to_string_lossy();
    let mut table = ResourceTable::default();
    let handle = table
        .sys_open_read(&path)
        .expect("open temporary file as handle");
    assert!(table.sys_listen(handle, 1).is_err());
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
