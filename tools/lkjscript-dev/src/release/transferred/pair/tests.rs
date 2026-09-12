use super::*;
use std::cell::RefCell;

type PhaseHook = Box<dyn FnMut(&str, &process::ProcessControl)>;
type ProcessHook = Box<dyn FnMut(&str, &mut process::ProcessSpec, &process::ProcessControl)>;
thread_local! {
    static PHASE_HOOK: RefCell<Option<PhaseHook>> = RefCell::new(None);
    static PROCESS_HOOK: RefCell<Option<ProcessHook>> = RefCell::new(None);
}
pub(in crate::release::transferred) fn phase_hook(name: &str, control: &process::ProcessControl) {
    PHASE_HOOK.with(|hook| {
        if let Some(hook) = hook.borrow_mut().as_mut() {
            hook(name, control);
        }
    });
}
pub(in crate::release::transferred) fn process_hook(
    name: &str,
    mut spec: process::ProcessSpec,
    control: &process::ProcessControl,
) -> process::ProcessSpec {
    PROCESS_HOOK.with(|hook| {
        if let Some(hook) = hook.borrow_mut().as_mut() {
            hook(name, &mut spec, control);
        }
    });
    spec
}

fn fixture_members(root: &Path) -> Vec<archive::tests::TestMember> {
    let owner = crate::release::target::tests::elf_fixture(&[(0, 0)]);
    let candidate = root.join("elf");
    fs::write(&candidate, &owner).expect("test-only ELF");
    let mut members = archive::tests::test_members();
    members[1].bytes = owner;
    let hash = |bytes: &[u8]| archive::sha256_bytes(bytes).expect("digest");
    let manifest = ReleaseManifest {
        publication_mode: PublicationMode::DryRun,
        product: ProductIdentity {
            name: "lkjscript".to_owned(),
            version: lkjscript::PRODUCT_VERSION.to_owned(),
        },
        source: SourceIdentity {
            repository: "lkjsxc/lkjscript".to_owned(),
            expected_release_tag: format!("v{}", lkjscript::PRODUCT_VERSION),
            tagged_commit_sha: "0".repeat(40),
            commit_timestamp_unix_seconds: 1_700_000_000,
            annotated_tag_object_sha: None,
        },
        target_triple: target::TARGET_TRIPLE.to_owned(),
        toolchain: ToolchainIdentity {
            rustc: "test".to_owned(),
            cargo: "test".to_owned(),
            toolchain_channel: crate::release::TOOLCHAIN_CHANNEL.to_owned(),
        },
        cargo_lock_sha256: hash(b"test"),
        executable: ExecutableIdentity {
            archive_mode: 0o755,
            byte_length: members[1].bytes.len() as u64,
            sha256: hash(&members[1].bytes),
            elf: target::inspect_static_elf(&candidate).expect("static fixture"),
            capabilities_digest: "2".repeat(64),
        },
        root_license: PayloadIdentity {
            archive_mode: 0o644,
            byte_length: members[2].bytes.len() as u64,
            sha256: hash(&members[2].bytes),
        },
        third_party_notices: NoticeIdentity {
            generator: "cargo-about".to_owned(),
            generator_version: crate::release::CARGO_ABOUT_VERSION.to_owned(),
            downloaded_archive_sha256: Sha256Digest::new(
                crate::release::CARGO_ABOUT_ARCHIVE_SHA256.to_owned(),
            )
            .expect("notice digest"),
            executable_sha256: Sha256Digest::new(
                crate::release::CARGO_ABOUT_EXECUTABLE_SHA256.to_owned(),
            )
            .expect("tool digest"),
            invocation: crate::release::normalized_notice_invocation(),
            archive_mode: 0o644,
            byte_length: members[3].bytes.len() as u64,
            sha256: hash(&members[3].bytes),
        },
        packaging: PackagingIdentity {
            tar_format: "posix-ustar".to_owned(),
            tar_version: "test".to_owned(),
            tar_invocation: archive::normalized_tar_invocation(1_700_000_000),
            gzip_version: "test".to_owned(),
            gzip_level: 9,
            gzip_name_header: false,
            gzip_time_header: false,
            gzip_invocation: archive::normalized_gzip_invocation(),
            numeric_owner: 0,
            numeric_group: 0,
            source_timestamp_unix_seconds: 1_700_000_000,
            members: archive::manifest_members(),
        },
    };
    crate::release::validate_manifest(&manifest).expect("valid fixture manifest");
    members[4].bytes = archive::canonical_json(&manifest).expect("manifest");
    members
}
fn pack(root: &Path, members: &[archive::tests::TestMember]) -> PathBuf {
    let input = root.join("assets");
    fs::create_dir(&input).expect("assets");
    let tar = root.join("input.tar");
    fs::write(&tar, archive::tests::test_tar(members)).expect("test tar");
    let output = std::process::Command::new("gzip")
        .args(["-n", "-9", "-c"])
        .arg(&tar)
        .output()
        .expect("gzip fixture");
    assert!(output.status.success());
    fs::write(input.join(archive::ARCHIVE_NAME), output.stdout).expect("archive");
    let digest = archive::sha256_file(&input.join(archive::ARCHIVE_NAME))
        .expect("digest")
        .0;
    fs::write(
        input.join(archive::CHECKSUM_NAME),
        crate::release::checksum_bytes(&digest),
    )
    .expect("checksums");
    // Deliberately malformed archive fixtures have no admissible bootstrap; admission must fail first.
    let script = match archive::verify_archive(&input.join(archive::ARCHIVE_NAME), root, None) {
        Ok(verified) => crate::release::bootstrap::render(&verified).expect("fixture bootstrap"),
        Err(_) => b"invalid fixture bootstrap\n".to_vec(),
    };
    fs::write(input.join(crate::release::bootstrap::NAME), script).expect("installer");
    input
}
fn admit_fixture(root: &Path, input: &Path) -> Result<archive::VerifiedArchive, DevError> {
    archive::admit_archive(
        &input.join(archive::ARCHIVE_NAME),
        &input.join(archive::CHECKSUM_NAME),
        root,
        &root.join("extracted"),
        &process::ProcessControl::default(),
    )
}

