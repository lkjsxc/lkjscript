use super::*;

fn handle_main(body: &str) -> String {
    format!(
        concat!(
            "main/\nsig/\ninputs/\ncapability/\nfile-system\n/capability\n/inputs\noutput/\nunit\n/output\n/sig\n",
            "params/\nfile-system\ncapability/\nfile-system\n/capability\n/params\n",
            "let/\nbind/\nreader\nunwrap-ok/\nopen-file-reader/\nfile-system\n",
            "unwrap-ok/\nconvert-string-to-path/\nstring-literal/\n/tmp/lkjscript-affine-test\n/string-literal\n",
            "/convert-string-to-path\n/unwrap-ok\n/open-file-reader\n/unwrap-ok\n",
            "/bind\n{body}\n/let\n/main\n"
        ),
        body = body
    )
}

fn cleanup() -> &'static str {
    "do/\nunwrap-ok/\ndrop/\nreader\n/drop\n/unwrap-ok\nunit\n/do"
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
        "do/\nunwrap-ok/\ndrop/\nreader\n/drop\n/unwrap-ok\nunwrap-ok/\ndrop/\nreader\n/drop\n/unwrap-ok\nunit\n/do",
    );
    let error = compile_source(&double, "handle-double-drop.lkjscript", &Limits::default())
        .expect_err("double drop")
        .to_string();
    assert!(error.contains("already moved or dropped"));

    let reused = handle_main(
        "do/\nunwrap-ok/\ndrop/\nreader\n/drop\n/unwrap-ok\nread-resource-byte/\nreader\n/read-resource-byte\nunit\n/do",
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
        "unwrap-ok/\ndrop/\nstandard-input/\nstdio\n/standard-input\n/drop\n/unwrap-ok",
    );
    let error = compile_source(&source, "borrowed-drop.lkjscript", &Limits::default())
        .expect_err("borrowed handle drop")
        .to_string();
    assert!(error.contains("drop does not accept resource kind input-stream"));
}
