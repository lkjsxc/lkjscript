//! Public AOT output is a detached executable with no interpreter dependency.
use super::*;
use std::io::Write;
#[path = "../support/selfhost_proposals.rs"]
mod compiler_proposals;

#[test]
fn native_elf_public_derived_hash_matches_published_vectors() {
    let _semantics = semantic_command::Scope::enter();
    let temporary = tempfile::tempdir().unwrap();
    let copied =
        semantic_cli::executable(temporary.path(), semantic_cli::Mode::selected(), &binary());
    let project = temporary.path().join("hash-source");
    let run = |args: &[&str]| compact_success_at(&copied, temporary.path(), args);
    run(&[
        "new",
        path(&project),
        "--template",
        "minimal",
        "--name",
        "hash",
    ]);
    let base = current_revision_at(&copied, temporary.path(), &project);
    let request = temporary.path().join("hash.lkjc");
    std::fs::write(
        &request,
        format!(
            "request base={base}\n{}",
            include_str!("../fixtures/native-derived-hash.lkjc")
        ),
    )
    .unwrap();
    let review = temporary.path().join("hash.lkjplan");
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
    let executable = temporary.path().join("hash");
    run(&[
        "--project",
        path(&project),
        "build",
        "--native-elf",
        path(&executable),
        "--target",
        "hash",
    ]);
    let detached = temporary.path().join("detached");
    std::fs::create_dir(&detached).unwrap();
    let input = detached.join("material");
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/blake3-derive-vectors.json")).unwrap();
    for vector in vectors["cases"].as_array().unwrap() {
        let length = vector["input_len"].as_u64().unwrap() as usize;
        let material: Vec<_> = (0..length).map(|i| (i % 251) as u8).collect();
        std::fs::write(&input, material).unwrap();
        let output = support::spawn(
            Command::new(&executable)
                .env_clear()
                .current_dir(&detached)
                .stdin(Stdio::from(File::open(&input).unwrap()))
                .stdout(Stdio::piped())
                .stderr(Stdio::piped()),
        )
        .unwrap()
        .wait_with_output()
        .unwrap();
        assert!(output.status.success(), "length {length}: {output:?}");
        assert!(output.stderr.is_empty());
        let expected = vector["hash"].as_str().unwrap();
        assert_eq!(output.stdout.len() * 2, expected.len());
        for (byte, hex) in output
            .stdout
            .iter()
            .zip(expected.as_bytes().as_chunks::<2>().0)
        {
            assert_eq!(format!("{byte:02x}").as_bytes(), hex);
        }
    }
}

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
        format!("request base={base}\n{}", compiler_proposals::SOURCE),
    )
    .unwrap();
    let initial_head = std::fs::read(project.join("HEAD")).unwrap();
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
    assert_eq!(initial_head, std::fs::read(project.join("HEAD")).unwrap());
    assert!(std::fs::metadata(&review).unwrap().len() > 0);
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
    let accepted_head = std::fs::read(project.join("HEAD")).unwrap();
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
    assert_eq!(accepted_head, std::fs::read(project.join("HEAD")).unwrap());

    // Independently encode profile 6: a main function calls checked add(1, 41).
    // No exporter/code generator is used to construct this input.
    // Header: 160 bytes; descriptors: 7 * 32; functions: 2 * 80; instructions: 4 * 64.
    // The 96-byte blob contains four stack-state cells and two external-parameter cells.
    let mut payload = vec![0u8; 896];
    payload[..8].copy_from_slice(b"LKJCI001");
    fn put(bytes: &mut [u8], at: usize, value: u64) {
        bytes[at..at + 8].copy_from_slice(&value.to_le_bytes());
    }
    payload[8..12].copy_from_slice(&6u32.to_le_bytes());
    // Policy tag @12 and allowance @32 remain zero: explicitly unmetered.
    for (at, value) in [
        (16, 896),
        (24, 2),
        (72, 160),
        (80, 7),
        (88, 384),
        (96, 2),
        (104, 544),
        (120, 4),
        (128, 800),
        (136, 96),
        (144, 0),
    ] {
        put(&mut payload, at, value);
    }
    for ty in 1..=7 {
        put(&mut payload, 160 + (ty - 1) * 32, ty as u64);
    }
    put(&mut payload, 360, 1); // List<I64>'s element type.
    for (i, value) in [
        0, 1, 0, 0, 0, 4, 0, 800, 0, 0, 0, 1, 2, 2, 4, 0, 1, 864, 0, 0,
    ]
    .into_iter()
    .enumerate()
    {
        put(&mut payload, 384 + i * 8, value);
    }
    for (i, (tag, a, b, immediate, state, depth)) in [
        (1, 0, 0, 1i64, 800, 0),
        (1, 0, 0, 41, 800, 1),
        (9, 1, 2, 0, 816, 2),
        (11, 0, 0, 0, 848, 1),
    ]
    .into_iter()
    .enumerate()
    {
        let at = 544 + i * 64;
        put(&mut payload, at, tag);
        put(&mut payload, at + 8, a);
        put(&mut payload, at + 16, b);
        payload[at + 32..at + 40].copy_from_slice(&immediate.to_le_bytes());
        put(&mut payload, at + 48, state);
        put(&mut payload, at + 56, depth);
    }
    for at in (800..896).step_by(16) {
        put(&mut payload, at, 1); // I64 type identity.
        put(&mut payload, at + 8, 0); // Owned qualifier, separate from identity.
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
