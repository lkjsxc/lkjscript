use super::*;
use std::fs;
use std::path::PathBuf;

pub(super) fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("owned Cargo fixture");
    fs::create_dir_all(root.path().join("src")).expect("fixture source");
    fs::create_dir_all(root.path().join("target/release")).expect("declared output parent");
    fs::write(
        root.path().join("Cargo.toml"),
        "[package]\nname = \"lkjscript\"\nversion = \"0.0.0\"\nedition = \"2024\"\n[workspace]\n",
    )
    .expect("fixture manifest");
    fs::write(
        root.path().join("Cargo.lock"),
        "version = 4\n[[package]]\nname = \"lkjscript\"\nversion = \"0.0.0\"\n",
    )
    .expect("fixture lockfile");
    fs::write(root.path().join("src/main.rs"), "fn main() {}\n").expect("fixture program");
    fs::write(
        root.path().join("target/release/lkjscript"),
        b"stale-owned-output\n",
    )
    .expect("pre-existing regular output");
    root
}

pub(super) fn build(repository: &Path, target: &Path, name: &str) -> GateReceipt {
    let run = repository.join(name);
    fs::create_dir(&run).expect("fixture run directory");
    let mut command: Vec<String> = [
        "cargo",
        "build",
        "--workspace",
        "--release",
        "--locked",
        "--message-format=json",
        "--target-dir",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    command.push(target.to_str().expect("fixture target UTF-8").to_owned());
    let mut gate = Gate::new(name, command);
    gate.required_outputs = vec![repository.join("target/release/lkjscript")];
    execute_fresh(
        repository,
        &gate,
        Some(Path::new(env!("CARGO"))),
        VerificationDigest::of(b"independent-real-cargo-fixture"),
        run.join("stdout.log"),
        run.join("stderr.log"),
        unix_nanoseconds().expect("fixture start time"),
        Instant::now(),
        CacheObservation {
            eligible: false,
            lookup: CacheLookupStatus::Bypassed,
            reason: None,
            record: None,
            write: None,
        },
    )
    .expect("joined producer observation")
}

#[test]
fn redirected_cargo_cannot_certify_a_preexisting_regular_output() {
    let fixture = fixture();
    let receipt = build(
        fixture.path(),
        &fixture.path().join("elsewhere"),
        "redirected",
    );
    assert_eq!(
        receipt.process.as_ref().expect("process").status,
        ProcessStatus::Passed
    );
    assert_eq!(
        fs::read(fixture.path().join("target/release/lkjscript")).expect("stale output"),
        b"stale-owned-output\n"
    );
    let actual = fixture.path().join("elsewhere/release/lkjscript");
    assert!(
        actual.is_file(),
        "Cargo did produce its separately selected executable"
    );
    if receipt.status != GateStatus::Failed {
        eprintln!("Preserved failing fixture: {}", fixture.keep().display());
    }
    assert_eq!(
        receipt.status,
        GateStatus::Failed,
        "stale output accepted: {receipt:#?}"
    );
    assert!(
        receipt
            .reason
            .as_deref()
            .is_some_and(|reason| reason.starts_with("cargo_output_binding:"))
    );
}

#[test]
fn exact_cargo_output_and_unchanged_cargo_cache_hit_both_pass() {
    let fixture = fixture();
    let target: PathBuf = fixture.path().join("target");
    for name in ["cold", "warm"] {
        let receipt = build(fixture.path(), &target, name);
        assert_eq!(receipt.status, GateStatus::Passed, "{name}: {receipt:#?}");
        assert_eq!(receipt.outputs.len(), 1);
        assert_eq!(receipt.retained_outputs.len(), 1);
        let log = fs::read_to_string(fixture.path().join(name).join("stdout.log"))
            .expect("original producer records");
        assert!(log.contains(if name == "cold" {
            "\"fresh\":false"
        } else {
            "\"fresh\":true"
        }));
    }
}

#[test]
fn a_working_predecessor_executable_cannot_stand_in_for_the_current_source() {
    let fixture = fixture();
    let root = fixture.path();
    let source = root.join("src/main.rs");
    fs::write(&source, "fn main() { println!(\"predecessor\"); }\n").expect("predecessor source");
    let before = build(root, &root.join("target"), "before");
    assert_eq!(before.status, GateStatus::Passed);
    let old_binary = root.join("target/release/lkjscript");
    let old_bytes = fs::read(&old_binary).expect("original executable bytes");
    fs::write(&source, "fn main() { println!(\"current-source\"); }\n").expect("changed source");
    let changed = build(root, &root.join("elsewhere"), "changed");
    assert_eq!(
        changed.process.as_ref().expect("Cargo process").status,
        ProcessStatus::Passed
    );
    assert_eq!(
        fs::read(&old_binary).expect("unchanged predecessor"),
        old_bytes
    );
    for (path, expected) in [
        (old_binary, b"predecessor\n".as_slice()),
        (
            root.join("elsewhere/release/lkjscript"),
            b"current-source\n".as_slice(),
        ),
    ] {
        let outcome = std::process::Command::new(path)
            .output()
            .expect("joined owned executable");
        assert!(outcome.status.success());
        assert_eq!(outcome.stdout, expected);
    }
    assert_eq!(
        changed.status,
        GateStatus::Failed,
        "a runnable old product is still the wrong producer output"
    );
    assert!(
        changed
            .reason
            .as_deref()
            .is_some_and(|reason| reason.starts_with("cargo_output_binding:"))
    );
}
