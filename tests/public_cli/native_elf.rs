//! Public AOT output is a detached executable with no interpreter dependency.
use super::*;

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
