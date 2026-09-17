use super::*;
use lkjscript::platform::control::{parse_records, render_record};

/// Feedback for changed boundary authoring/reader logic, never new-candidate acceptance.
/// The caller supplies owned, absent storage and the frozen public v0.1.38 triple.
#[test]
#[ignore = "requires an explicitly supplied frozen v0.1.38 archive and owned absent evidence root"]
fn live_small_lifecycle_with_frozen_v0138() {
    let source = PathBuf::from(
        std::env::var_os("LKJSCRIPT_LIFECYCLE_FIXTURE_ASSETS").expect("frozen assets"),
    );
    let root = PathBuf::from(
        std::env::var_os("LKJSCRIPT_LIFECYCLE_FIXTURE_ROOT").expect("owned absent root"),
    );
    assert!(source.is_absolute() && root.is_absolute() && !root.exists());
    assert_eq!(
        archive::sha256_file(&source.join(archive::ARCHIVE_NAME))
            .expect("public archive digest")
            .0
            .as_str(),
        "fa10e7afb09c06e4a243f421050a0497ca9acd331cdf34c046650c85aef377aa"
    );
    fs::create_dir(&root).expect("owned evidence");
    let root = root.canonicalize().expect("canonical evidence");
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).expect("private root");
    let assets = root.join("assets");
    fs::create_dir(&assets).expect("owned assets");
    for name in [
        archive::ARCHIVE_NAME,
        archive::CHECKSUM_NAME,
        crate::release::bootstrap::NAME,
    ] {
        fs::copy(source.join(name), assets.join(name)).expect("immutable fixture copy");
    }
    let options = PairOptions {
        verify: false,
        exact_assets: assets,
        latest_assets: PathBuf::new(),
        tag: "v0.1.38".to_owned(),
        commit: "7083f9a6d56ed702017942e100c3696fc6f35308".to_owned(),
        tier: Tier::BoundaryExactSmoke,
        acquisition: Acquisition::Simulated,
        evidence_root: root.clone(),
        verifier_identity: PathBuf::new(),
        expected_verifier_sha256: "0".repeat(64),
        expected_verifier_bytes: 1,
    };
    let route = Route::Exact;
    let route_root = route.root(&options);
    fs::create_dir(&route_root).expect("route");
    fs::set_permissions(&route_root, fs::Permissions::from_mode(0o700)).expect("private route");
    let control = process::ProcessControl::default();
    let admitted = archive::admit_archive(
        &options.exact_assets.join(archive::ARCHIVE_NAME),
        &options.exact_assets.join(archive::CHECKSUM_NAME),
        &route_root,
        &route.extraction(&options),
        &control,
    )
    .expect("genuine historical archive admission");
    assert_eq!(admitted.manifest.source.tagged_commit_sha, options.commit);
    let receipt = run(&options, route, &admitted, &control).expect("lifecycle result retained");
    assert_eq!(
        receipt.status,
        Status::FreshPassed,
        "{}",
        receipt.failure.as_deref().unwrap_or("no diagnostic")
    );
    validate(&options, route, &admitted, &receipt).expect("maintained original reader");
    for fault in ["omitted-command", "cleanup-failed", "missing-installation"] {
        let mut changed = receipt.clone();
        match fault {
            "omitted-command" => {
                changed.commands.pop();
            }
            "cleanup-failed" => changed.cleanup_complete = false,
            _ => changed.installation = None,
        }
        evidence::publish_json(&route_root.join("lifecycle.json"), &changed)
            .expect("owned forged producer");
        assert!(
            validate(&options, route, &admitted, &changed).is_err(),
            "{fault}"
        );
    }
    evidence::publish_json(&route_root.join("lifecycle.json"), &receipt).expect("restore original");
    let rejected = route_root.join("rejected.lkjc");
    let original = fs::read(&rejected).expect("literal rejected edit");
    fs::write(&rejected, b"changed retained request\n").expect("owned falsification");
    assert!(validate(&options, route, &admitted, &receipt).is_err());
    fs::write(&rejected, original).expect("restore exact original");
    validate(&options, route, &admitted, &receipt).expect("restored original recovers");
    println!(
        "{}",
        serde_json::json!({"fixture": "frozen-public-v0.1.38-small-lifecycle", "root": root,
        "source": options.commit, "commands": receipt.commands.len(), "broad_application_invocations": 0,
        "product_builds": 0, "elapsed_nanoseconds": receipt.elapsed_nanoseconds,
        "created": receipt.structural.as_ref().expect("structural").created_value,
        "edited": receipt.structural.as_ref().expect("structural").replaced_value,
        "new_candidate_acceptance": false })
    );
}

