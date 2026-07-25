use super::*;

#[test]
fn bounded_terminal_operations_replace_arbitrary_ioctl() {
    let canonical = unit_main(
        "sys-tty-get/\nstdin-handle/\n/stdin-handle\nbuf-new/\n60\n/buf-new\n/sys-tty-get",
    );
    let arbitrary = unit_main(
        "sys-ioctl/\nstdin-handle/\n/stdin-handle\n21505\nbuf-new/\n1\n/buf-new\n/sys-ioctl",
    );
    assert!(compile_source(&canonical, "terminal.lkjscript", &Limits::default()).is_ok());
    assert!(compile_source(&arbitrary, "terminal.lkjscript", &Limits::default()).is_err());
}

#[test]
fn descriptor_names_are_handle_and_result_explicit() {
    let canonical =
        unit_main("is-ok/\nsys-close/\nstdin-handle/\n/stdin-handle\n/sys-close\n/is-ok");
    let obsolete = unit_main("close/\nstdin-handle/\n/stdin-handle\n/close");
    assert!(compile_source(&canonical, "handles.lkjscript", &Limits::default()).is_ok());
    assert!(compile_source(&obsolete, "handles.lkjscript", &Limits::default()).is_err());
}
