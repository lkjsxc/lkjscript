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

pub(super) fn fixture_members(root: &Path) -> Vec<archive::tests::TestMember> {
    let owner = crate::release::target::tests::elf_fixture(&[(0, 0)]);
    let candidate = root.join("elf");
    fs::write(&candidate, &owner).expect("test-only ELF");
    let mut members = archive::tests::test_members();
    members[1].bytes = owner;
    let hash = |bytes: &[u8]| archive::sha256_bytes(bytes).expect("digest");
    let manifest = ReleaseManifest {
        encoding: ManifestEncoding::PublicationNeutral {
            build: BuildIdentity {
                target_policy_sha256: Sha256Digest::new(target::policy_sha256().expect("policy"))
                    .expect("policy digest"),
                command: [
                    "cargo",
                    "build",
                    "--release",
                    "--locked",
                    "--bin",
                    "lkjscript",
                    "--target",
                    "x86_64-unknown-linux-musl",
                ]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            },
        },
        product: ProductIdentity {
            name: "lkjscript".to_owned(),
            version: lkjscript::PRODUCT_VERSION.to_owned(),
        },
        source: SourceIdentity {
            repository: "lkjsxc/lkjscript".to_owned(),
            expected_release_tag: format!("v{}", lkjscript::PRODUCT_VERSION),
            tagged_commit_sha: "0".repeat(40),
            commit_timestamp_unix_seconds: 1_700_000_000,
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
            tier: Tier::BoundarySmoke,
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
            for foreign in ["tag", "source"] {
                let mut invalid = options.clone();
                match foreign {
                    "tag" => invalid.tag = "v0.0.0".to_owned(),
                    "source" => invalid.commit = "1".repeat(40),
                    _ => unreachable!(),
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

#[test]
fn boundary_options_distinguish_exact_pair_and_once_candidate_installation() {
    let common = [
        "--exact-assets",
        "/exact",
        "--tag",
        "v0.1.38",
        "--commit",
        "0000000000000000000000000000000000000000",
        "--acquisition",
        "simulated",
        "--evidence-root",
        "/evidence",
        "--verifier-identity",
        "/verifier/identity",
        "--expected-verifier-sha256",
        "0000000000000000000000000000000000000000000000000000000000000000",
        "--expected-verifier-bytes",
        "1",
    ];
    for operation in ["exact-run", "exact-verify"] {
        let options = parse_options(operation, common.into_iter().map(OsString::from))
            .expect("exact-only options");
        assert_eq!(options.tier, Tier::BoundaryExactSmoke);
        assert!(options.latest_assets.as_os_str().is_empty());
        let supplied_latest = common.into_iter().chain(["--latest-assets", "/latest"]);
        assert!(parse_options(operation, supplied_latest.map(OsString::from)).is_err());
    }
    for (operation, expected) in [
        ("pair-run", Tier::BoundarySmoke),
        ("installation-run", Tier::CandidateInstallation),
    ] {
        assert!(parse_options(operation, common.into_iter().map(OsString::from)).is_err());
        let options = parse_options(
            operation,
            common
                .into_iter()
                .chain(["--latest-assets", "/latest"])
                .map(OsString::from),
        )
        .expect("both routes");
        assert_eq!(options.tier, expected);
        let obsolete =
            common
                .into_iter()
                .chain(["--latest-assets", "/latest", "--publication", "release"]);
        assert!(parse_options(operation, obsolete.map(OsString::from)).is_err());
    }
    assert!(super::super::command([OsString::from("run")].into_iter()).is_err());
}

#[test]
fn completed_child_or_forged_passed_summary_cannot_accept_missing_boundary_work() {
    let fixture = tempfile::tempdir().expect("owned fixture");
    let members = fixture_members(fixture.path());
    let exact = tempfile::tempdir().expect("exact fixture");
    let latest = tempfile::tempdir().expect("latest fixture");
    let options = PairOptions {
        verify: false,
        tier: Tier::BoundarySmoke,
        exact_assets: pack(exact.path(), &members),
        latest_assets: pack(latest.path(), &members),
        tag: format!("v{}", lkjscript::PRODUCT_VERSION),
        commit: "0".repeat(40),
        acquisition: Acquisition::Simulated,
        evidence_root: fixture.path().join("evidence"),
        verifier_identity: fixture.path().join("identity.json"),
        expected_verifier_sha256: "0".repeat(64),
        expected_verifier_bytes: 1,
    };
    let executable = fixture.path().join("verifier");
    fs::copy("/bin/true", &executable).expect("bounded genuine successful process");
    fs::write(&options.verifier_identity, b"diagnostic-only").expect("handoff input");
    fs::create_dir(&options.evidence_root).expect("owned evidence");
    let child = process::run(
        &process::ProcessSpec {
            command: vec![executable.display().to_string()],
            cwd: fixture.path().to_path_buf(),
            environment: BTreeMap::new(),
            timeout: Duration::from_secs(5),
            maximum_stdout_bytes: 1024,
            maximum_stderr_bytes: 1024,
            stdout_path: fixture.path().join("child.stdout"),
            stderr_path: fixture.path().join("child.stderr"),
            unavailable_exit_code: None,
        },
        fixture.path(),
    );
    assert_eq!(child.status, process::ProcessStatus::Passed);
    for cancelled in [false, true] {
        let mut receipt =
            initial_receipt(&options, &executable).expect("incomplete producer record");
        receipt.cleanup_complete = false;
        record_result(&mut receipt, Ok(()), cancelled);
        assert_eq!(receipt.status, Status::Failed);
        assert!(validate(&receipt, &options, &executable).is_err());
    }
    let mut forged = initial_receipt(&options, &executable).expect("incomplete producer record");
    forged.cleanup_complete = true;
    record_result(&mut forged, Ok(()), false);
    forged.completed_unix_nanoseconds = Some(forged.started_unix_nanoseconds);
    persist(&options, &forged).expect("forged passed JSON");
    assert!(read_controlled(&options, &executable, None).is_err());
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
        "wrong-executable-digest",
        "unsupported-host",
        "missing-tools",
        "duplicate-prefix",
    ] {
        let curl = match mode {
            "transfer-failure" => "#!/bin/sh\nexit 22\n".to_owned(),
            "partial" => "#!/bin/sh\nwhile [ \"$1\" != --output ]; do shift; done\nprintf partial > \"$2\"\nexit 18\n".to_owned(),
            "wrong-length" => "#!/bin/sh\nwhile [ \"$1\" != --output ]; do shift; done\nprintf short > \"$2\"\n".to_owned(),
            "wrong-digest" => "#!/bin/sh\nwhile [ \"$1\" != --output ]; do shift; done\nhead -c \"$FIXTURE_LENGTH\" /dev/zero > \"$2\"\n".to_owned(),
            "wrong-executable-digest" => "#!/bin/sh\nwhile [ \"$1\" != --output ]; do shift; done\ncp \"$FIXTURE_ARCHIVE\" \"$2\"\n".to_owned(),
            _ => "#!/bin/sh\nexit 99\n".to_owned(),
        };
        let curl_path = fixture.join("curl");
        fs::write(&curl_path, curl).expect("curl fixture");
        fs::set_permissions(&curl_path, fs::Permissions::from_mode(0o755)).expect("mode");
        let tar = fixture.join("tar");
        if mode == "wrong-executable-digest" {
            fs::write(
                &tar,
                "#!/bin/sh\nhead -c \"$FIXTURE_EXECUTABLE_LENGTH\" /dev/zero\n",
            )
            .expect("changed extracted bytes");
            fs::set_permissions(&tar, fs::Permissions::from_mode(0o755)).expect("mode");
        } else if tar.exists() {
            fs::remove_file(&tar).expect("remove extraction fixture");
        }
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
            .env("FIXTURE_ARCHIVE", assets.join(archive::ARCHIVE_NAME))
            .env(
                "FIXTURE_EXECUTABLE_LENGTH",
                members[1].bytes.len().to_string(),
            )
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
        if mode == "wrong-executable-digest" {
            assert!(String::from_utf8_lossy(&output.stderr).contains("candidate SHA-256 mismatch"));
        }
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
