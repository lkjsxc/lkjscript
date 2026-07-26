use super::*;

#[test]
fn bounded_terminal_operations_replace_arbitrary_ioctl() {
    let canonical = stdio_unit_main(
        "sys-tty-get/\nstdin-handle/\nstdio\n/stdin-handle\nbuf-new/\n60\n/buf-new\n/sys-tty-get",
    );
    let arbitrary = stdio_unit_main(
        "sys-ioctl/\nstdin-handle/\nstdio\n/stdin-handle\n21505\nbuf-new/\n1\n/buf-new\n/sys-ioctl",
    );
    assert!(compile_source(&canonical, "terminal.lkjscript", &Limits::default()).is_ok());
    assert!(compile_source(&arbitrary, "terminal.lkjscript", &Limits::default()).is_err());
}

#[test]
fn descriptor_drop_requires_an_owned_handle() {
    let canonical =
        stdio_unit_main("is-ok/\ndrop/\nstdin-handle/\nstdio\n/stdin-handle\n/drop\n/is-ok");
    let obsolete = stdio_unit_main("close/\nstdin-handle/\nstdio\n/stdin-handle\n/close");
    let borrowed = compile_source(&canonical, "handles.lkjscript", &Limits::default())
        .expect_err("borrowed stdin is not owned")
        .to_string();
    assert!(borrowed.contains("direct affine Handle local"));
    assert!(compile_source(&obsolete, "handles.lkjscript", &Limits::default()).is_err());
}