#[test]
fn equality_inventory_and_same_tag_different_valid_content_are_independent() {
    let fixture = tempfile::tempdir().expect("fixture");
    let original = fixture_members(fixture.path());
    for different in [false, true] {
        let root = tempfile::tempdir().expect("pair");
        let left = tempfile::tempdir().expect("left");
        let right = tempfile::tempdir().expect("right");
        let exact_assets = pack(left.path(), &original);
        let mut latest = original.clone();
        if different {
            latest[3].bytes.push(b'!');
            let mut manifest: ReleaseManifest =
                serde_json::from_slice(&latest[4].bytes).expect("manifest");
            manifest.third_party_notices.byte_length = latest[3].bytes.len() as u64;
            manifest.third_party_notices.sha256 =
                archive::sha256_bytes(&latest[3].bytes).expect("digest");
            latest[4].bytes =
                archive::canonical_json(&manifest).expect("canonical changed manifest");
        }
        let latest_assets = pack(right.path(), &latest);
        let options = PairOptions {
            acquisition: Acquisition::Simulated,
            verify: false,
            exact_assets,
            latest_assets,
            tag: format!("v{}", lkjscript::PRODUCT_VERSION),
            commit: "0".repeat(40),
            publication: PublicationMode::DryRun,
            evidence_root: root.path().to_path_buf(),
            verifier_identity: root.path().join("unused"),
            expected_verifier_sha256: "0".repeat(64),
            expected_verifier_bytes: 1,
        };
        let routes = [Route::Exact, Route::Latest].map(|route| {
            fs::create_dir(route.root(&options)).expect("route");
            let admission = admit_fixture(&route.root(&options), route.assets(&options))
                .expect("each container is valid");
            validate_admission(&admission, &options).expect("same source/tag/publication");
            for foreign in ["tag", "source", "publication"] {
                let mut invalid = options.clone();
                match foreign {
                    "tag" => invalid.tag = "v0.0.0".to_owned(),
                    "source" => invalid.commit = "1".repeat(40),
                    _ => invalid.publication = PublicationMode::Release,
                }
                assert!(
                    validate_admission(&admission, &invalid).is_err(),
                    "{route:?}/{foreign}"
                );
            }
            RouteReceipt {
                route,
                inputs: inputs(route.assets(&options)).expect("inputs"),
                extraction: Some(
                    extraction(&route.extraction(&options), &admission).expect("extracted"),
                ),
                admission: Some(admission),
                admission_nanoseconds: 1,
                lifecycle: None,
            }
        });
        if different {
            assert!(
                compare(&options, &routes).is_err(),
                "equal versions cannot establish equality"
            );
        } else {
            let equality = compare(&options, &routes).expect("full equality");
            assert_eq!(
                equality
                    .comparisons
                    .iter()
                    .map(|c| c.name.as_str())
                    .collect::<Vec<_>>(),
                [
                    "archive",
                    "checksums",
                    "installer",
                    "manifest",
                    "executable"
                ]
            );
            assert!(equality.comparisons.iter().all(|c| c.compared_bytes > 0
                && c.compared_bytes == c.exact.file.byte_length
                && c.compared_bytes == c.latest.file.byte_length));
            let mut omitted = routes.clone();
            omitted[1].admission = None;
            assert!(
                compare(&options, &omitted).is_err(),
                "identical inputs do not excuse a missing admission"
            );
        }
        let mut swapped = options.clone();
        std::mem::swap(&mut swapped.exact_assets, &mut swapped.latest_assets);
        if different {
            assert!(compare(&swapped, &routes).is_err());
        }
    }
}

