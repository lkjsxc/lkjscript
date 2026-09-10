//! Source-bound falsification of an actual completed target admission, through its frozen reader.
use super::*;

#[test]
#[ignore = "requires LKJSCRIPT_NOMINAL_ADMISSION_FIXTURE from fresh exact target admission"]
fn live_nominal_receipt_omissions_reject_at_target_admission() {
    let root = PathBuf::from(
        std::env::var_os("LKJSCRIPT_NOMINAL_ADMISSION_FIXTURE")
            .expect("explicit owned admission fixture"),
    );
    assert!(root.is_absolute());
    let path = root.join("receipt.json");
    let original = process::read_bounded(&path, MAXIMUM_RECEIPT_BYTES).expect("actual receipt");
    let baseline: TargetAdmissionReceipt =
        serde_json::from_slice(&original).expect("typed receipt");
    let scratch = tempfile::Builder::new()
        .prefix("nominal-receipt-faults-")
        .tempdir_in(&root)
        .expect("owned bounded logs");
    let mut ordinal = 0_u64;
    let mut invoke = || {
        ordinal = ordinal.checked_add(1).expect("bounded fault count");
        let spec = ProcessSpec {
            command: vec![
                baseline.verifier.path.clone(),
                "release".into(),
                "admission-verify".into(),
                "--candidate".into(),
                baseline.candidate.path.clone(),
                "--receipt".into(),
                path.display().to_string(),
                "--commit".into(),
                baseline.source_commit.clone(),
            ],
            cwd: root.clone(),
            environment: process::environment(),
            timeout: Duration::from_secs(120),
            maximum_stdout_bytes: 128 * 1024,
            maximum_stderr_bytes: 128 * 1024,
            stdout_path: scratch.path().join(format!("{ordinal}.stdout")),
            stderr_path: scratch.path().join(format!("{ordinal}.stderr")),
            unavailable_exit_code: None,
        };
        let observation = process::run(&spec, scratch.path());
        let error = process::read_bounded(&spec.stderr_path, 128 * 1024).expect("bounded error");
        (
            observation.status,
            String::from_utf8(error).expect("UTF-8 error"),
        )
    };
    assert_eq!(
        invoke().0,
        ProcessStatus::Passed,
        "actual admission baseline"
    );
    let mut results = Vec::new();
    // Fixed required inventory independent of the production ORACLES array.
    for name in [
        "distributed_http",
        "outbound_http",
        "offline_packages",
        "pure_tail",
        "stateful_http",
        "service_acceptance",
    ] {
        let mut fault = baseline.clone();
        fault.oracles.retain(|item| item.name != name);
        assert_eq!(fault.oracles.len() + 1, baseline.oracles.len());
        fs::write(
            &path,
            evidence::encode_json(&fault).expect("canonical fault"),
        )
        .expect("write owned aggregate");
        let rejected = invoke();
        fs::write(&path, &original).expect("restore aggregate before assertion");
        assert_eq!(rejected.0, ProcessStatus::Failed, "missing oracle {name}");
        results.push(
            serde_json::json!({"fault":format!("omit-oracle/{name}"),"rejection":rejected.1}),
        );
    }
    for (role, pointers) in [
        (
            Oracle::OfflinePackages,
            vec![
                "/nominal/generic_declarations",
                "/nominal/inspections",
                "/nominal/inspections/batch",
                "/nominal/inspections/edit",
                "/nominal/inspections/snapshot",
                "/nominal/inspections/pair",
                "/nominal/inspections/snapshot-definition",
                "/nominal/producer_removed_before_execution",
                "/nominal/results/i64-snapshot",
                "/nominal/results/i64-retention",
                "/nominal/results/heterogeneous",
                "/nominal/results/pair-reordered-arguments",
                "/nominal/maximum_map_items",
                "/nominal/maximum_map_sum",
                "/nominal/changed_body_result",
                "/nominal/session/messages",
                "/nominal/state_steps",
                "/nominal/retained_states",
                "/nominal/session_rejections",
                "/nominal/replacement_library_revision",
                "/nominal/replacement_result",
                "/nominal/replacement_keep_result",
                "/observations/artifact-nominal-changed-body-exact",
                "/observations/artifact-nominal-reordered-arguments-exact",
                "/observations/artifact-nominal-replaced-template-case-bound-exact",
            ],
        ),
        (
            Oracle::PureTail,
            vec![
                "/outcomes/checked-value-nominal-forward-matrix",
                "/outcomes/checked-value-nominal-bound-matrix",
                "/outcomes/checked-value-nominal-bound-matrix/0/preparation/production_steps",
                "/outcomes/checked-value-nominal-bound-matrix/0/preparation/reference_steps",
                "/outcomes/checked-value-nominal-bound-matrix/0/preparation/production_bytes",
                "/outcomes/checked-value-nominal-bound-matrix/0/preparation/reference_bytes",
                "/outcomes/nominal_typed_data/complete_batch_and_edit",
                "/outcomes/nominal_typed_data/independent_layout_and_payload",
                "/outcomes/nominal_typed_data/expected_sha256",
            ],
        ),
    ] {
        let child_path = root.join(role.name()).join("receipt.json");
        let child_original = process::read_bounded(&child_path, MAXIMUM_RECEIPT_BYTES)
            .expect("bounded actual child receipt");
        let child: Value = serde_json::from_slice(&child_original).expect("child JSON");
        for pointer in pointers {
            let mut fault = child.clone();
            let selected = fault
                .pointer_mut(pointer)
                .expect("required independent observation");
            *selected = match selected {
                Value::Bool(_) => serde_json::json!(false),
                Value::Number(_) => serde_json::json!(0),
                Value::String(_) => serde_json::json!("foreign"),
                Value::Array(_) => serde_json::json!([]),
                Value::Object(_) => serde_json::json!({}),
                _ => panic!("unsupported omission {pointer}"),
            };
            let bytes = match role {
                Oracle::OfflinePackages => {
                    crate::offline_packages::encode_transferred_test_fixture(fault)
                }
                Oracle::PureTail => crate::pure_tail::encode_transferred_test_fixture(fault),
                _ => panic!("unexpected nominal owner"),
            }
            .expect("canonical typed omission");
            fs::write(&child_path, bytes).expect("write owned child");
            let mut aggregate = baseline.clone();
            aggregate
                .oracles
                .iter_mut()
                .find(|item| item.name == role.admission_name())
                .expect("named oracle")
                .receipt = external_evidence(&child_path).expect("recomputed child hash");
            fs::write(
                &path,
                evidence::encode_json(&aggregate).expect("canonical aggregate"),
            )
            .expect("write rebound aggregate");
            let rejected = invoke();
            fs::write(&child_path, &child_original).expect("restore child before assertion");
            fs::write(&path, &original).expect("restore aggregate before assertion");
            assert_eq!(
                rejected.0,
                ProcessStatus::Failed,
                "{} {pointer}",
                role.name()
            );
            results.push(serde_json::json!({"role":role.name(),"fault":pointer,"recomputed_outer_hash":true,"rejection":rejected.1}));
        }
        assert_eq!(
            process::read_bounded(&child_path, MAXIMUM_RECEIPT_BYTES).expect("restored child"),
            child_original
        );
    }
    assert_eq!(
        invoke().0,
        ProcessStatus::Passed,
        "healthy recovery after all faults"
    );
    assert_eq!(
        process::read_bounded(&path, MAXIMUM_RECEIPT_BYTES).expect("restored aggregate"),
        original
    );
    assert_eq!(results.len(), 40, "complete nominal target fault inventory");
    scratch.close().expect("owned log cleanup");
    archive::write_new(&root.join("nominal-receipt-faults.json"), &evidence::encode_json(&serde_json::json!({
        "schema":"lkjscript-nominal-target-receipt-faults-1",
        "source_commit":baseline.source_commit,
        "candidate_sha256":baseline.candidate.sha256,
        "verifier_sha256":baseline.verifier.sha256,
        "original_receipt_sha256":archive::sha256_bytes(&original).expect("original receipt hash"),
        "baseline_and_recovery_passed":true,"restored":true,"cleanup_complete":true,
        "faults":results
    })).expect("bounded summary"), 0o600).expect("new fault evidence");
}
