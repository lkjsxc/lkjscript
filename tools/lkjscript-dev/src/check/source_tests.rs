use super::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;

#[test]
fn declared_semantic_receipt_paths_cannot_be_selected_by_recorded_output_claims() {
    let temporary = tempfile::tempdir().unwrap();
    let repository = temporary.path().canonicalize().unwrap();
    let path = repository.join("declared-receipt.json");
    let other = repository.join("independent-valid-diagnostic.json");
    fs::write(&path, b"same immutable receipt bytes").unwrap();
    fs::write(&other, b"same immutable receipt bytes").unwrap();
    let mut gate = super::super::model::Gate::new("offline_packages", vec!["source-owner".into()]);
    gate.required_outputs = vec![path.clone()];
    let proof = evidence::proof(&path, evidence::relative(&repository, &path)).unwrap();
    assert_eq!(
        declared_output(&repository, &gate, std::slice::from_ref(&proof))
            .unwrap()
            .0,
        path
    );
    let substitute = evidence::proof(&other, evidence::relative(&repository, &other)).unwrap();
    assert!(declared_output(&repository, &gate, &[substitute]).is_err());
    assert!(declared_output(&repository, &gate, &[]).is_err());
    assert!(declared_output(&repository, &gate, &[proof.clone(), proof.clone()]).is_err());
    gate.required_outputs.push(other);
    assert!(declared_output(&repository, &gate, &[proof]).is_err());
}

#[test]
fn originals_reject_changed_bytes_modes_links_and_aliased_paths() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let path = root.join("receipt.json");
    fs::write(&path, b"original receipt").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let proof = evidence::proof(&path, "receipt.json".into()).unwrap();
    verify(&path, &proof).unwrap();
    fs::write(&path, b"different source").unwrap();
    assert!(verify(&path, &proof).is_err());
    fs::write(&path, b"original receipt").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(verify(&path, &proof).is_err());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let alias = root.join("alias");
    std::os::unix::fs::symlink(&root, &alias).unwrap();
    assert!(verify(&alias.join("receipt.json"), &proof).is_err());
    let linked = root.join("linked.json");
    std::os::unix::fs::symlink(&path, &linked).unwrap();
    assert!(verify(&linked, &proof).is_err());
    verify(&path, &proof).unwrap();
}

#[test]
fn serialized_bindings_resolve_only_canonical_repository_relative_originals() {
    let root = Path::new("/owned/repository");
    let temporary = tempfile::NamedTempFile::new().unwrap();
    let mut proof = evidence::proof(
        temporary.path(),
        ".artifacts/check/source/receipt.json".into(),
    )
    .unwrap();
    assert_eq!(original_path(root, &proof).unwrap(), root.join(&proof.path));
    for path in [
        "",
        "/foreign/receipt.json",
        "../foreign.json",
        ".artifacts/../foreign.json",
        "./receipt.json",
    ] {
        proof.path = path.into();
        assert!(original_path(root, &proof).is_err(), "accepted {path}");
    }
}

#[test]
fn admitted_binding_keeps_original_receipt_logs_outputs_and_exact_verifier_immutable() {
    // This fixture exercises immutable binding checks only. It does not construct
    // a passed producing run or bypass the production strict admission entrypoint.
    let temporary = tempfile::tempdir().unwrap();
    let repository = temporary.path().canonicalize().unwrap();
    let path = repository.join("receipt.json");
    let retain = |name: &str, bytes: &[u8]| {
        let path = repository.join(name);
        fs::write(&path, bytes).unwrap();
        evidence::proof(&path, name.into()).unwrap()
    };
    let receipt = retain("receipt.json", b"immutable original receipt");
    let stdout = retain("workspace_tests.stdout.log", b"original public outcomes");
    let stderr = retain("workspace_tests.stderr.log", b"original Cargo harness");
    let offline = retain("offline.json", b"original offline source owner");
    let tail = retain("tail.json", b"original tail source owner");
    let verifier = std::env::current_exe().unwrap().canonicalize().unwrap();
    let verifier = evidence::proof(&verifier, verifier.display().to_string()).unwrap();
    let source = AdmittedSource {
        proof: SourceProof {
            source_commit: "1".repeat(40),
            required_gates: RELEASE_SOURCE_GATE_COUNT,
            receipt,
            verifier,
            workspace_stdout: stdout,
            workspace_stderr: stderr,
            owner_receipts: vec![
                OwnerProof {
                    owner: SourceOwner::OfflinePackages,
                    receipt: offline,
                },
                OwnerProof {
                    owner: SourceOwner::PureTail,
                    receipt: tail,
                },
            ],
        },
        path,
        owner_paths: BTreeMap::from([
            (
                SourceOwner::OfflinePackages,
                repository.join("offline.json"),
            ),
            (SourceOwner::PureTail, repository.join("tail.json")),
        ]),
        repository: repository.clone(),
    };
    source.unchanged().unwrap();
    assert_eq!(source.gates(), 25);
    assert_eq!(
        source.workspace_logs().unwrap(),
        (
            "original public outcomes".into(),
            "original Cargo harness".into()
        )
    );
    for name in [
        "receipt.json",
        "workspace_tests.stdout.log",
        "workspace_tests.stderr.log",
        "offline.json",
        "tail.json",
    ] {
        let path = repository.join(name);
        let original = fs::read(&path).unwrap();
        fs::write(&path, b"changed original").unwrap();
        assert!(source.unchanged().is_err(), "accepted changed {name}");
        fs::write(&path, original).unwrap();
        source.unchanged().unwrap();
    }
    let mut other = source.clone();
    other.proof.verifier.bytes = Some(other.proof.verifier.bytes.unwrap() + 1);
    assert!(other.unchanged().is_err());
    let mut other = source.clone();
    other.proof.required_gates = RELEASE_SOURCE_GATE_COUNT - 1;
    assert!(
        other.unchanged().is_err(),
        "an incomplete source ownership count is not current evidence"
    );
    let owner = source.owner_proof(SourceOwner::PureTail).unwrap();
    assert_eq!(
        owner.path,
        repository.join("tail.json").display().to_string()
    );
    assert_eq!(owner.digest, source.proof.owner_receipts[1].receipt.digest);
}