#[test]
fn pair_admission_rejects_identical_malformed_containers_in_either_route() {
    let fixture = tempfile::tempdir().expect("fixture");
    let original = fixture_members(fixture.path());
    let valid = tempfile::tempdir().expect("valid control");
    let assets = pack(valid.path(), &original);
    let observed = admit_fixture(valid.path(), &assets).expect("complete valid admission");
    extraction(&valid.path().join("extracted"), &observed).expect("own extracted facts");
    let mut cases = Vec::new();
    let mut duplicate = original.clone();
    duplicate.push(original[4].clone());
    cases.push(("duplicate", duplicate));
    let mut extra = original.clone();
    let mut member = original[2].clone();
    member.name = "lkjscript/extra".to_owned();
    extra.push(member);
    cases.push(("extra", extra));
    for name in ["../escape", "/absolute", "lkjscript/../escape"] {
        let mut fault = original.clone();
        fault[2].name = name.to_owned();
        cases.push((name, fault));
    }
    for kind in *b"12" {
        let mut fault = original.clone();
        fault[2].kind = kind;
        cases.push(("link", fault));
    }
    for mode in [0o644, 0o777, 0o4755] {
        let mut fault = original.clone();
        fault[1].mode = mode;
        cases.push(("mode", fault));
    }
    let mut fault = original.clone();
    fault[1].bytes[0] ^= 1;
    cases.push(("invalid executable", fault));
    let mut fault = original.clone();
    fault[4].bytes = b"{}\n".to_vec();
    cases.push(("canonical invalid manifest", fault));
    let mut fault = original.clone();
    fault[4].bytes.push(b' ');
    cases.push(("noncanonical manifest", fault));
    let mut fault = original.clone();
    fault[1].bytes.push(1);
    cases.push(("candidate mismatch", fault));
    for (name, members) in cases {
        for route in [Route::Exact, Route::Latest] {
            let root = tempfile::tempdir().expect("fault root");
            let assets = pack(root.path(), &members);
            assert!(
                admit_fixture(root.path(), &assets).is_err(),
                "{route:?}: {name}"
            );
            assert!(
                !root.path().join("extracted").exists(),
                "failed admission published extraction"
            );
        }
    }
    for content in [
        b"".as_slice(),
        b"bogus\n",
        b"0000000000000000000000000000000000000000000000000000000000000000  wrong.tar.gz\n",
    ] {
        let root = tempfile::tempdir().expect("checksum root");
        let assets = pack(root.path(), &original);
        fs::write(assets.join(archive::CHECKSUM_NAME), content).expect("fault");
        assert!(admit_fixture(root.path(), &assets).is_err());
    }
}

#[test]
fn equality_compares_every_byte_in_both_input_orders() {
    let root = tempfile::tempdir().expect("root");
    let left = root.path().join("left");
    let right = root.path().join("right");
    let bytes = vec![42_u8; 2 * 64 * 1024 + 3];
    fs::write(&left, &bytes).expect("left");
    fs::write(&right, &bytes).expect("right");
    assert_eq!(
        stream_equal(&left, &right).expect("equal"),
        bytes.len() as u64
    );
    for index in [0, 65536, bytes.len() - 1] {
        let mut changed = bytes.clone();
        changed[index] ^= 1;
        fs::write(&right, &changed).expect("fault");
        assert!(stream_equal(&left, &right).is_err());
        assert!(stream_equal(&right, &left).is_err());
    }
    fs::write(&right, &bytes[..bytes.len() - 1]).expect("truncate");
    assert!(stream_equal(&left, &right).is_err());
    assert!(stream_equal(&right, &left).is_err());
}

#[test]
fn independent_inputs_reject_aliases_and_sidecars() {
    let root = tempfile::tempdir().expect("root");
    for name in [
        archive::ARCHIVE_NAME,
        archive::CHECKSUM_NAME,
        crate::release::bootstrap::NAME,
    ] {
        fs::write(root.path().join(name), b"fixture").expect("input");
    }
    inputs(root.path()).expect("independent regular files");
    assert!(directory(&PathBuf::from(format!("{}/.", root.path().display()))).is_err());
    let alias = root.path().join("alias");
    std::os::unix::fs::symlink(root.path(), &alias).expect("alias");
    assert!(inputs(&alias).is_err());
    assert!(inputs(root.path()).is_err());
    fs::remove_file(&alias).expect("remove owned alias");
    let other = tempfile::tempdir().expect("other");
    fs::hard_link(
        root.path().join(archive::ARCHIVE_NAME),
        other.path().join("archive"),
    )
    .expect("hard link");
    assert!(inputs(root.path()).is_err());
    assert!(binding(&other.path().join("archive")).is_err());
}

fn options_from_fixture(root: &Path) -> (PairOptions, PathBuf) {
    let identity_path = root.join("verifier/verifier-identity.json");
    let identity: serde_json::Value =
        serde_json::from_slice(&fs::read(&identity_path).expect("handoff")).expect("handoff JSON");
    let prepared: ReleaseReceipt = serde_json::from_slice(
        &fs::read(root.join("release-receipt.json")).expect("genuine preparation receipt"),
    )
    .expect("preparation JSON");
    assert_eq!(
        Some(prepared.commit_sha.as_str()),
        identity["commit_sha"].as_str()
    );
    assert_eq!(
        Some(prepared.release_tag.as_str()),
        identity["tag"].as_str()
    );
    let options = PairOptions {
        acquisition: Acquisition::Simulated,
        verify: false,
        exact_assets: root.join("exact-assets"),
        latest_assets: root.join("latest-assets"),
        tag: identity["tag"].as_str().expect("tag").to_owned(),
        commit: identity["commit_sha"].as_str().expect("commit").to_owned(),
        publication: prepared.publication_mode,
        evidence_root: root.join("pair"),
        verifier_identity: identity_path,
        expected_verifier_sha256: identity["file"]["sha256"]
            .as_str()
            .expect("hash")
            .to_owned(),
        expected_verifier_bytes: identity["file"]["byte_length"].as_u64().expect("bytes"),
    };
    (options, root.join("verifier/lkjscript-dev"))
}

fn flip_first_byte(path: &Path) -> u8 {
    use std::io::{Seek, SeekFrom, Write};
    let mut file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .expect("owned mutable fault input");
    let mut byte = [0];
    file.read_exact(&mut byte).expect("read byte");
    file.seek(SeekFrom::Start(0)).expect("seek");
    file.write_all(&[byte[0] ^ 1]).expect("flip");
    file.sync_all().expect("sync");
    byte[0]
}
fn restore_first_byte(path: &Path, original: u8) {
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .open(path)
        .expect("restore owned input");
    file.write_all(&[original]).expect("restore byte");
    file.sync_all().expect("sync original");
}

