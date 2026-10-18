//! Public AOT output is a detached executable with no interpreter dependency.
use super::*;
use std::io::Write;

#[test]
fn native_elf_public_build_runs_lkjscript_compiler_as_detached_binary_filter() {
    let _semantics = semantic_command::Scope::enter();
    let temporary = tempfile::tempdir().unwrap();
    let copied =
        semantic_cli::executable(temporary.path(), semantic_cli::Mode::selected(), &binary());
    let project = temporary.path().join("compiler-source");
    let run = |args: &[&str]| compact_success_at(&copied, temporary.path(), args);
    run(&[
        "new",
        path(&project),
        "--template",
        "minimal",
        "--name",
        "compiler",
    ]);
    let base = current_revision_at(&copied, temporary.path(), &project);
    let request = temporary.path().join("compiler.lkjc");
    std::fs::write(
        &request,
        format!(
            "request base={base}\n{}\n{}",
            include_str!("../../tools/selfhost/input.lkjc"),
            include_str!("../../tools/selfhost/elf.lkjc")
        ),
    )
    .unwrap();
    let review = temporary.path().join("compiler.lkjplan");
    let plan = run(&[
        "--project",
        path(&project),
        "change",
        "plan",
        "--input-file",
        path(&request),
        "--output",
        path(&review),
    ]);
    run(&[
        "--project",
        path(&project),
        "change",
        "apply",
        "--input-file",
        path(&request),
        "--plan",
        compact_field(compact_record(&plan, "plan"), "token").unwrap(),
    ]);
    let compiler = temporary.path().join("compiler");
    run(&[
        "--project",
        path(&project),
        "build",
        "--native-elf",
        path(&compiler),
        "--target",
        "compile-elf",
    ]);

    // Independently encode the documented profile-1 expression 1 + 41.
    let mut payload = vec![0u8; 168 + 4 * 16];
    payload[..8].copy_from_slice(b"LKJCI001");
    payload[8..12].copy_from_slice(&1u32.to_le_bytes());
    let length = payload.len() as u64;
    for (at, value) in [(16, length), (24, 1), (32, 7)] {
        payload[at..at + 8].copy_from_slice(&value.to_le_bytes());
    }
    for (i, value) in [
        104u32, 1, 120, 1, 136, 1, 168, 4, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0,
    ]
    .into_iter()
    .enumerate()
    {
        payload[72 + i * 4..76 + i * 4].copy_from_slice(&value.to_le_bytes());
    }
    for (i, (tag, value)) in [(1u32, 1i64), (1, 41), (2, 0), (6, 0)]
        .into_iter()
        .enumerate()
    {
        let at = 168 + i * 16;
        payload[at..at + 4].copy_from_slice(&tag.to_le_bytes());
        payload[at + 8..at + 16].copy_from_slice(&value.to_le_bytes());
    }
    let mut digest = blake3::Hasher::new();
    digest.update(&payload[..40]);
    digest.update(&payload[72..]);
    payload[40..72].copy_from_slice(digest.finalize().as_bytes());
    let input = temporary.path().join("input.lkjci");
    std::fs::write(&input, &payload).unwrap();
    let detached = temporary.path().join("detached");
    std::fs::create_dir(&detached).unwrap();
    let execute_compiler = || {
        support::spawn(
            Command::new(&compiler)
                .env_clear()
                .current_dir(&detached)
                .stdin(Stdio::from(File::open(&input).unwrap()))
                .stdout(Stdio::piped())
                .stderr(Stdio::piped()),
        )
        .unwrap()
        .wait_with_output()
        .unwrap()
    };
    let output = execute_compiler();
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty());
    let stage = detached.join("program.stage");
    let executable = detached.join("program");
    let mut file = File::create_new(&stage).unwrap();
    file.write_all(&output.stdout).unwrap();
    file.set_permissions(std::fs::Permissions::from_mode(0o755))
        .unwrap();
    file.sync_all().unwrap();
    drop(file);
    std::fs::rename(stage, &executable).unwrap();
    File::open(&detached).unwrap().sync_all().unwrap();
    let output =
        support::output(Command::new(&executable).env_clear().current_dir(&detached)).unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"42");
    assert!(output.stderr.is_empty());
    payload[0] = 0;
    std::fs::write(&input, payload).unwrap();
    let rejected = execute_compiler();
    assert_eq!(rejected.status.code(), Some(65));
    assert!(rejected.stdout.is_empty());
    assert_eq!(rejected.stderr, b"compiler_rejected:1\n");
}