fn output(target: &str, result: &str, differential: &str) -> Vec<CompactRecord> {
    let authority =
        render_record("authority", &[("revision", "reader-fixture")]).expect("fixture authority");
    let execution = render_record(
        "execution",
        &[
            ("target", target),
            ("value", result),
            ("differential", differential),
        ],
    )
    .expect("fixture result");
    parse_records(
        "independent-output-fixture",
        format!("{authority}{execution}").as_bytes(),
    )
    .expect("well-formed fixture output")
}

#[test]
fn f64_pair_reader_requires_literal_inputs_and_independent_result_bits() {
    let root = tempfile::tempdir().expect("owned reader fixture");
    let members = super::super::tests::fixture_members(root.path());
    let manifest: ReleaseManifest = serde_json::from_slice(&members[4].bytes).expect("manifest");
    let mut progress = Progress {
        revision: Some("reader-fixture".to_owned()),
        ..Progress::default()
    };
    for (command, target, result, file, literal) in [
        (
            "numerical-run-created",
            "numerical",
            "1.75",
            "numerical-input.json",
            NUMERICAL_INPUT,
        ),
        (
            "numerical-run-negative-zero",
            "number-text",
            "\"-0.0\"",
            "numerical-negative-zero.json",
            NEGATIVE_ZERO_INPUT,
        ),
    ] {
        let original = output(target, result, "equal");
        assert!(
            progress
                .observe(root.path(), command, &original, &manifest)
                .is_err(),
            "missing original {file}"
        );
        let path = root.path().join(file);
        archive::write_new(&path, literal.as_bytes(), 0o644).expect("original literal input");
        progress
            .observe(root.path(), command, &original, &manifest)
            .expect("original numerical observation");
        let changed = if file == "numerical-input.json" {
            "[1.25]"
        } else {
            "[0.0]"
        };
        fs::write(&path, changed).expect("alter owned literal input");
        // Even a freshly recomputed file binding cannot replace the literal the command used.
        binding(&path).expect("changed input has a valid current binding");
        assert!(
            progress
                .observe(root.path(), command, &original, &manifest)
                .is_err(),
            "changed original {file}"
        );
        fs::write(&path, literal).expect("restore exact original");
    }
    progress
        .observe(
            root.path(),
            "numerical-run-replaced",
            &output("numerical", "2.75", "equal"),
            &manifest,
        )
        .expect("independently fixed edited result");
    assert_eq!(progress.numerical_created_bits, Some(0x3ffc_0000_0000_0000));
    assert_eq!(
        progress.numerical_replaced_bits,
        Some(0x4006_0000_0000_0000)
    );
    assert_eq!(progress.negative_zero_text.as_deref(), Some("-0.0"));
    for (command, target, result, differential) in [
        ("numerical-run-created", "numerical", "1.5", "equal"),
        (
            "numerical-run-replaced",
            "numerical",
            "2.7500000000000004",
            "equal",
        ),
        ("numerical-run-replaced", "numerical", "\"2.75\"", "equal"),
        ("numerical-run-replaced", "numerical", "null", "equal"),
        ("numerical-run-replaced", "foreign", "2.75", "equal"),
        ("numerical-run-replaced", "numerical", "2.75", "not-run"),
        (
            "numerical-run-negative-zero",
            "number-text",
            "\"0.0\"",
            "equal",
        ),
    ] {
        assert!(
            progress
                .observe(
                    root.path(),
                    command,
                    &output(target, result, differential),
                    &manifest
                )
                .is_err(),
            "independently rejected {command}/{target}/{result}/{differential}",
        );
    }
    root.close().expect("owned reader fixture cleanup");
}