#[test]
#[ignore = "requires LKJSCRIPT_PAIR_SMOKE_ASSETS; only route admission/lifecycle development feedback, never full pair acceptance"]
fn live_route_recipe_smoke() {
    let input = PathBuf::from(
        std::env::var_os("LKJSCRIPT_PAIR_SMOKE_ASSETS").expect("existing real package assets"),
    );
    let root = tempfile::Builder::new()
        .prefix("lkjscript-pair-route-smoke-")
        .tempdir()
        .expect("outside checkout");
    let evidence_root = root.path().join("evidence");
    fs::create_dir(&evidence_root).expect("evidence");
    let mut options = PairOptions {
        acquisition: Acquisition::Simulated,
        verify: false,
        exact_assets: root.path().join("exact-assets"),
        latest_assets: root.path().join("latest-assets"),
        tag: String::new(),
        commit: String::new(),
        publication: PublicationMode::Release,
        evidence_root,
        verifier_identity: root.path().join("unused"),
        expected_verifier_sha256: "0".repeat(64),
        expected_verifier_bytes: 1,
    };
    for route in [Route::Exact, Route::Latest] {
        fs::create_dir(route.assets(&options)).expect("assets");
        for name in [
            archive::ARCHIVE_NAME,
            archive::CHECKSUM_NAME,
            crate::release::bootstrap::NAME,
        ] {
            archive::copy_new(&input.join(name), &route.assets(&options).join(name), 0o644)
                .expect("independent local input");
        }
        fs::create_dir(route.root(&options)).expect("route root");
        let admitted =
            admit_fixture(&route.root(&options), route.assets(&options)).expect("strict admission");
        options.tag = admitted.manifest.source.expected_release_tag.clone();
        options.commit = admitted.manifest.source.tagged_commit_sha.clone();
        options.publication = admitted.manifest.publication_mode;
        let receipt = lifecycle::run(
            &options,
            route,
            &admitted,
            &process::ProcessControl::default(),
        )
        .expect("lifecycle result");
        if receipt.status != Status::FreshPassed {
            let retained = root.keep();
            panic!(
                "route smoke failed; retained {}: {receipt:?}",
                retained.display()
            );
        }
        lifecycle::validate(&options, route, &admitted, &receipt).expect("actual recipe receipt");
    }
    root.close().expect("owned smoke cleanup");
}

// These only inject failures into private owned attempts. No hook exists in a production build,
// and none generates an accepted application receipt. The subsequent real pair is the recovery.
#[test]
#[ignore = "requires LKJSCRIPT_PAIR_REHEARSAL_ROOT with genuinely prepared exact/latest assets and verifier, before the final pair run"]
fn live_pair_interruptions_preserve_failures_before_fresh_recovery() {
    let root = PathBuf::from(
        std::env::var_os("LKJSCRIPT_PAIR_REHEARSAL_ROOT").expect("prepared rehearsal root"),
    );
    let (baseline, verifier) = options_from_fixture(&root);
    validate_options(&baseline, &verifier).expect("real prepared inputs");
    let sentinel = root.join("unrelated-sentinel");
    fs::write(&sentinel, b"unrelated state").expect("sentinel");
    let mut reports = Vec::new();
    for fault in [
        "between-admission-and-sharing",
        "lifecycle-interrupt",
        "output-exhaustion",
        "timeout",
        "cleanup-failure",
        "failing-child",
        "child-interrupt",
        "candidate-drift",
        "verifier-drift",
        "input-drift",
        "handoff-drift",
        "two-lifecycles-before-suite",
    ] {
        let mut options = baseline.clone();
        options.evidence_root = root.join(format!("failed-{fault}"));
        let events = std::rc::Rc::new(RefCell::new(Vec::new()));
        let observed = events.clone();
        let threads = std::rc::Rc::new(RefCell::new(Vec::new()));
        let handles = threads.clone();
        PROCESS_HOOK.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(move |name, spec, control| {
                observed.borrow_mut().push(name.to_owned());
                match (fault, name) {
                    ("lifecycle-interrupt", "exact-check") => control.kill(),
                    ("output-exhaustion", "exact-capabilities") => spec.maximum_stdout_bytes = 1,
                    ("timeout", "exact-check") => spec.timeout = Duration::from_nanos(1),
                    ("cleanup-failure", "exact-status-final") => {
                        fs::set_permissions(&spec.cwd, fs::Permissions::from_mode(0o500))
                            .expect("owned cleanup fault")
                    }
                    ("failing-child", "distributed-http") => {
                        spec.command = vec!["/bin/false".to_owned()]
                    }
                    ("child-interrupt", "distributed-http") => {
                        let control = control.clone();
                        handles.borrow_mut().push(std::thread::spawn(move || {
                            std::thread::sleep(Duration::from_millis(250));
                            control.kill();
                        }));
                    }
                    _ => (),
                }
            }))
        });
        let drifted = std::rc::Rc::new(RefCell::new(Vec::new()));
        let changes = drifted.clone();
        let drift_path = match fault {
            "candidate-drift" => Route::Exact.extraction(&options).join("lkjscript"),
            "verifier-drift" => verifier.clone(),
            "input-drift" => options.latest_assets.join(archive::ARCHIVE_NAME),
            _ => options.verifier_identity.clone(),
        };
        PHASE_HOOK.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(move |phase, control| {
                if fault.ends_with("-drift") && phase == "exact-lifecycle" {
                    changes
                        .borrow_mut()
                        .push((drift_path.clone(), flip_first_byte(&drift_path)));
                }
                if (fault == "between-admission-and-sharing" && phase == "latest-admission")
                    || (fault == "two-lifecycles-before-suite" && phase == "before-suite")
                {
                    control.kill();
                }
            }))
        });
        let result = run(&options, &verifier, &process::ProcessControl::default());
        PROCESS_HOOK.with(|hook| *hook.borrow_mut() = None);
        PHASE_HOOK.with(|hook| *hook.borrow_mut() = None);
        for (path, original) in drifted.borrow_mut().drain(..) {
            restore_first_byte(&path, original);
        }
        for thread in threads.borrow_mut().drain(..) {
            thread.join().expect("joined fault control");
        }
        let receipt = result.expect("failure receipt persisted");
        assert_ne!(receipt.status, Status::FreshPassed, "{fault}");
        assert!(read(&options, &verifier).is_err(), "{fault}");
        if fault == "two-lifecycles-before-suite" {
            assert!(
                receipt.routes.iter().all(|r| r
                    .lifecycle
                    .as_ref()
                    .is_some_and(|l| l.status == Status::FreshPassed)),
                "both routes must really finish"
            );
            assert_eq!(
                *events.borrow(),
                [
                    "exact-capabilities",
                    "exact-new",
                    "exact-status",
                    "exact-check",
                    "exact-build",
                    "exact-run",
                    "exact-status-final",
                    "latest-capabilities",
                    "latest-new",
                    "latest-status",
                    "latest-check",
                    "latest-build",
                    "latest-run",
                    "latest-status-final"
                ]
            );
        }
        for route in [Route::Exact, Route::Latest] {
            let runtime = route.root(&options).join("runtime");
            if fault == "cleanup-failure" && runtime.exists() {
                fs::set_permissions(&runtime, fs::Permissions::from_mode(0o700))
                    .expect("restore owned mode");
                fs::remove_dir_all(&runtime).expect("cleanup retained failure state");
            }
            assert!(!runtime.exists(), "{fault}: route cleanup");
        }
        reports.push(serde_json::json!({"fault":fault,"receipt":receipt_identity(&options.evidence_root.join("receipt.json")).expect("failure identity"),"phase":receipt.phase,"failure":receipt.failure,"process_operations":*events.borrow()}));
        assert_eq!(
            fs::read(&sentinel).expect("sentinel preserved"),
            b"unrelated state"
        );
    }
    fs::remove_file(sentinel).expect("owned test sentinel");
    evidence::publish_json(&root.join("pair-interruption-results.json"), &reports)
        .expect("retained fault results");
}

