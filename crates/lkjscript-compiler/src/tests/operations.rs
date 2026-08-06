use super::*;

#[test]
fn bounded_terminal_operations_replace_arbitrary_ioctl() {
    let canonical = stdio_unit_main(
        "let/\nbind/\nstate\nnew-byte-vector/\n60\n/new-byte-vector\n/bind\nget-terminal-state/\nstandard-input/\nstdio\n/standard-input\nborrow-mut/\nstate\n/borrow-mut\n/get-terminal-state\n/let",
    );
    let arbitrary = stdio_unit_main(
        "sys-ioctl/\nstandard-input/\nstdio\n/standard-input\n21505\nnew-byte-vector/\n1\n/new-byte-vector\n/sys-ioctl",
    );
    compile_source(&canonical, "terminal.lkjscript").expect("bounded terminal operations compile");
    assert!(compile_source(&arbitrary, "terminal.lkjscript").is_err());
}

#[test]
fn descriptor_drop_requires_an_owned_handle() {
    let canonical =
        stdio_unit_main("is-ok/\ndrop/\nstandard-input/\nstdio\n/standard-input\n/drop\n/is-ok");
    let obsolete = stdio_unit_main("close/\nstandard-input/\nstdio\n/standard-input\n/close");
    let borrowed = compile_source(&canonical, "handles.lkjscript")
        .expect_err("borrowed stdin is not owned")
        .to_string();
    assert!(borrowed.contains("drop does not accept resource kind input-stream"));
    assert!(compile_source(&obsolete, "handles.lkjscript").is_err());
}
