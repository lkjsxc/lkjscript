//! Source-bound falsification of an actual completed target admission, through its frozen reader.
use super::*;

// Faults touch authenticated originals only while the reader runs. Restore every remembered
// file's bytes and permissions on ordinary exits and assertion unwinding, including failures
// before a reader starts or after an omission fault removes a private review plan.
struct OriginalFile {
    bytes: Vec<u8>,
    permissions: fs::Permissions,
}

struct OriginalFiles(BTreeMap<PathBuf, OriginalFile>);

impl OriginalFiles {
    fn remember(&mut self, path: &Path, bytes: &[u8]) {
        let metadata = fs::symlink_metadata(path).expect("owned original file metadata");
        assert!(metadata.is_file(), "owned original must be a regular file");
        if let Some(original) = self.0.get(path) {
            assert_eq!(original.bytes, bytes, "fault reused an unrestored original");
            assert_eq!(
                original.permissions.mode(),
                metadata.permissions().mode(),
                "fault reused unrestored original permissions"
            );
        } else {
            self.0.insert(
                path.to_path_buf(),
                OriginalFile {
                    bytes: bytes.to_vec(),
                    permissions: metadata.permissions(),
                },
            );
        }
    }

    fn restore(&self) -> std::io::Result<()> {
        let mut failure = None;
        for (path, original) in &self.0 {
            if let Err(error) = fs::write(path, &original.bytes)
                .and_then(|()| fs::set_permissions(path, original.permissions.clone()))
            {
                failure = Some(error);
            }
        }
        failure.map_or(Ok(()), Err)
    }
}

impl Drop for OriginalFiles {
    fn drop(&mut self) {
        if let Err(error) = self.restore() {
            eprintln!("failed to restore owned admission originals: {error}");
        }
    }
}

fn rebind_offline_file(child: &mut Value, root: &Path, name: &str) {
    let proof = serde_json::to_value(
        evidence::proof(&root.join(name), name.to_owned()).expect("recomputed retained file proof"),
    )
    .expect("file proof JSON");
    *child["files"]
        .as_array_mut()
        .expect("offline file inventory")
        .iter_mut()
        .find(|file| file["path"] == name)
        .expect("retained fault file belongs to original inventory") = proof.clone();
    for command in child["commands"].as_array_mut().expect("offline commands") {
        for stream in ["stdout", "stderr"] {
            if command["observation"][stream]["path"] == name {
                command["observation"][stream] = proof.clone();
            }
        }
    }
}

fn replace_record_field(input: &str, operation: &str, field: &str, value: &str) -> String {
    let mut records =
        parse_records("owned-receipt-fault", input.as_bytes()).expect("original public records");
    let record = records
        .iter_mut()
        .find(|record| record.operation == operation)
        .expect("independent required public record");
    record
        .fields
        .iter_mut()
        .find(|item| item.name == field)
        .expect("independent required public field")
        .value = value.to_owned();
    records
        .iter()
        .map(|record| {
            let fields = record
                .fields
                .iter()
                .map(|item| (item.name.as_str(), item.value.as_str()))
                .collect::<Vec<_>>();
            lkjscript::platform::control::render_record(&record.operation, &fields)
                .expect("canonical hostile public record")
        })
        .collect()
}