#[test]
#[ignore = "requires LKJSCRIPT_PAIR_FIXTURE_ROOT from a genuine completed pair; no applications execute"]
fn live_pair_receipt_fault_matrix() {
    let pair_root = PathBuf::from(
        std::env::var_os("LKJSCRIPT_PAIR_FIXTURE_ROOT").expect("actual completed pair"),
    );
    let fixture_root = pair_root.parent().expect("fixture parent");
    let (mut options, verifier) = options_from_fixture(fixture_root);
    options.evidence_root = pair_root.clone();
    options.verify = true;
    let observed: PairReceipt =
        serde_json::from_slice(&fs::read(pair_root.join("receipt.json")).expect("pair source"))
            .expect("typed pair source");
    options.acquisition = observed.acquisition;
    let baseline = read(&options, &verifier).expect("unmodified real pair passes");
    let path = pair_root.join("receipt.json");
    let original = fs::read(&path).expect("pair bytes");
    let single = single_options(&options);
    let aggregate = read_receipt(
        &single,
        &verifier,
        &load_manifest(&single).expect("manifest"),
    )
    .expect("real complete suite");
    assert_eq!(
        aggregate
            .children
            .iter()
            .map(|c| c.role.name())
            .collect::<Vec<_>>(),
        [
            "distributed-http",
            "outbound-http",
            "offline-packages",
            "pure-tail",
            "stateful-http"
        ]
    );
    assert_eq!(
        baseline.suite.as_ref().expect("suite").expensive_executions,
        5
    );
    assert_eq!(
        baseline
            .routes
            .iter()
            .filter(|r| r
                .lifecycle
                .as_ref()
                .is_some_and(|l| l.status == Status::FreshPassed))
            .count(),
        2
    );
    let mut results = Vec::new();
    let mut reject = |label: &str, fault: PairReceipt| {
        fs::write(
            &path,
            evidence::encode_json(&fault).expect("canonical mutation"),
        )
        .expect("fault");
        let error = read(&options, &verifier);
        fs::write(&path, &original).expect("restore exact original");
        results.push(
            serde_json::json!({"fault":label,"rejection":error.expect_err(label).to_string()}),
        );
    };
    for index in [0, 1] {
        let mut fault = baseline.clone();
        fault.routes[index].admission = None;
        reject("omit-route-admission", fault);
        let mut fault = baseline.clone();
        fault.routes[index].lifecycle = None;
        reject("omit-route-lifecycle", fault);
        let mut fault = baseline.clone();
        fault.routes[index].inputs.archive.file.byte_length += 1;
        reject("route-identity", fault);
        let mut fault = baseline.clone();
        fault.routes[index].inputs.installer.file.byte_length += 1;
        reject("installer-identity", fault);
        let mut fault = baseline.clone();
        fault.routes[index]
            .lifecycle
            .as_mut()
            .expect("lifecycle")
            .installation = None;
        reject("omit-installed-bootstrap", fault);
        let mut fault = baseline.clone();
        fault.routes[index]
            .lifecycle
            .as_mut()
            .expect("lifecycle")
            .commands
            .remove(2);
        reject("skip-route-operation", fault);
        let mut fault = baseline.clone();
        fault.routes[index]
            .lifecycle
            .as_mut()
            .expect("lifecycle")
            .cleanup_complete = false;
        reject("route-cleanup", fault);
    }
    let mut fault = baseline.clone();
    fault.routes.swap(0, 1);
    reject("route-order", fault);
    let mut fault = baseline.clone();
    fault.routes[1] = fault.routes[0].clone();
    reject("copy-exact-route", fault);
    let mut fault = baseline.clone();
    fault.suite = None;
    reject("absent-source", fault);
    let mut fault = baseline.clone();
    fault.recovery = None;
    reject("omit-predecessor-recovery", fault);
    let mut fault = baseline.clone();
    fault.equality = None;
    reject("absent-equality", fault);
    let mut fault = baseline.clone();
    fault
        .equality
        .as_mut()
        .expect("equality")
        .comparisons
        .remove(0);
    fault.suite.as_mut().expect("suite").equality_digest = evidence::VerificationDigest::of(
        &evidence::encode_json(fault.equality.as_ref().expect("equality"))
            .expect("rehashed equality"),
    );
    reject("weaken-full-content-equality-rehashed", fault);
    let mut fault = baseline.clone();
    fault.suite.as_mut().expect("suite").latest = Disposition::FreshExecution;
    reject("invent-latest-fresh-full", fault);
    let mut fault = baseline.clone();
    fault.suite.as_mut().expect("suite").expensive_executions = 10;
    reject("invent-count", fault);
    let mut fault = baseline.clone();
    fault
        .suite
        .as_mut()
        .expect("suite")
        .source_aggregate
        .file
        .path = options
        .evidence_root
        .join("foreign/receipt.json")
        .display()
        .to_string();
    reject("foreign-source-reference", fault);
    let mut fault = baseline.clone();
    fault.suite.as_mut().expect("suite").scope = Scope::PublicFormatValidation;
    reject("stale-source-scope", fault);
    let mut fault = baseline.clone();
    fault.scope = Scope::PublicFormatValidation;
    reject("rehearsal-cannot-be-public", fault);
    let mut fault = baseline.clone();
    fault.hosted_context.run_id = Some("1".to_owned());
    reject("run-id", fault);
    let mut fault = baseline.clone();
    fault
        .suite
        .as_mut()
        .expect("suite")
        .hosted_context
        .run_attempt = Some("2".to_owned());
    reject("attempt-binding", fault);
    let mut fault = baseline.clone();
    fault.target = "foreign".to_owned();
    reject("target", fault);
    let mut fault = baseline.clone();
    fault.source_commit = "0".repeat(40);
    reject("source", fault);
    let mut fault = baseline.clone();
    fault.status = Status::NotRun;
    reject("partial-receipt", fault);
    let mut fault = baseline.clone();
    fault.cleanup_complete = false;
    reject("pair-cleanup", fault);
    // Corrupt actual source receipts and recompute the pair's outer reference. Existing full child
    // validation must still reject; the separate transferred matrix falsifies all semantic cases.
    let aggregate_path = single.evidence_root.join("receipt.json");
    let aggregate_original = fs::read(&aggregate_path).expect("aggregate original");
    for name in [
        "distributed-http",
        "outbound-http",
        "offline-packages",
        "pure-tail",
        "stateful-http",
        "duplicate",
        "missing-process",
        "foreign-verifier",
    ] {
        let mut altered = aggregate.clone();
        match name {
            "duplicate" => altered.children.push(altered.children[0].clone()),
            "missing-process" => altered.children[0].process = None,
            "foreign-verifier" => {
                altered.verifier.sha256 = Sha256Digest::new("0".repeat(64)).expect("hash")
            }
            name => altered.children.retain(|c| c.role.name() != name),
        }
        fs::write(
            &aggregate_path,
            evidence::encode_json(&altered).expect("alter aggregate"),
        )
        .expect("write aggregate fault");
        let mut pair_fault = baseline.clone();
        pair_fault.suite.as_mut().expect("suite").source_aggregate =
            receipt_identity(&aggregate_path).expect("recomputed aggregate reference");
        fs::write(
            &path,
            evidence::encode_json(&pair_fault).expect("pair fault"),
        )
        .expect("write pair fault");
        let rejected = read(&options, &verifier);
        fs::write(&aggregate_path, &aggregate_original).expect("restore aggregate");
        fs::write(&path, &original).expect("restore pair");
        results.push(serde_json::json!({"fault":format!("rehashed-source-{name}"),"rejection":rejected.expect_err(name).to_string()}));
    }
    let child_path = single.evidence_root.join("offline-packages/receipt.json");
    let child_original = fs::read(&child_path).expect("real offline receipt");
    let child_value: serde_json::Value =
        serde_json::from_slice(&child_original).expect("offline JSON");
    for pointer in [
        "/observations/generic_capture_factory",
        "/observations/imported_capture_constraint",
        "/recursive/scale_sum",
        "/recursive/results/retained",
        "/recursive/session/messages",
        "/recursive/transaction_cancellation",
        "/effects/foreground/cells",
        "/effects/foreground/failures",
    ] {
        let mut child_fault = child_value.clone();
        let value = child_fault
            .pointer_mut(pointer)
            .expect("independent required observation");
        *value = match value {
            serde_json::Value::Bool(v) => serde_json::json!(!*v),
            serde_json::Value::Number(_) => serde_json::json!(0),
            serde_json::Value::String(_) => serde_json::json!("foreign"),
            serde_json::Value::Array(_) => serde_json::json!([]),
            serde_json::Value::Object(_) => serde_json::json!({}),
            _ => panic!("unsupported mutation"),
        };
        fs::write(
            &child_path,
            offline_packages::encode_transferred_test_fixture(child_fault)
                .expect("typed canonical child fault"),
        )
        .expect("child fault");
        let mut changed_aggregate = aggregate.clone();
        changed_aggregate
            .children
            .iter_mut()
            .find(|c| c.role == Oracle::OfflinePackages)
            .expect("owner")
            .receipt = Some(receipt_identity(&child_path).expect("rehashed child"));
        fs::write(
            &aggregate_path,
            evidence::encode_json(&changed_aggregate).expect("aggregate encoding"),
        )
        .expect("rehashed aggregate");
        let mut pair_fault = baseline.clone();
        pair_fault.suite.as_mut().expect("suite").source_aggregate =
            receipt_identity(&aggregate_path).expect("rehashed source");
        fs::write(
            &path,
            evidence::encode_json(&pair_fault).expect("pair encoding"),
        )
        .expect("rehashed pair");
        let rejected = read(&options, &verifier);
        fs::write(&child_path, &child_original).expect("restore child");
        fs::write(&aggregate_path, &aggregate_original).expect("restore aggregate");
        fs::write(&path, &original).expect("restore pair");
        results.push(serde_json::json!({"fault":pointer,"rehashed_child_aggregate_and_pair":true,"rejection":rejected.expect_err(pointer).to_string()}));
    }
    // Rewrite both nested source receipt and enclosing pair to exercise their actual readers.
    for route_index in [0, 1] {
        for pointer in [
            "/installation/command",
            "/installation/environment/PATH",
            "/installation/cleanup_complete",
            "/installation/candidate/file/sha256",
        ] {
            let mut value = serde_json::to_value(&baseline.routes[route_index].lifecycle)
                .expect("lifecycle JSON");
            let field = value
                .pointer_mut(pointer)
                .expect("required installation field");
            *field = match field {
                serde_json::Value::Array(_) => serde_json::json!([]),
                serde_json::Value::Bool(_) => serde_json::json!(false),
                _ => serde_json::json!("foreign"),
            };
            let decoded = serde_json::from_value::<lifecycle::Lifecycle>(value);
            if let Ok(changed) = decoded {
                let child_path = baseline.routes[route_index]
                    .route
                    .root(&options)
                    .join("lifecycle.json");
                let child_original = fs::read(&child_path).expect("lifecycle bytes");
                fs::write(
                    &child_path,
                    evidence::encode_json(&changed).expect("canonical lifecycle"),
                )
                .expect("rewrite lifecycle");
                let mut fault = baseline.clone();
                fault.routes[route_index].lifecycle = Some(changed);
                fs::write(&path, evidence::encode_json(&fault).expect("pair bytes"))
                    .expect("rewrite pair");
                let rejected = read(&options, &verifier);
                fs::write(&child_path, child_original).expect("restore lifecycle");
                fs::write(&path, &original).expect("restore pair");
                results.push(serde_json::json!({"fault":pointer,"rehashed_nested_pair":true,"rejection":rejected.expect_err(pointer).to_string()}));
            } else {
                results.push(
                    serde_json::json!({"fault":pointer,"rejection":"typed identity admission"}),
                );
            }
        }
    }
    for pointer in ["/commands", "/residents", "/retained", "/cleanup_complete"] {
        let mut value = serde_json::to_value(&baseline.recovery).expect("recovery JSON");
        let field = value.pointer_mut(pointer).expect("required recovery field");
        *field = if field.is_boolean() {
            serde_json::json!(false)
        } else {
            serde_json::json!([])
        };
        let changed: recovery::Recovery =
            serde_json::from_value(value).expect("typed recovery mutation");
        let child_path = options.evidence_root.join("recovery/receipt.json");
        let child_original = fs::read(&child_path).expect("recovery bytes");
        fs::write(
            &child_path,
            evidence::encode_json(&changed).expect("canonical recovery"),
        )
        .expect("rewrite recovery");
        let mut fault = baseline.clone();
        fault.recovery = Some(changed);
        fs::write(&path, evidence::encode_json(&fault).expect("pair bytes")).expect("rewrite pair");
        let rejected = read(&options, &verifier);
        fs::write(&child_path, child_original).expect("restore recovery");
        fs::write(&path, &original).expect("restore pair");
        results.push(serde_json::json!({"fault":pointer,"rehashed_nested_pair":true,"rejection":rejected.expect_err(pointer).to_string()}));
    }
    for input in [
        verifier.clone(),
        options.verifier_identity.clone(),
        Route::Exact.extraction(&options).join("lkjscript"),
        Route::Latest.extraction(&options).join("lkjscript"),
        Route::Exact
            .extraction(&options)
            .join("RELEASE-MANIFEST.json"),
        Route::Latest
            .extraction(&options)
            .join("RELEASE-MANIFEST.json"),
    ] {
        let original = flip_first_byte(&input);
        let rejected = read(&options, &verifier);
        restore_first_byte(&input, original);
        results.push(serde_json::json!({"fault":"binding-drift","path":input,"rejection":rejected.expect_err("binding drift").to_string()}));
    }
    for route in [Route::Exact, Route::Latest] {
        for relative in [
            archive::ARCHIVE_NAME,
            archive::CHECKSUM_NAME,
            crate::release::bootstrap::NAME,
        ] {
            let input = route.assets(&options).join(relative);
            use std::io::{Seek, SeekFrom, Write};
            let mut file = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&input)
                .expect("owned input");
            let mut first = [0_u8];
            file.read_exact(&mut first).expect("first byte");
            file.seek(SeekFrom::Start(0)).expect("seek");
            file.write_all(&[first[0] ^ 1]).expect("drift");
            file.sync_all().expect("flush drift");
            let rejected = read(&options, &verifier);
            file.seek(SeekFrom::Start(0)).expect("seek");
            file.write_all(&first).expect("restore original byte");
            file.sync_all().expect("flush original");
            results.push(serde_json::json!({"fault":format!("{route:?}-{relative}-input-drift"),"rejection":rejected.expect_err("input drift").to_string()}));
        }
    }
    read(&options, &verifier).expect("restored pair control passes");
    assert_eq!(fs::read(&path).expect("final pair"), original);
    evidence::publish_json(&pair_root.join("live-pair-fault-results.json"), &results)
        .expect("fault evidence");
}

