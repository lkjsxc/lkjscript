use super::super::model::{Gate, InputSnapshot};
use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

fn now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("fixture clock")
        .as_nanos()
}

struct Fixture {
    _temporary: tempfile::TempDir,
    repository: std::path::PathBuf,
    root: std::path::PathBuf,
    registry: registry::GateRegistry,
    snapshot: InputSnapshot,
    receipt: CheckReceipt,
}

impl Fixture {
    fn new() -> Self {
        let temporary = tempfile::tempdir().expect("source superset fixture");
        let repository = temporary.path().canonicalize().expect("canonical fixture");
        fs::write(
            repository.join("Cargo.lock"),
            b"independent source fixture\n",
        )
        .expect("fixture lock");
        fs::write(
            repository.join("Cargo.toml"),
            b"[workspace]\nmembers = []\nresolver = \"3\"\n",
        )
        .expect("fixture manifest");
        fs::write(repository.join(".gitignore"), b".artifacts/\n").expect("ignore originals");
        fs::create_dir_all(repository.join("docs/guides/examples")).expect("audited directory");
        for arguments in [
            vec!["init", "--quiet"],
            vec!["add", "--", "Cargo.lock", "Cargo.toml", ".gitignore"],
            vec![
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "fixture",
            ],
        ] {
            assert!(
                std::process::Command::new("git")
                    .args(arguments)
                    .current_dir(&repository)
                    .status()
                    .expect("fixture git")
                    .success()
            );
        }
        let root = repository.join(".artifacts/lkjscript-dev/check/full-original");
        fs::create_dir_all(&root).expect("original directory");
        fs::copy(
            std::env::current_exe().expect("current verifier"),
            root.join("lkjscript-dev"),
        )
        .expect("retained verifier");
        let producer = Gate::new(
            "producer",
            vec!["/bin/true".to_owned(), "producer".to_owned()],
        );
        let mut required = Gate::new(
            "required",
            vec!["/bin/true".to_owned(), "required".to_owned()],
        );
        required.dependencies.push("producer".to_owned());
        // A distinct real executable makes the complete producer's runtime larger
        // than its required source subset, as well as exercising a third process.
        let extra = Gate::new(
            "extra_owner",
            vec!["/usr/bin/printf".to_owned(), "full-only\n".to_owned()],
        );
        let source_only = Gate::new(
            "source_only",
            vec!["/bin/true".to_owned(), "source-only".to_owned()],
        );
        let registry = registry::GateRegistry::new(vec![producer, required, extra, source_only])
            .expect("independent source and producer registry");
        let requested = vec!["required".to_owned(), "extra_owner".to_owned()];
        let selected = registry.closure(&requested).expect("full fixture closure");
        let snapshot = snapshot::capture(&repository).expect("source snapshot");
        let runtime = snapshot::runtime_identity(
            &repository,
            registry
                .input_commands(&selected)
                .expect("complete producer tools"),
        )
        .expect("producer runtime");
        evidence::publish_json(
            &root.join("inputs.json"),
            &InputManifest {
                contract_version: CHECK_CONTRACT_VERSION,
                snapshot: snapshot.clone(),
            },
        )
        .expect("original inputs");
        evidence::publish_json(
            &root.join("dag.json"),
            &registry
                .manifest(&requested, &selected, 1)
                .expect("full DAG"),
        )
        .expect("original DAG");
        let started = now();
        let cache = cache::VerificationCache::new(&repository, &root.join("cache"));
        let gates = executor::execute_dag(
            &registry,
            &selected,
            &executor::ExecutionOptions {
                repository: &repository,
                run_directory: &root,
                snapshot: &snapshot,
                runtime: &runtime,
                maximum_workers: 1,
                allow_reuse: false,
                fresh_reason: "full_profile_requires_fresh",
                cache: &cache,
                origin: None,
                live: None,
            },
        )
        .expect("actual fresh bounded processes");
        let receipt = CheckReceipt {
            contract_version: CHECK_CONTRACT_VERSION,
            status: AggregateStatus::Passed,
            profile: "full".to_owned(),
            profile_definition_digest: registry
                .profile_digest("full", &requested)
                .expect("full profile identity"),
            started_unix_nanoseconds: started,
            completed_unix_nanoseconds: now(),
            elapsed_nanoseconds: 1,
            git_head: Some(snapshot.git_head.clone()),
            worktree_input_digest: Some(snapshot.digest.clone()),
            final_worktree_input_digest: Some(snapshot.digest.clone()),
            input_stable: true,
            input_manifest: Some(evidence::relative(&repository, &root.join("inputs.json"))),
            dag_manifest: Some(evidence::relative(&repository, &root.join("dag.json"))),
            runtime: Some(runtime),
            requested_gates: requested,
            selected_gates: selected,
            passed_gates: 3,
            fresh_passed_gates: 3,
            reused_passed_gates: 0,
            unrun_gates: Vec::new(),
            maximum_workers: 1,
            fresh_required: true,
            gates,
            failure: None,
        };
        Self {
            _temporary: temporary,
            repository,
            root,
            registry,
            snapshot,
            receipt,
        }
    }