#[test]
fn receipt_fault_originals_restore_on_assertion_unwind() {
    let root = tempfile::tempdir().expect("owned restoration fixture");
    let path = root.path().join("receipt.json");
    let plan = root.path().join("review.lkjplan");
    fs::write(&path, b"original\n").expect("write restoration original");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640))
        .expect("set original receipt permissions");
    fs::write(&plan, b"original plan\n").expect("write private review original");
    fs::set_permissions(&plan, fs::Permissions::from_mode(0o600))
        .expect("set original private review permissions");
    let result = std::panic::catch_unwind(|| {
        let mut originals = OriginalFiles(BTreeMap::new());
        originals.remember(&path, b"original\n");
        originals.remember(&plan, b"original plan\n");
        fs::write(&path, b"fault\n").expect("write owned fault");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .expect("alter existing receipt permissions");
        fs::remove_file(&plan).expect("withhold private review original");
        fs::write(&plan, b"recreated fault\n").expect("recreate owned review fault");
        fs::set_permissions(&plan, fs::Permissions::from_mode(0o644))
            .expect("set recreated permissions independently of process umask");
        panic!("deliberate failure before explicit restoration");
    });
    assert!(result.is_err());
    assert_eq!(fs::read(&path).expect("restored original"), b"original\n");
    assert_eq!(
        fs::read(&plan).expect("restored private review"),
        b"original plan\n"
    );
    assert_eq!(
        fs::metadata(&plan)
            .expect("restored private review metadata")
            .permissions()
            .mode()
            & 0o7777,
        0o600,
        "private review permissions must survive removal and recreation"
    );
    assert_eq!(
        fs::metadata(&path)
            .expect("restored receipt metadata")
            .permissions()
            .mode()
            & 0o7777,
        0o640,
        "existing receipt permissions must be restored after a mode change"
    );
}

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
    let mut originals = OriginalFiles(BTreeMap::new());
    originals.remember(&path, &original);
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
        originals
            .restore()
            .expect("restore originals before assertion");
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
                "/recursive/results/small-leaves",
                "/recursive/results/retained",
                "/recursive/results/flip-next",
                "/recursive/scale_leaves",
                "/recursive/scale_leaves/1",
                "/recursive/scale_shape",
                "/recursive/scale_sum",
                "/recursive/mapped_scale_sum",
                "/recursive/changed_sum",
                "/recursive/library_revision",
                "/recursive/changed_library_revision",
                "/recursive/session/messages",
                "/recursive/schemas",
                "/recursive/persistence",
                "/recursive/persisted_bytes_sha256",
                "/recursive/resources",
                "/recursive/transaction_cancellation",
                "/effects/iteration",
                "/effects/iteration/rounds/0",
                "/effects/iteration/rounds/3",
                "/effects/iteration/rounds/8",
                "/effects/iteration/rounds/9",
                "/effects/resources/iteration/rows",
                "/effects/resources/iteration/authority",
                "/effects/resources/iteration/rows/4/events",
                "/effects/resources/iteration/rows/4/work/maximum_live_locals",
                "/effects/resources/iteration/rows/24/work/maximum_live_effect_bindings",
                "/effects/public_rejections",
                "/effects/definitions",
                "/effects/foreground/consumers",
                "/effects/foreground/cells",
                "/effects/foreground/loops",
                "/effects/foreground/failures",
                "/observations/artifact-nominal-changed-body-exact",
                "/observations/artifact-nominal-reordered-arguments-exact",
                "/observations/artifact-nominal-replaced-template-case-bound-exact",
                "/observations/finite_commands",
                "/observations/finite_artifact_rejections",
                "/observations/finite_installed_upgrade_commands",
                "/observations/requirement_commands",
                "/observations/requirement_sources_removed",
                "/observations/requirement_retry",
                "/observations/requirement_packages",
                "/observations/requirement_predecessor",
                "/observations/requirement_transaction_predecessor",
                "/observations/requirement_conflict",
                "/observations/requirement_queue",
                "/observations/requirement_unencodable",
                "/observations/requirement_structural",
                "/observations/requirement_reject_arity",
                "/observations/requirement_reject_scope",
                "/observations/requirement_reject_minimum-operation",
                "/observations/requirement_reject_forwarding",
                "/observations/requirement_reject_interface",
                "/observations/requirement_reject_callback-row",
                "/observations/f64",
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
        originals.remember(&child_path, &child_original);
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
            originals
                .restore()
                .expect("restore originals before assertion");
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
    // Nested structural evidence is stored as a string in the existing observation map. Decode
    // that string before changing each required discriminant; rebinding all containing hashes
    // forces the frozen owner to judge the evidence rather than reject an old outer checksum.
    let child_root = root.join(Oracle::OfflinePackages.name());
    let child_path = child_root.join("receipt.json");
    let child_original = process::read_bounded(&child_path, MAXIMUM_RECEIPT_BYTES)
        .expect("restored offline receipt");
    originals.remember(&child_path, &child_original);
    let child: Value = serde_json::from_slice(&child_original).expect("offline JSON");
    let structural: Value = serde_json::from_str(
        child["observations"]["requirement_structural"]
            .as_str()
            .expect("structural observation string"),
    )
    .expect("nested structural observation");
    let update_before = structural["update_before"]
        .as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .expect("original update inspection");
    let update_after = structural["update_after"]
        .as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .expect("edited update inspection");
    let before_name = format!("command-{update_before:04}.stdout");
    let after_name = format!("command-{update_after:04}.stdout");
    let requirement_commands: Vec<usize> = serde_json::from_str(
        child["observations"]["requirement_commands"]
            .as_str()
            .expect("completed outcome invocation inventory"),
    )
    .expect("completed outcome indices");
    let condition_name = format!("command-{:04}.stdout", requirement_commands[9]);
    let predecessor_artifact = "requirement-transaction-predecessor--predecessor.lkja";
    for name in [
        "requirement-consumer-structural.lkjplan",
        "requirement-supplier-structural.lkjplan",
        "requirement-consumer-structural.lkjc",
        "requirement-store-observations.json",
        predecessor_artifact,
        &condition_name,
        &before_name,
        &after_name,
    ] {
        let bytes = process::read_bounded(&child_root.join(name), MAXIMUM_RECEIPT_BYTES)
            .expect("original retained structural bytes");
        originals.remember(&child_root.join(name), &bytes);
    }
    let numerical: Value = serde_json::from_str(
        child["observations"]["f64"]
            .as_str()
            .expect("numerical observation string"),
    )
    .expect("nested numerical observation");
    let numerical_command = |name: &str| {
        numerical["calls"]
            .as_array()
            .expect("numerical invocation inventory")
            .iter()
            .find(|call| call["name"] == name)
            .expect("independently selected numerical case")["command"]
            .as_u64()
            .and_then(|index| usize::try_from(index).ok())
            .expect("numerical command index")
    };
    let numerical_scale = format!("command-{:04}.stdout", numerical_command("scale"));
    let numerical_small = format!("command-{:04}.stdout", numerical_command("direct-small"));
    let numerical_after = format!(
        "command-{:04}.stdout",
        numerical["inspect_after"]
            .as_u64()
            .expect("edited calibration inspection")
    );
    let numerical_originals = [
        "f64-producer.lkjc",
        "f64-consumer.lkjc",
        "f64-edit.lkjc",
        "f64-producer.lkjplan",
        "f64-consumer.lkjplan",
        "f64-edit.lkjplan",
        "f64-input-scale.json",
        "f64-input-resume.json",
        "f64-checkpoint.json",
    ];
    for name in numerical_originals.into_iter().chain([
        "f64-input-direct-small.json",
        "f64-after.lkja",
        numerical_scale.as_str(),
        numerical_small.as_str(),
        numerical_after.as_str(),
    ]) {
        let bytes = process::read_bounded(&child_root.join(name), MAXIMUM_RECEIPT_BYTES)
            .expect("original retained numerical bytes");
        originals.remember(&child_root.join(name), &bytes);
    }
    let numerical_before_artifact =
        process::read_bounded(&child_root.join("f64-before.lkja"), MAXIMUM_RECEIPT_BYTES)
            .expect("original valid artifact before projection replacement");
    assert_ne!(
        numerical_before_artifact,
        fs::read(child_root.join("f64-after.lkja")).expect("original edited artifact"),
        "projection edit produced distinct artifact bytes"
    );
    let mut offline_fault = |fault: Value, label: &str, reason: Option<&str>| {
        fs::write(
            &child_path,
            crate::offline_packages::encode_transferred_test_fixture(fault)
                .expect("canonical offline original evidence fault"),
        )
        .expect("write owned child fault");
        let mut aggregate = baseline.clone();
        aggregate
            .oracles
            .iter_mut()
            .find(|item| item.name == Oracle::OfflinePackages.admission_name())
            .expect("offline aggregate owner")
            .receipt = external_evidence(&child_path).expect("rebound child proof");
        fs::write(
            &path,
            evidence::encode_json(&aggregate).expect("canonical rebound aggregate"),
        )
        .expect("write aggregate fault");
        let rejected = invoke();
        originals
            .restore()
            .expect("restore all originals before fault assertion");
        assert_eq!(
            rejected.0,
            ProcessStatus::Failed,
            "offline original fault {label}"
        );
        if let Some(reason) = reason {
            assert!(
                rejected.1.contains(reason),
                "offline original fault {label} rejected outside its intended owner: {}",
                rejected.1
            );
        }
        results.push(serde_json::json!({"role":"offline-packages","fault":label,
            "recomputed_outer_hash":true,"rejection":rejected.1}));
    };
    for pointer in [
        "/producer/commands",
        "/producer/revision",
        "/consumer/commands",
        "/consumer/revision",
        "/supplier/commands",
        "/supplier/revision",
        "/factory_before",
        "/factory_after",
        "/update_before",
        "/update_after",
    ] {
        let mut observation = structural.clone();
        *observation
            .pointer_mut(pointer)
            .expect("required nested structural field") = Value::Null;
        let mut fault = child.clone();
        fault["observations"]["requirement_structural"] = Value::String(observation.to_string());
        offline_fault(fault, &format!("structural/omit{pointer}"), None);
    }
    let mut observation = structural.clone();
    observation["producer"]["commands"] = structural["consumer"]["commands"].clone();
    let mut fault = child.clone();
    fault["observations"]["requirement_structural"] = Value::String(observation.to_string());
    offline_fault(
        fault,
        "structural/foreign-producer-commands",
        Some("plan changed executable"),
    );

    let mut observation = structural.clone();
    observation["supplier"]["commands"]
        .as_array_mut()
        .expect("supplier commands")
        .swap(0, 1);
    let mut fault = child.clone();
    fault["observations"]["requirement_structural"] = Value::String(observation.to_string());
    offline_fault(
        fault,
        "structural/reordered-supplier-plans",
        Some("plans/apply missing or reordered"),
    );

    let apply_index = structural["supplier"]["commands"][2]
        .as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .expect("supplier apply command");
    let mut fault = child.clone();
    *fault["commands"][apply_index]["command"]
        .as_array_mut()
        .expect("supplier command arguments")
        .last_mut()
        .expect("apply token") = serde_json::json!("foreign");
    offline_fault(
        fault,
        "structural/foreign-flat-plan-token",
        Some("did not consume the equivalent flat plan token"),
    );

    let name = "requirement-consumer-structural.lkjplan";
    fs::write(
        child_root.join(name),
        process::read_bounded(
            &child_root.join("requirement-producer-flat.lkjplan"),
            MAXIMUM_RECEIPT_BYTES,
        )
        .expect("valid foreign review bytes"),
    )
    .expect("substitute one valid review");
    let mut fault = child.clone();
    rebind_offline_file(&mut fault, &child_root, name);
    offline_fault(
        fault,
        "structural/foreign-valid-review",
        Some("token is not bound to the strict reviewed bytes"),
    );

    let name = "requirement-supplier-structural.lkjplan";
    let input = fs::read_to_string(child_root.join(name)).expect("restored supplier review");
    let mut removed = false;
    let shortened = input
        .lines()
        .filter(|line| {
            if !removed && line.starts_with("logical-plan.retirement ") {
                removed = true;
                false
            } else {
                true
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    assert!(removed, "supplier review must retire replaced body owners");
    fs::write(child_root.join(name), shortened).expect("omit one reviewed retirement");
    let mut fault = child.clone();
    rebind_offline_file(&mut fault, &child_root, name);
    offline_fault(
        fault,
        "structural/omitted-reviewed-retirement",
        Some("logical plan"),
    );

    let before =
        fs::read_to_string(child_root.join(&before_name)).expect("original update definition");
    let altered = replace_record_field(
        &before,
        "definition.expression",
        "id",
        "expr_ffffffffffffffffffffffffffffff",
    );
    assert_ne!(before, altered, "counterfeit owner must change observation");
    fs::write(child_root.join(&before_name), altered).expect("counterfeit old body owner");
    let mut fault = child.clone();
    rebind_offline_file(&mut fault, &child_root, &before_name);
    offline_fault(
        fault,
        "structural/unretired-former-body-owner",
        Some("omitted retirement of a former body owner"),
    );

    let after = fs::read_to_string(child_root.join(&after_name)).expect("edited update definition");
    let records = parse_records("edited-definition", after.as_bytes()).expect("public definition");
    let duplicate = records
        .iter()
        .filter(|record| record.operation == "definition.expression")
        .nth(1)
        .expect("second edited expression")
        .fields
        .iter()
        .find(|field| field.name == "id")
        .expect("expression identity")
        .value
        .clone();
    fs::write(
        child_root.join(&after_name),
        replace_record_field(&after, "definition.expression", "id", &duplicate),
    )
    .expect("duplicate an edited body owner");
    let mut fault = child.clone();
    rebind_offline_file(&mut fault, &child_root, &after_name);
    offline_fault(
        fault,
        "structural/duplicate-edited-body-owner",
        Some("duplicates an expression or binding identity"),
    );

    let after =
        fs::read_to_string(child_root.join(&after_name)).expect("restored edited definition");
    let revision = structural["producer"]["revision"]
        .as_str()
        .expect("original producer revision");
    let altered = replace_record_field(&after, "definition.header", "revision", revision);
    let altered = replace_record_field(&altered, "revision", "observed", revision);
    fs::write(child_root.join(&after_name), altered)
        .expect("substitute an earlier inspection revision");
    let mut fault = child.clone();
    rebind_offline_file(&mut fault, &child_root, &after_name);
    offline_fault(
        fault,
        "structural/foreign-inspection-revision",
        Some("inspection changed function identity, accepted revision"),
    );

    let name = "requirement-consumer-structural.lkjc";
    let input = fs::read_to_string(child_root.join(name)).expect("original outcome literal");
    let completed_literal =
        include_str!("../../offline_packages/requirements.outcomes.consumer.structural.lkjc");
    assert_eq!(input.matches(completed_literal).count(), 1);
    fs::write(
        child_root.join(name),
        input.replacen(completed_literal, "", 1),
    )
    .expect("withhold owned completed outcome literal");
    let mut fault = child.clone();
    rebind_offline_file(&mut fault, &child_root, name);
    offline_fault(
        fault,
        "transaction-outcome/omit-literal-consumer",
        Some("independent literal library request changed"),
    );

    let output = fs::read_to_string(child_root.join(&condition_name))
        .expect("original condition-failed completion");
    fs::write(
        child_root.join(&condition_name),
        replace_record_field(
            &output,
            "execution",
            "value",
            r#"{"case":"Committed","value":19}"#,
        ),
    )
    .expect("forge accepted candidate for a suppressed transaction");
    let mut fault = child.clone();
    rebind_offline_file(&mut fault, &child_root, &condition_name);
    offline_fault(
        fault,
        "transaction-outcome/condition-failed-forged-as-committed",
        Some("requirement result, operation count, or cleanup differs"),
    );

    let name = "requirement-store-observations.json";
    let mut states: Value = serde_json::from_slice(
        &fs::read(child_root.join(name)).expect("original physical store observations"),
    )
    .expect("store observations JSON");
    for row in states.as_array_mut().expect("store rows") {
        row[0]
            .as_object_mut()
            .expect("number store")
            .remove("revision");
    }
    fs::write(
        child_root.join(name),
        evidence::encode_json(&states).expect("omitted physical revisions"),
    )
    .expect("withhold owned number-store HEAD identity");
    let mut fault = child.clone();
    rebind_offline_file(&mut fault, &child_root, name);
    offline_fault(
        fault,
        "transaction-outcome/omit-physical-head-identities",
        Some("independent store revision, complete cells, or entry bytes omitted"),
    );

    fs::remove_file(child_root.join(predecessor_artifact))
        .expect("withhold owned authentic predecessor artifact");
    let mut fault = child.clone();
    fault["files"]
        .as_array_mut()
        .expect("offline retained files")
        .retain(|file| file["path"] != predecessor_artifact);
    offline_fault(
        fault,
        "transaction-outcome/omit-authentic-predecessor-artifact",
        None,
    );

    // Remove each original and its enclosing file-inventory entry together. A consistently
    // rehashed aggregate still needs the literal numerical authoring and checkpoint materials.
    for name in numerical_originals {
        fs::remove_file(child_root.join(name)).expect("withhold owned numerical original");
        let mut fault = child.clone();
        let files = fault["files"]
            .as_array_mut()
            .expect("offline original files");
        let original_count = files.len();
        files.retain(|file| file["path"] != name);
        assert_eq!(
            files.len() + 1,
            original_count,
            "original inventory owns {name}"
        );
        offline_fault(fault, &format!("f64/omit-original/{name}"), None);
    }

    let output = fs::read_to_string(child_root.join(&numerical_scale))
        .expect("original independently expected scale result");
    let records = parse_records("numerical-scale-result", output.as_bytes())
        .expect("original numerical result records");
    let value = records
        .iter()
        .find(|record| record.operation == "execution")
        .expect("numerical execution")
        .fields
        .iter()
        .find(|field| field.name == "value")
        .expect("numerical result value")
        .value
        .as_str();
    let expected_variance = "\"population-variance\":87381.328125";
    assert_eq!(value.matches(expected_variance).count(), 1);
    let changed = value.replacen(expected_variance, "\"population-variance\":87381.5", 1);
    fs::write(
        child_root.join(&numerical_scale),
        replace_record_field(&output, "execution", "value", &changed),
    )
    .expect("alter one scale field without reparsing other binary64 results");
    let mut fault = child.clone();
    rebind_offline_file(&mut fault, &child_root, &numerical_scale);
    offline_fault(
        fault,
        "f64/rehashed-independent-scale-result",
        Some("F64 scale:"),
    );

    // Substitute a coherent alternative workload in both supplied input and command, and make
    // its reported mean agree. The reader must retain the selected independently fixed fixture.
    let name = "f64-input-direct-small.json";
    let input = fs::read_to_string(child_root.join(name)).expect("original small input");
    assert_eq!(input, "[[0.5,1.5,2.5,3.5]]");
    let changed_input = "[[1.5,2.5,3.5,4.5]]";
    fs::write(child_root.join(name), changed_input).expect("replace selected numerical input");
    let output =
        fs::read_to_string(child_root.join(&numerical_small)).expect("original small result");
    let records = parse_records("numerical-small-result", output.as_bytes())
        .expect("original small result records");
    let value = records
        .iter()
        .find(|record| record.operation == "execution")
        .expect("small execution")
        .fields
        .iter()
        .find(|field| field.name == "value")
        .expect("small result value")
        .value
        .as_str();
    assert_eq!(value.matches("\"mean\":2.0").count(), 1);
    let changed = value.replacen("\"mean\":2.0", "\"mean\":3.0", 1);
    fs::write(
        child_root.join(&numerical_small),
        replace_record_field(&output, "execution", "value", &changed),
    )
    .expect("supply corresponding alternative numerical result");
    let mut fault = child.clone();
    *fault["commands"][numerical_command("direct-small")]["command"]
        .as_array_mut()
        .expect("selected invocation arguments")
        .last_mut()
        .expect("literal JSON arguments") = Value::String(changed_input.to_owned());
    rebind_offline_file(&mut fault, &child_root, name);
    rebind_offline_file(&mut fault, &child_root, &numerical_small);
    offline_fault(
        fault,
        "f64/rehashed-substituted-input-command-and-result",
        Some("F64 retained runtime input differs"),
    );

    let output = fs::read_to_string(child_root.join(&numerical_after))
        .expect("original edited calibration definition");
    let records = parse_records("edited-numerical-definition", output.as_bytes())
        .expect("original calibration records");
    let parameter_types = records
        .iter()
        .filter(|record| record.operation == "definition.parameter")
        .map(|record| {
            record
                .fields
                .iter()
                .find(|field| field.name == "type")
                .expect("retained calibration parameter type")
                .value
                .clone()
        })
        .collect::<Vec<_>>();
    assert_eq!(parameter_types.len(), 2);
    assert_ne!(
        parameter_types[0], parameter_types[1],
        "offset and Measurement types differ"
    );
    fs::write(
        child_root.join(&numerical_after),
        replace_record_field(&output, "definition.parameter", "type", &parameter_types[1]),
    )
    .expect("change the retained offset type without changing its identity");
    let mut fault = child.clone();
    rebind_offline_file(&mut fault, &child_root, &numerical_after);
    offline_fault(
        fault,
        "f64/rehashed-calibration-parameter-type",
        Some("F64 projection edit changed its complete retained signature or header"),
    );

    let output = fs::read_to_string(child_root.join(&numerical_after))
        .expect("restored edited calibration definition");
    let previous = numerical["before"]["revision"]
        .as_str()
        .expect("original consumer revision");
    let changed = replace_record_field(&output, "definition.header", "revision", previous);
    let changed = replace_record_field(&changed, "revision", "observed", previous);
    fs::write(child_root.join(&numerical_after), changed)
        .expect("substitute a foreign accepted revision in the edited inspection");
    let mut fault = child.clone();
    rebind_offline_file(&mut fault, &child_root, &numerical_after);
    offline_fault(
        fault,
        "f64/rehashed-calibration-inspection-revision",
        Some("F64 calibration inspection changed accepted revision"),
    );

    let mut observation = numerical.clone();
    assert_ne!(
        observation["standard"]["transport"],
        observation["producer"]["transport"]
    );
    observation["standard"]["transport"] = observation["producer"]["transport"].clone();
    let mut fault = child.clone();
    fault["observations"]["f64"] = Value::String(observation.to_string());
    offline_fault(
        fault,
        "f64/rehashed-foreign-standard-transport",
        Some("F64 standard identity differs from original copied-product discovery"),
    );

    // These are genuine valid artifact bytes from the same workload before its accepted edit.
    // Recompute both file checksums so the independent source binding must reject the swap.
    let name = "f64-after.lkja";
    fs::write(child_root.join(name), &numerical_before_artifact)
        .expect("substitute the original valid preceding artifact");
    let mut fault = child.clone();
    fault["observations"]["artifact-f64-after"] = serde_json::to_value(
        archive::sha256_bytes(&numerical_before_artifact).expect("recomputed artifact SHA-256"),
    )
    .expect("artifact digest JSON");
    rebind_offline_file(&mut fault, &child_root, name);
    offline_fault(
        fault,
        "f64/rehashed-valid-artifact-from-before-edit",
        Some("F64 original artifact/source binding"),
    );

    assert_eq!(
        invoke().0,
        ProcessStatus::Passed,
        "healthy recovery after all faults"
    );
    assert_eq!(
        process::read_bounded(&path, MAXIMUM_RECEIPT_BYTES).expect("restored aggregate"),
        original
    );
    assert_eq!(
        results.len(),
        130,
        "complete nominal, recursive, requirement, and numerical target fault inventory"
    );
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
