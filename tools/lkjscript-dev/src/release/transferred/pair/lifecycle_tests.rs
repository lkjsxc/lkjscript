use super::*;
use lkjscript::platform::control::{parse_records, render_record};

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