    fn read(&self, receipt: &CheckReceipt) -> Result<usize, DevError> {
        let requested = if receipt.profile == "release-source" {
            vec!["required".to_owned()]
        } else {
            vec!["required".to_owned(), "extra_owner".to_owned()]
        };
        admit_required_source(
            &self.root.join("receipt.json"),
            &self.repository,
            &self.snapshot.git_head,
            receipt,
            &self.registry,
            &requested,
            &["required".to_owned()],
        )
    }
}

#[test]
fn complete_full_original_admits_only_the_required_source_scope_and_refuses_faults() {
    let fixture = Fixture::new();
    assert_eq!(
        fixture
            .read(&fixture.receipt)
            .expect("genuine full original"),
        2
    );
    assert_eq!(fixture.receipt.profile, "full");
    assert_eq!(fixture.receipt.gates.len(), 3);
    for fault in 0..10 {
        let mut receipt = fixture.receipt.clone();
        match fault {
            0 => {
                receipt.gates.pop();
                receipt.selected_gates.pop();
                receipt.passed_gates = 2;
                receipt.fresh_passed_gates = 2;
            }
            1 => receipt.gates[2].status = GateStatus::Failed,
            2 => receipt.gates[2].execution = ExecutionKind::Reused,
            3 => {
                receipt.profile = "product".to_owned();
                receipt.profile_definition_digest = fixture
                    .registry
                    .profile_digest("product", &receipt.requested_gates)
                    .expect("coherently renamed custom profile");
            }
            4 => {
                receipt.profile = "release-source".to_owned();
                receipt.profile_definition_digest = fixture
                    .registry
                    .profile_digest("release-source", &receipt.requested_gates)
                    .expect("renamed full producer");
            }
            5 => {
                receipt.profile_definition_digest = evidence::VerificationDigest::of(b"substitute")
            }
            6 => {
                receipt.gates.remove(1);
                receipt.selected_gates.remove(1);
                receipt.passed_gates = 2;
                receipt.fresh_passed_gates = 2;
            }
            7 => {
                let runtime = receipt.runtime.as_mut().expect("original runtime");
                assert!(
                    runtime
                        .command_executables
                        .remove("/usr/bin/printf")
                        .is_some()
                );
                runtime.digest =
                    snapshot::runtime_digest(runtime).expect("rehashed subset runtime");
            }
            8 => receipt.git_head = Some("f".repeat(40)),
            9 => receipt.gates[2].status = GateStatus::Skipped,
            _ => unreachable!(),
        }
        assert!(
            fixture.read(&receipt).is_err(),
            "full source fault {fault} passed"
        );
    }
    assert!(
        admit_required_source(
            &fixture.root.join("receipt.json"),
            &fixture.repository,
            &fixture.snapshot.git_head,
            &fixture.receipt,
            &fixture.registry,
            &fixture.receipt.requested_gates,
            &["source_only".to_owned()],
        )
        .is_err(),
        "a complete producer cannot supply a required gate outside its closure"
    );
    let dag_path = fixture.root.join("dag.json");
    let dag = fs::read(&dag_path).expect("original DAG bytes");
    let mut substituted: super::super::model::DagManifest =
        serde_json::from_slice(&dag).expect("original DAG");
    substituted.nodes.pop();
    substituted.selected_closure.pop();
    evidence::publish_json(&dag_path, &substituted).expect("substituted source-only DAG");
    assert!(fixture.read(&fixture.receipt).is_err());
    fs::write(&dag_path, dag).expect("restore full DAG");
    let lock = fixture.repository.join("Cargo.lock");
    let original = fs::read(&lock).expect("original source context");
    fs::write(&lock, b"changed source context\n").expect("change source context");
    assert!(fixture.read(&fixture.receipt).is_err());
    fs::write(&lock, original).expect("restore source context");
    let log = fixture.root.join("extra_owner.stdout.log");
    let original = fs::read(&log).expect("full-only owner original");
    fs::write(&log, b"forged extra owner\n").expect("change extra original");
    assert!(fixture.read(&fixture.receipt).is_err());
    fs::write(&log, original).expect("restore extra original");
    assert_eq!(
        fixture.read(&fixture.receipt).expect("restored originals"),
        2
    );
    evidence::publish_json(&fixture.root.join("receipt.json"), &fixture.receipt)
        .expect("original fixture receipt");
    assert!(
        read_release_source_receipt(
            &fixture.root.join("receipt.json"),
            &fixture.repository,
            &fixture.snapshot.git_head,
        )
        .is_err(),
        "a real but incomplete fixture is not the maintained full profile"
    );
}

#[test]
fn maintained_full_original_covers_all_twenty_five_source_gates() {
    let temporary = tempfile::tempdir().expect("registry fixture");
    let registry =
        registry::base_registry(temporary.path(), temporary.path(), Path::new("/bin/true"))
            .expect("maintained registry");
    let source = registry
        .closure(&registry::profile("release-source").expect("source profile"))
        .expect("source closure");
    let full = registry
        .closure(&registry::profile("full").expect("full profile"))
        .expect("full closure");
    assert_eq!(source.len(), super::super::RELEASE_SOURCE_GATE_COUNT);
    assert_eq!(full.len(), 29);
    assert!(source.iter().all(|name| full.contains(name)));
}
