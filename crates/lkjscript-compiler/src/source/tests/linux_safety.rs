use super::*;

#[cfg(unix)]
#[test]
fn loader_rejects_symlink_import_escape() -> std::io::Result<()> {
    use std::os::unix::fs::symlink;

    let package = TempDir::new("package")?;
    let outside = TempDir::new("outside")?;
    fs::create_dir_all(package.0.join("src/std"))?;
    fs::write(outside.0.join("escaped.lkjscript"), named_def("escaped"))?;
    symlink(
        outside.0.join("escaped.lkjscript"),
        package.0.join("escaped.lkjscript"),
    )?;
    let entry = package.0.join("main.lkjscript");
    fs::write(
        &entry,
        format!(
            "imports/\nimport/\nescaped.lkjscript#escaped\n/import\n/imports\n{}",
            unit_main("unit")
        ),
    )?;
    let error = load(&entry, &Limits::default())
        .expect_err("symlink escape")
        .to_string();
    assert!(error.contains("escapes package roots"));
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn loader_rejects_fifo_as_non_regular_without_blocking() -> std::io::Result<()> {
    use std::process::Command;

    let directory = TempDir::new("fifo")?;
    let fifo = directory.0.join("main.lkjscript");
    let status = Command::new("mkfifo").arg(&fifo).status()?;
    if !status.success() {
        return Err(std::io::Error::other("mkfifo failed"));
    }
    let error = load(&fifo, &Limits::default()).expect_err("FIFO source must fail");
    assert!(error.message().contains("not a regular file"));
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn opened_outside_descriptor_is_rejected_for_inside_looking_request() -> std::io::Result<()> {
    let package = TempDir::new("descriptor-package")?;
    let outside = TempDir::new("descriptor-outside")?;
    let requested = package.0.join("inside.lkjscript");
    let actual = outside.0.join("actual.lkjscript");
    fs::write(&requested, unit_main("unit"))?;
    fs::write(&actual, unit_main("unit"))?;
    let package_root = package.0.canonicalize()?;
    let file = super::load::open_source_file(&actual)?;
    let error = super::load::opened_source_path(
        &file,
        &requested,
        &package_root,
        None,
        &SourceOrigin::in_memory("inside.lkjscript"),
    )
    .expect_err("opened outside descriptor must fail containment");
    assert!(error
        .message()
        .contains("opened source escapes package roots"));
    assert!(error.message().contains("inside.lkjscript"));
    assert!(error.message().contains("actual.lkjscript"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn non_utf8_host_logical_paths_are_rejected_without_collapse() -> std::io::Result<()> {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let package = TempDir::new("non-utf8")?;
    let package_root = package.0.canonicalize()?;
    let first = package
        .0
        .join(OsString::from_vec(b"source-\x80.lkjscript".to_vec()));
    let second = package
        .0
        .join(OsString::from_vec(b"source-\x81.lkjscript".to_vec()));
    fs::write(&first, unit_main("unit"))?;
    fs::write(&second, unit_main("unit"))?;
    let first = first.canonicalize()?;
    let second = second.canonicalize()?;
    let first_error = super::load::source_origin(&first, &package_root, None)
        .expect_err("first non-UTF-8 path must fail");
    let second_error = super::load::source_origin(&second, &package_root, None)
        .expect_err("second non-UTF-8 path must fail");
    assert!(first_error.message().contains("not valid UTF-8"));
    assert!(second_error.message().contains("not valid UTF-8"));
    assert_ne!(first_error.message(), second_error.message());
    Ok(())
}
