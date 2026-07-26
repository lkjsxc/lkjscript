use std::os::unix::ffi::OsStrExt;

use super::*;

#[test]
fn buf_slice_copies_exact_bounded_ranges() {
    let mut arena = Arena::default();
    let source = arena
        .alloc(lkjscript_core::HeapObj::Buf(vec![0, 1, 2, 3]))
        .expect("test source buffer allocation");
    let slice = buf_slice(&mut arena, source, 1, 2).expect("slice range");
    assert_eq!(as_buf(&arena, slice).expect("slice bytes"), &[1, 2]);
    assert!(buf_slice(&mut arena, source, -1, 1).is_err());
    assert!(buf_slice(&mut arena, source, 3, 2).is_err());
}
#[test]
fn bulk_file_io_is_bounded_exact_and_reports_progress() -> std::io::Result<()> {
    let input = TempFile::new(&[0, 0xc3, 0xa9, 0xff, b'x'])?;
    let output = TempFile::new(&[])?;
    let mut arena = Arena::default();
    let mut handles = ResourceTable::default();
    let buffer = buf_new(&mut arena, 8).expect("bulk buffer");
    let input_handle = handles
        .sys_open_read(input.0.as_os_str().as_bytes())
        .expect("open input");
    assert_eq!(
        sys_read_into(&mut arena, &handles, input_handle, buffer, 1, 7).ok(),
        Some(5)
    );
    assert_eq!(
        &as_buf(&arena, buffer).expect("buffer")[1..6],
        &[0, 0xc3, 0xa9, 0xff, b'x']
    );
    assert_eq!(
        sys_read_into(&mut arena, &handles, input_handle, buffer, 0, 1).ok(),
        Some(0)
    );
    assert!(sys_read_into(&mut arena, &handles, input_handle, buffer, -1, 1).is_err());
    assert!(sys_read_into(&mut arena, &handles, input_handle, buffer, 7, 2).is_err());
    assert!(sys_read_into(
        &mut arena,
        &handles,
        input_handle,
        buffer,
        0,
        MAX_BULK_IO_BYTES as i64 + 1,
    )
    .is_err());
    assert!(sys_read_into(
        &mut arena,
        &handles,
        Value::from_small_i64(1).expect("integer"),
        buffer,
        0,
        0,
    )
    .is_err());
    handles.close(input_handle).expect("close input");
    assert!(sys_read_into(&mut arena, &handles, input_handle, buffer, 0, 0).is_err());

    let output_handle = handles
        .sys_open_write(output.0.as_os_str().as_bytes())
        .expect("open output");
    assert_eq!(
        sys_write_from(&arena, &handles, output_handle, buffer, 1, 5).ok(),
        Some(5)
    );
    assert_eq!(
        sys_write_from(&arena, &handles, output_handle, buffer, 0, 0).ok(),
        Some(0)
    );
    handles.close(output_handle).expect("close output");
    assert_eq!(fs::read(&output.0)?, vec![0, 0xc3, 0xa9, 0xff, b'x']);
    Ok(())
}
#[test]
fn random_fill_obeys_exact_bounded_ranges() {
    let mut arena = Arena::default();
    let buffer = buf_new(&mut arena, 8).expect("buffer");
    for index in 0..8 {
        buf_set(&mut arena, buffer, index, 0xaa).expect("initialize buffer");
    }
    assert_eq!(
        sys_random_fill(&mut arena, buffer, 2, 4).ok(),
        Some(Value::UNIT)
    );
    let bytes = as_buf(&arena, buffer).expect("filled buffer");
    assert_eq!(&bytes[..2], &[0xaa, 0xaa]);
    assert_eq!(&bytes[6..], &[0xaa, 0xaa]);
    assert_ne!(&bytes[2..6], &[0, 0, 0, 0]);
    assert!(sys_random_fill(&mut arena, buffer, -1, 1).is_err());
    assert!(sys_random_fill(&mut arena, buffer, 7, 2).is_err());
    assert!(sys_random_fill(&mut arena, buffer, 0, MAX_BULK_IO_BYTES as i64 + 1,).is_err());
}
