use super::*;

fn handle_main(body: &str) -> String {
    format!(
        concat!(
            "main/\nsig/\nCapability/\nFileSystem\n/Capability\n->\nUnit\n/sig\n",
            "params/\nfile-system\nCapability/\nFileSystem\n/Capability\n/params\n",
            "let/\nbind/\nhandle\nunwrap-ok/\nsys-open-read/\nfile-system\n",
            "unwrap-ok/\npath-from-str/\nstr/\n/tmp/lkjscript-affine-test\n/str\n",
            "/path-from-str\n/unwrap-ok\n/sys-open-read\n/unwrap-ok\n",
            "/bind\n{body}\n/let\n/main\n"
        ),
        body = body
    )
}

fn cleanup() -> &'static str {
    "do/\nunwrap-ok/\ndrop/\nhandle\n/drop\n/unwrap-ok\nunit\n/do"
}

#[test]
fn affine_handle_requires_explicit_cleanup() {
    let source = handle_main("unit");
    let error = compile_source(&source, "handle-leak.lkjscript", &Limits::default())
        .expect_err("unconsumed handle")
        .to_string();
    assert!(error.contains("must be returned, moved, or dropped"));
}

#[test]
fn affine_handle_rejects_double_drop_and_use_after_drop() {
    let double = handle_main(
        "do/\nunwrap-ok/\ndrop/\nhandle\n/drop\n/unwrap-ok\nunwrap-ok/\ndrop/\nhandle\n/drop\n/unwrap-ok\nunit\n/do",
    );
    let error = compile_source(&double, "handle-double-drop.lkjscript", &Limits::default())
        .expect_err("double drop")
        .to_string();
    assert!(error.contains("already moved or dropped"));

    let reused = handle_main(
        "do/\nunwrap-ok/\ndrop/\nhandle\n/drop\n/unwrap-ok\nsys-read-byte/\nhandle\n/sys-read-byte\nunit\n/do",
    );
    let error = compile_source(
        &reused,
        "handle-use-after-drop.lkjscript",
        &Limits::default(),
    )
    .expect_err("use after drop")
    .to_string();
    assert!(error.contains("already moved or dropped"));
}

#[test]
fn affine_handle_cleanup_reaches_verified_ssa() {
    let source = handle_main(cleanup());
    let program = compile_source(&source, "handle-drop.lkjscript", &Limits::default())
        .expect("explicit drop");
    assert!(!program.ssa().program().functions.is_empty());
}

#[test]
fn borrowed_stdin_handle_cannot_be_dropped_as_an_owned_local() {
    let source = stdio_unit_main(
        "unwrap-ok/\ndrop/\nstdin-handle/\nstdio\n/stdin-handle\n/drop\n/unwrap-ok",
    );
    let error = compile_source(&source, "borrowed-drop.lkjscript", &Limits::default())
        .expect_err("borrowed handle drop")
        .to_string();
    assert!(error.contains("direct affine Handle local"));
}