#[test]
fn native_elf_build_executes_detached_and_preserves_create_new_publication() {
    let _semantics = semantic_command::Scope::enter();
    let temporary = tempfile::tempdir().unwrap();
    let copied =
        semantic_cli::executable(temporary.path(), semantic_cli::Mode::selected(), &binary());
    let project = temporary.path().join("native");
    let run = |args: &[&str]| compact_success_at(&copied, temporary.path(), args);
    run(&[
        "new",
        path(&project),
        "--template",
        "minimal",
        "--name",
        "native",
    ]);
    let base = current_revision_at(&copied, temporary.path(), &project);
    let request = temporary.path().join("native.lkjc");
    std::fs::write(
        &request,
        format!(
            "request base={base}\n{}",
            include_str!("../fixtures/native-elf.lkjc")
        ),
    )
    .unwrap();
    let review = temporary.path().join("native.lkjplan");
    let plan = run(&[
        "--project",
        path(&project),
        "change",
        "plan",
        "--input-file",
        path(&request),
        "--output",
        path(&review),
    ]);
    run(&[
        "--project",
        path(&project),
        "change",
        "apply",
        "--input-file",
        path(&request),
        "--plan",
        compact_field(compact_record(&plan, "plan"), "token").unwrap(),
    ]);
    let before = current_revision_at(&copied, temporary.path(), &project);
    let artifact = temporary.path().join("standalone");
    let built = run(&[
        "--project",
        path(&project),
        "build",
        "--native-elf",
        path(&artifact),
        "--target",
        "arithmetic",
    ]);
    assert_eq!(
        compact_field(compact_record(&built, "native"), "target"),
        Some("arithmetic")
    );
    assert_eq!(
        std::fs::metadata(&artifact).unwrap().permissions().mode() & 0o777,
        0o755
    );
    let result = run(&[
        "--project",
        path(&project),
        "run",
        "arithmetic",
        "--arguments",
        "[7,3]",
    ]);
    assert!(
        result
            .iter()
            .any(|record| compact_field(record, "value") == Some("14"))
    );
    let detached = temporary.path().join("detached");
    std::fs::create_dir(&detached).unwrap();
    let output = support::output(
        Command::new(&artifact)
            .args(["7", "3"])
            .current_dir(&detached)
            .env_clear(),
    )
    .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(output.stdout, b"14");
    assert!(output.stderr.is_empty());
    let original = std::fs::read(&artifact).unwrap();
    for args in [
        vec![
            "--project",
            path(&project),
            "build",
            "--native-elf",
            path(&artifact),
            "--target",
            "arithmetic",
        ],
        vec![
            "--project",
            path(&project),
            "build",
            "--native-elf",
            path(&artifact),
            "--output",
            "conflicting",
        ],
        vec![
            "--project",
            path(&project),
            "build",
            "--native-elf",
            path(&artifact),
            "--target",
            "absent",
        ],
    ] {
        let result = support::output(
            Command::new(&copied)
                .args(args)
                .current_dir(temporary.path()),
        )
        .unwrap();
        assert!(!result.status.success());
        assert_eq!(std::fs::read(&artifact).unwrap(), original);
    }
    assert_eq!(
        current_revision_at(&copied, temporary.path(), &project),
        before
    );
    let unsupported = temporary.path().join("unsupported");
    let rejected = support::output(
        Command::new(&copied)
            .args([
                "--project",
                path(&project),
                "build",
                "--native-elf",
                path(&unsupported),
                "--target",
                "unsupported",
            ])
            .current_dir(temporary.path()),
    )
    .unwrap();
    assert!(!rejected.status.success());
    assert!(!unsupported.exists());
    assert!(
        String::from_utf8_lossy(&rejected.stdout).contains("native_elf_unsupported")
            || String::from_utf8_lossy(&rejected.stderr).contains("native_elf_unsupported")
    );
    let bad = support::output(
        Command::new(&artifact)
            .args(["9223372036854775808", "1"])
            .env_clear(),
    )
    .unwrap();
    assert_eq!(bad.status.code(), Some(64));
    assert!(bad.stdout.is_empty());
    assert_eq!(bad.stderr, b"native_argument\n");
}