#[test]
fn bootstrap_acquisition_failures_never_create_a_prefix() {
    let root = tempfile::tempdir().expect("bootstrap fixture");
    let members = fixture_members(root.path());
    let assets = pack(root.path(), &members);
    let script = assets.join(crate::release::bootstrap::NAME);
    let prefix = root
        .path()
        .join("prefix with spaces and $(touch should-not-exist)");
    let fixture = root.path().join("tools");
    fs::create_dir(&fixture).expect("tools");
    let tmp = root.path().join("tmp");
    fs::create_dir(&tmp).expect("tmp");
    for mode in [
        "transfer-failure",
        "partial",
        "wrong-length",
        "wrong-digest",
        "unsupported-host",
        "missing-tools",
        "duplicate-prefix",
    ] {
        let curl = match mode {
            "transfer-failure" => "#!/bin/sh\nexit 22\n".to_owned(),
            "partial" => "#!/bin/sh\nwhile [ \"$1\" != --output ]; do shift; done\nprintf partial > \"$2\"\nexit 18\n".to_owned(),
            "wrong-length" => "#!/bin/sh\nwhile [ \"$1\" != --output ]; do shift; done\nprintf short > \"$2\"\n".to_owned(),
            "wrong-digest" => "#!/bin/sh\nwhile [ \"$1\" != --output ]; do shift; done\nhead -c \"$FIXTURE_LENGTH\" /dev/zero > \"$2\"\n".to_owned(),
            _ => "#!/bin/sh\nexit 99\n".to_owned(),
        };
        let curl_path = fixture.join("curl");
        fs::write(&curl_path, curl).expect("curl fixture");
        fs::set_permissions(&curl_path, fs::Permissions::from_mode(0o755)).expect("mode");
        let uname = fixture.join("uname");
        if mode == "unsupported-host" {
            fs::write(&uname, "#!/bin/sh\nprintf Darwin\\n\n").expect("uname");
            fs::set_permissions(&uname, fs::Permissions::from_mode(0o755)).expect("mode");
        } else if uname.exists() {
            fs::remove_file(&uname).expect("remove uname");
        }
        let mut args = vec![
            script.display().to_string(),
            "--prefix".to_owned(),
            prefix.display().to_string(),
        ];
        if mode == "duplicate-prefix" {
            args.extend(["--prefix".to_owned(), prefix.display().to_string()]);
        }
        let output = std::process::Command::new("/bin/sh")
            .args(args)
            .env_clear()
            .env(
                "PATH",
                if mode == "missing-tools" {
                    fixture.display().to_string()
                } else {
                    format!("{}:/usr/bin:/bin", fixture.display())
                },
            )
            .env("TMPDIR", &tmp)
            .env(
                "FIXTURE_LENGTH",
                fs::metadata(assets.join(archive::ARCHIVE_NAME))
                    .expect("archive")
                    .len()
                    .to_string(),
            )
            .current_dir(root.path())
            .output()
            .expect("unchanged generated bootstrap");
        assert!(!output.status.success(), "{mode}");
        assert!(!prefix.exists(), "{mode}");
        assert!(!root.path().join("should-not-exist").exists());
        assert_eq!(
            fs::read_dir(&tmp).expect("temporary leftovers").count(),
            0,
            "{mode}"
        );
    }
}

#[test]
fn historical_integrity_admission_does_not_relax_current_producer_policy() {
    let root = tempfile::tempdir().expect("producer policy fixture");
    let members = fixture_members(root.path());
    let original: ReleaseManifest =
        serde_json::from_slice(&members[4].bytes).expect("canonical manifest");
    for field in [
        "toolchain",
        "notice-version",
        "notice-digest",
        "packaging-invocation",
    ] {
        let mut historical = original.clone();
        match field {
            "toolchain" => historical.toolchain.toolchain_channel = "1.75.0".to_owned(),
            "notice-version" => {
                historical.third_party_notices.generator_version = "0.1.0".to_owned()
            }
            "notice-digest" => {
                historical.third_party_notices.downloaded_archive_sha256 =
                    Sha256Digest::new("9".repeat(64)).expect("digest")
            }
            _ => historical.packaging.tar_invocation = vec!["historical tar invocation".to_owned()],
        }
        lkjscript::release_container::validate_manifest(&historical)
            .expect("historical format is internally valid");
        assert!(
            crate::release::validate_manifest(&historical).is_err(),
            "{field}"
        );
    }
}
