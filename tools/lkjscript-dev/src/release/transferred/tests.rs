use super::*;
use std::io::{Read, Seek, SeekFrom, Write};

fn encode(role: Oracle, value: serde_json::Value) -> Result<Vec<u8>, DevError> {
    match role {
        Oracle::DistributedHttp => distributed_http::encode_transferred_test_fixture(value),
        Oracle::OutboundHttp => outbound_http::encode_transferred_test_fixture(value),
        Oracle::OfflinePackages => offline_packages::encode_transferred_test_fixture(value),
        Oracle::PureTail => pure_tail::encode_transferred_test_fixture(value),
        Oracle::StatefulHttp => stateful_http::encode_transferred_test_fixture(value),
    }
}

#[test]
fn supervised_cancellation_and_timeout_clean_separate_process_groups_then_recover() {
    let root = tempfile::tempdir().expect("owned test root");
    for kill in [false, true] {
        let control = process::ProcessControl::default();
        let spec = process::ProcessSpec {
            command: vec![
                "/bin/sh".to_owned(),
                "-c".to_owned(),
                "setsid sleep 60 & echo $!; wait".to_owned(),
            ],
            cwd: root.path().to_path_buf(),
            environment: BTreeMap::from([("PATH".to_owned(), "/usr/bin:/bin".to_owned())]),
            timeout: Duration::from_millis(350),
            maximum_stdout_bytes: 1024,
            maximum_stderr_bytes: 1024,
            stdout_path: root.path().join(format!("{kill}.stdout")),
            stderr_path: root.path().join(format!("{kill}.stderr")),
            unavailable_exit_code: None,
        };
        let trigger = control.clone();
        let thread = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            if kill {
                trigger.kill();
            }
        });
        let observation = process::run_supervised(&spec, root.path(), Some(&control));
        thread.join().expect("joined cancellation");
        assert_eq!(
            observation.status,
            if kill {
                process::ProcessStatus::Signaled
            } else {
                process::ProcessStatus::Timeout
            },
            "{observation:?}"
        );
        let pid = fs::read_to_string(&spec.stdout_path)
            .expect("child identity")
            .trim()
            .parse::<u32>()
            .expect("pid");
        if let Ok(stat) = fs::read_to_string(format!("/proc/{pid}/stat")) {
            assert!(
                stat.rsplit_once(") ")
                    .expect("proc stat")
                    .1
                    .starts_with('Z')
            );
        }
    }
    let spec = process::ProcessSpec {
        command: vec!["/bin/true".to_owned()],
        cwd: root.path().to_path_buf(),
        environment: BTreeMap::new(),
        timeout: Duration::from_secs(5),
        maximum_stdout_bytes: 1024,
        maximum_stderr_bytes: 1024,
        stdout_path: root.path().join("healthy.stdout"),
        stderr_path: root.path().join("healthy.stderr"),
        unavailable_exit_code: None,
    };
    assert_eq!(
        process::run_supervised(&spec, root.path(), None).status,
        process::ProcessStatus::Passed
    );
}

// Uses an actual separately executed aggregate, never a synthetic success fixture. Run explicitly
// after the no-checkout rehearsal; this is verifier/source-bound falsification, not product behavior.
#[test]
#[ignore = "requires LKJSCRIPT_TRANSFERRED_FIXTURE_ROOT from a fresh complete transfer rehearsal"]
fn live_receipt_fault_matrix() {
    let root = PathBuf::from(
        std::env::var_os("LKJSCRIPT_TRANSFERRED_FIXTURE_ROOT").expect("explicit rehearsal root"),
    );
    let path = root.join("receipt.json");
    let original = fs::read(&path).expect("aggregate bytes");
    let baseline: Receipt = serde_json::from_slice(&original).expect("aggregate");
    let verifier = PathBuf::from(&baseline.verifier.path);
    let options = Options {
        verify: true,
        candidate: PathBuf::from(&baseline.candidate.path),
        manifest: PathBuf::from(&baseline.manifest.path),
        tag: baseline.tag.clone(),
        commit: baseline.source_commit.clone(),
        publication: baseline.publication,
        boundary: baseline.boundary,
        verifier_identity: verifier
            .parent()
            .expect("handoff")
            .join("verifier-identity.json"),
        expected_verifier_sha256: baseline.verifier.sha256.as_str().to_owned(),
        expected_verifier_bytes: baseline.verifier.byte_length,
        evidence_root: root.clone(),
    };
    let manifest = load_manifest(&options).expect("valid extracted manifest");
    read_receipt(&options, &verifier, &manifest).expect("actual unmodified rehearsal passes");
    let mut results = Vec::new();
    let mut reject_aggregate = |name: &str, fault: Receipt| {
        fs::write(
            &path,
            evidence::encode_json(&fault).expect("canonical fault"),
        )
        .expect("write fault");
        let rejected = read_receipt(&options, &verifier, &manifest);
        fs::write(&path, &original).expect("restore original");
        let rejection = rejected.expect_err(name).to_string();
        results.push(serde_json::json!({"fault":name,"rejection":rejection}));
    };
    // Independent required inventory, including the valid-HTTP-only predecessor case.
    for name in [
        "distributed-http",
        "outbound-http",
        "offline-packages",
        "pure-tail",
        "stateful-http",
    ] {
        let mut fault = baseline.clone();
        fault.children.retain(|child| child.role.name() != name);
        reject_aggregate(&format!("omit-{name}"), fault);
    }
    let mut fault = baseline.clone();
    fault.children.retain(|child| {
        matches!(
            child.role,
            Oracle::DistributedHttp | Oracle::OutboundHttp | Oracle::StatefulHttp
        )
    });
    reject_aggregate("old-three-http-only", fault);
    let mut fault = baseline.clone();
    fault.children.push(fault.children[0].clone());
    reject_aggregate("duplicate-extra-role", fault);
    for status in [Status::Failed, Status::Unavailable, Status::NotRun] {
        let mut fault = baseline.clone();
        fault.children[2].status = status;
        reject_aggregate(&format!("child-{status:?}"), fault);
    }
    let mut fault = baseline.clone();
    fault.children[2].cleanup_complete = false;
    reject_aggregate("child-unclean", fault);
    let mut fault = baseline.clone();
    fault.children[2].evidence_root = "/tmp/foreign".to_owned();
    reject_aggregate("foreign-passing-root", fault);
    let mut fault = baseline.clone();
    fault.source_commit = "f".repeat(40);
    reject_aggregate("foreign-source", fault);
    let mut fault = baseline.clone();
    fault.boundary = Boundary::LatestDownload;
    reject_aggregate("foreign-boundary", fault);
    let mut fault = baseline.clone();
    fault.schema.version = 0;
    reject_aggregate("predecessor-aggregate", fault);
    let mut unknown_role = serde_json::to_value(&baseline).expect("aggregate value");
    unknown_role["children"][0]["role"] = serde_json::json!("foreign-role");
    for (name, bytes) in [
        (
            "noncanonical-aggregate",
            serde_json::to_vec(&baseline).expect("compact JSON"),
        ),
        (
            "unknown-extra-role",
            serde_json::to_vec(&unknown_role).expect("unknown role JSON"),
        ),
    ] {
        fs::write(&path, bytes).expect("malformed aggregate");
        let rejected = read_receipt(&options, &verifier, &manifest);
        fs::write(&path, &original).expect("restore before assertion");
        results.push(
            serde_json::json!({"fault":name,"rejection":rejected.expect_err(name).to_string()}),
        );
    }
    // Each pointer is an independent old workflow or mandatory language obligation. Recompute outer
    // hashes after each mutation so the owned validator, rather than digest disagreement, rejects it.
    for (role, pointers) in [
        (
            Oracle::DistributedHttp,
            vec![
                "/schema/version",
                "/workflow",
                "/execution_context",
                "/candidate/sha256",
                "/copied_candidate/sha256",
                "/result/product_version",
                "/result/capabilities_digest",
                "/result/authority_unchanged",
                "/result/clean_incremental_equal",
                "/result/restart_equal",
                "/result/startup_failures_without_ready",
                "/result/responses",
                "/result/topology/method",
                "/result/topology/path",
                "/result/topology/route_count",
                "/result/topology/route",
                "/result/topology/target",
                "/result/topology/component",
                "/result/topology/port",
                "/result/topology/function",
                "/result/topology/route_set",
                "/result/topology/context_owners",
                "/result/topology/context_relations",
                "/result/topology/streams_requirement",
                "/result/topology/predecessor_port_absent",
                "/result/topology/context_complete",
                "/result/definition_projection/function",
                "/result/definition_projection/initial_revision",
                "/result/definition_projection/accepted_revision",
                "/result/definition_projection/initial_digest",
                "/result/definition_projection/accepted_digest",
                "/result/definition_projection/initial_records",
                "/result/definition_projection/accepted_records",
                "/result/definition_projection/initial_pages",
                "/result/definition_projection/accepted_pages",
                "/result/definition_projection/initial_body_records",
                "/result/definition_projection/accepted_body_records",
                "/result/definition_projection/initial_fact_records",
                "/result/definition_projection/accepted_fact_records",
                "/result/definition_projection/initial_logical_bytes",
                "/result/definition_projection/accepted_logical_bytes",
                "/result/definition_projection/initial_output_bytes",
                "/result/definition_projection/accepted_output_bytes",
                "/result/definition_projection/initial_literal_sha256",
                "/result/definition_projection/accepted_literal_sha256",
                "/result/definition_projection/contract_unchanged",
                "/result/definition_projection/intended_body_change",
                "/result/definition_projection/digest_recomputed",
                "/result/definition_projection/changed_page_budgets",
                "/result/definition_projection/direct_file_plan_equal",
                "/result/definition_projection/malformed_continuation_rejected",
                "/result/definition_projection/stale_continuation_rejected",
                "/result/definition_projection/projection_input_rejected",
                "/result/definition_projection/authority_unchanged_before_apply",
                "/cleanup/runner_cleanup_complete",
                "/cleanup/isolated_root_removed",
            ],
        ),
        (
            Oracle::OutboundHttp,
            vec![
                "/schema/version",
                "/workflow",
                "/execution_context",
                "/candidate/sha256",
                "/copied_candidate/sha256",
                "/result/product_version",
                "/result/capabilities_digest",
                "/result/authority_unchanged",
                "/result/clean_incremental_equal",
                "/result/restart_equal",
                "/result/startup_failures_without_ready",
                "/result/responses",
                "/result/no_connection",
                "/result/certificate/generator",
                "/result/topology/method",
                "/result/topology/path",
                "/result/topology/route_count",
                "/result/topology/route",
                "/result/topology/target",
                "/result/topology/component",
                "/result/topology/port",
                "/result/topology/route_set",
                "/result/topology/predecessor_port_absent",
                "/result/topology/predecessor_predicate_absent",
                "/cleanup/runner_cleanup_complete",
                "/cleanup/oracle_cleanup_complete",
                "/cleanup/isolated_root_removed",
                "/cleanup/raw_secret_values_retained",
            ],
        ),
        (
            Oracle::StatefulHttp,
            vec![
                "/schema/version",
                "/workflow",
                "/execution_context",
                "/candidate/sha256",
                "/copied_candidate/sha256",
                "/result/workflow",
                "/result/initial_template",
                "/result/initial_owners",
                "/result/initial_dependencies",
                "/result/dependency_staged",
                "/result/topology/target_name",
                "/result/topology/runner",
                "/result/topology/module",
                "/result/topology/component",
                "/result/topology/target",
                "/result/topology/route_set",
                "/result/topology/exact_routes",
                "/result/topology/pattern_routes",
                "/result/topology/pattern_segments",
                "/result/topology/maximum_specificity_chain",
                "/result/topology/routes",
                "/result/topology/requirements",
                "/result/pattern_lifecycle/set_preserved_identity",
                "/result/pattern_lifecycle/altered_plan_rejected",
                "/result/pattern_lifecycle/stale_plan_rejected",
                "/result/pattern_lifecycle/reviewed_selector_evidence",
                "/result/pattern_lifecycle/temporary_pattern_inspected",
                "/result/pattern_lifecycle/temporary_pattern_deleted",
                "/result/pattern_lifecycle/intermediate_revisions",
                "/result/idempotent_reconciliation",
                "/result/deterministic",
                "/result/incremental_sha256",
                "/result/live/data_contract",
                "/result/live/routes_checked",
                "/result/live/requests",
                "/result/live/exact_over_pattern_precedence",
                "/result/live/ordered_two_captures",
                "/result/live/capture_query_ignored",
                "/result/live/matcher_nodes",
                "/result/live/matcher_step_bound",
                "/result/live/runtime/runs",
                "/result/live/runtime/admitted_tasks",
                "/result/live/runtime/maximum_queued_tasks",
                "/result/live/runtime/maximum_active_tasks",
                "/result/live/runtime/maximum_admission_permits",
                "/result/live/runtime/maximum_worker_permits",
                "/result/live/persistence_after_restart",
                "/result/live/backup_restore_equivalent",
                "/result/live/absent_root_no_ready",
                "/result/live/corrupt_root_no_ready",
                "/result/live/malformed_request_contained",
                "/result/live/startup_failures_without_ready",
                "/result/live/runner_starts",
                "/result/live/shutdown_cleanup_failures",
                "/result/live/temporary_data_cleanup_complete",
                "/result/live/authority_unchanged",
                "/cleanup/temporary_root_removed",
                "/cleanup/data_cleanup_complete",
                "/cleanup/runner_cleanup_complete",
                "/cleanup/raw_secret_values_retained",
            ],
        ),
        (
            Oracle::PureTail,
            vec![
                "/schema",
                "/status",
                "/candidate_sha256",
                "/copied_candidate_sha256",
                "/commands/0/command/0",
                "/verifier_sha256",
                "/evidence_root",
                "/cleanup_complete",
                "/outcomes/capture-safe-constraint-clear",
                "/outcomes/generic-factory-definitions",
                "/outcomes/persistent-map/configured",
                "/outcomes/persistent-map/large_sum",
                "/outcomes/persistent-map/large_length",
                "/outcomes/persistent-map/aliases_unchanged",
                "/outcomes/standalone_http/configured_response",
                "/outcomes/standalone_http/initial_committed_changes",
                "/outcomes/transaction_cancellation/cancelled_key_absent",
                "/outcomes/checked-value-nominal-forward-matrix",
                "/outcomes/checked-value-nominal-bound-matrix",
                "/outcomes/nominal_typed_data/complete_batch_and_edit",
                "/outcomes/nominal_typed_data/expected_sha256",
            ],
        ),
        (
            Oracle::OfflinePackages,
            vec![
                "/schema",
                "/status",
                "/candidate_sha256",
                "/copied_candidate_sha256",
                "/commands/0/command/0",
                "/verifier_sha256",
                "/evidence_root",
                "/cleanup_complete",
                "/observations/generic_capture_factory",
                "/observations/imported_capture_constraint",
                "/observations/rejection-kernel_type_constraint",
                "/observations/fixed_results",
                "/observations/producers_absent_before_execution",
                "/observations/private_factory_map",
                "/nominal/generic_declarations",
                "/nominal/inspections",
                "/nominal/inspections/batch",
                "/nominal/inspections/edit",
                "/nominal/inspections/snapshot",
                "/nominal/inspections/pair",
                "/nominal/inspections/snapshot-definition",
                "/nominal/producer_removed_before_execution",
                "/nominal/results/i64-snapshot",
                "/nominal/maximum_map_items",
                "/nominal/maximum_map_sum",
                "/nominal/changed_body_result",
                "/nominal/session/messages",
                "/nominal/state_steps",
                "/nominal/retained_states",
                "/nominal/session_rejections",
                "/nominal/replacement_library_revision",
                "/nominal/replacement_result",
                "/observations/artifact-nominal-changed-body-exact",
                "/observations/artifact-nominal-reordered-arguments-exact",
                "/observations/artifact-nominal-replaced-template-case-bound-exact",
                "/nominal/results/pair-reordered-arguments",
                "/nominal/replacement_keep_result",
                "/nominal/results/i64-retention",
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
                "/nominal/results/heterogeneous",
            ],
        ),
    ] {
        let child_path = root.join(role.name()).join("receipt.json");
        let child_original = fs::read(&child_path).expect("child bytes");
        let child_value: serde_json::Value =
            serde_json::from_slice(&child_original).expect("child JSON");
        for pointer in pointers {
            let mut fault = child_value.clone();
            let value = fault
                .pointer_mut(pointer)
                .unwrap_or_else(|| panic!("missing independent pointer {pointer}"));
            *value = match value {
                serde_json::Value::Bool(value) => serde_json::json!(!*value),
                serde_json::Value::Number(value) => {
                    serde_json::json!(if value.as_u64() == Some(0) { 999 } else { 0 })
                }
                serde_json::Value::String(_) => serde_json::json!("foreign"),
                serde_json::Value::Array(_) => serde_json::json!([]),
                serde_json::Value::Object(_) => serde_json::json!({}),
                _ => panic!("unsupported fault {pointer}"),
            };
            let bytes = encode(role, fault).expect("canonical typed mutation");
            fs::write(&child_path, &bytes).expect("mutate child");
            let mut aggregate = baseline.clone();
            aggregate
                .children
                .iter_mut()
                .find(|child| child.role == role)
                .expect("role")
                .receipt = Some(receipt_identity(&child_path).expect("rebound outer identity"));
            fs::write(
                &path,
                evidence::encode_json(&aggregate).expect("canonical aggregate"),
            )
            .expect("rebound aggregate");
            let rejected = read_receipt(&options, &verifier, &manifest);
            fs::write(&child_path, &child_original).expect("restore child before assertion");
            fs::write(&path, &original).expect("restore aggregate before assertion");
            let error = rejected.expect_err(&format!("{} {pointer}", role.name()));
            results.push(serde_json::json!({"role":role.name(),"fault":pointer,"recomputed_outer_hash":true,"rejection":error.to_string()}));
        }
        if matches!(role, Oracle::PureTail | Oracle::OfflinePackages) {
            let section = if role == Oracle::PureTail {
                "outcomes"
            } else {
                "observations"
            };
            let required: Vec<String> = if role == Oracle::PureTail {
                child_value[section]
                    .as_object()
                    .expect("outcomes")
                    .keys()
                    .filter(|key| {
                        key.ends_with("-generic-pair")
                            || key.ends_with("-constant-text")
                            || key.ends_with("-constant-lists")
                            || key.as_str() == "generic-factory-definitions"
                            || key.as_str() == "capture-safe-constraint-clear"
                    })
                    .cloned()
                    .collect()
            } else {
                [
                    "generic_capture_factory",
                    "imported_capture_constraint",
                    "rejection-kernel_type_constraint",
                ]
                .map(str::to_owned)
                .to_vec()
            };
            assert!(
                required.len() >= 3,
                "independent constrained-outcome inventory"
            );
            let mut omissions = Vec::new();
            for key in required {
                let mut fault = child_value.clone();
                assert!(
                    fault[section]
                        .as_object_mut()
                        .expect("section")
                        .remove(&key)
                        .is_some()
                );
                omissions.push((
                    format!("omit-{section}/{key}"),
                    encode(role, fault).expect("typed omission"),
                ));
            }
            let mut predecessor = child_value.clone();
            predecessor["schema"] = serde_json::json!(if role == Oracle::PureTail {
                "lkjscript-pure-tail-acceptance-4"
            } else {
                "lkjscript-offline-packages-acceptance-4"
            });
            omissions.push((
                "obsolete-language-receipt".to_owned(),
                encode(role, predecessor).expect("predecessor"),
            ));
            for (name, bytes) in omissions {
                fs::write(&child_path, bytes).expect("omitted child outcome");
                let mut aggregate = baseline.clone();
                aggregate
                    .children
                    .iter_mut()
                    .find(|child| child.role == role)
                    .expect("role")
                    .receipt = Some(receipt_identity(&child_path).expect("rebound omission hash"));
                fs::write(&path, evidence::encode_json(&aggregate).expect("aggregate"))
                    .expect("write aggregate");
                let rejected = read_receipt(&options, &verifier, &manifest);
                fs::write(&child_path, &child_original).expect("restore child");
                fs::write(&path, &original).expect("restore aggregate");
                results.push(serde_json::json!({"role":role.name(),"fault":name,"recomputed_outer_hash":true,"rejection":rejected.expect_err(&name).to_string()}));
            }
        }
    }
    // Corrupt bytes in place only in the explicitly owned frozen rehearsal, restoring before assertion.
    for file in [&options.candidate, &verifier] {
        let mut handle = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(file)
            .expect("owned frozen test file");
        let mut byte = [0];
        handle.read_exact(&mut byte).expect("read byte");
        handle.seek(SeekFrom::Start(0)).expect("seek");
        handle.write_all(&[byte[0] ^ 1]).expect("fault byte");
        let rejected = read_receipt(&options, &verifier, &manifest);
        handle.seek(SeekFrom::Start(0)).expect("seek restore");
        handle.write_all(&byte).expect("restore byte");
        handle.sync_all().expect("sync restored bytes");
        results.push(serde_json::json!({"fault":"changed-executable-bytes","file":file,"rejection":rejected.expect_err("changed executable").to_string()}));
    }
    read_receipt(&options, &verifier, &manifest).expect("healthy restored evidence");
    let output = root
        .parent()
        .expect("parent")
        .join("transferred-fault-results.json");
    evidence::publish_json(&output, &results).expect("bounded fault evidence");
    println!(
        "{}",
        serde_json::json!({"faults":results.len(),"receipt":output,"healthy_recovery":true})
    );
}

#[test]
#[ignore = "requires LKJSCRIPT_TRANSFERRED_FIXTURE_ROOT and two new sibling evidence roots"]
fn live_cancellation_then_healthy_transfer() {
    let root = PathBuf::from(
        std::env::var_os("LKJSCRIPT_TRANSFERRED_FIXTURE_ROOT").expect("rehearsal root"),
    );
    let baseline: Receipt =
        serde_json::from_slice(&fs::read(root.join("receipt.json")).expect("receipt"))
            .expect("baseline");
    let parent = root.parent().expect("parent");
    let cancelled_root = parent.join("cancelled-transfer");
    let healthy_root = parent.join("healthy-transfer-after-cancellation");
    assert!(
        !cancelled_root.exists() && !healthy_root.exists(),
        "retry roots must be new"
    );
    let command = |evidence_root: &Path| {
        vec![
            baseline.verifier.path.clone(),
            "release".to_owned(),
            "transferred".to_owned(),
            "run".to_owned(),
            "--candidate".to_owned(),
            baseline.candidate.path.clone(),
            "--manifest".to_owned(),
            baseline.manifest.path.clone(),
            "--tag".to_owned(),
            baseline.tag.clone(),
            "--commit".to_owned(),
            baseline.source_commit.clone(),
            "--publication".to_owned(),
            match baseline.publication {
                PublicationMode::DryRun => "dry-run",
                PublicationMode::Release => "release",
            }
            .to_owned(),
            "--boundary".to_owned(),
            "pre-publication".to_owned(),
            "--verifier-identity".to_owned(),
            Path::new(&baseline.verifier.path)
                .parent()
                .expect("handoff")
                .join("verifier-identity.json")
                .display()
                .to_string(),
            "--expected-verifier-sha256".to_owned(),
            baseline.verifier.sha256.as_str().to_owned(),
            "--expected-verifier-bytes".to_owned(),
            baseline.verifier.byte_length.to_string(),
            "--evidence-root".to_owned(),
            evidence_root.display().to_string(),
        ]
    };
    let control = process::ProcessControl::default();
    let trigger = control.clone();
    let watch = cancelled_root.clone();
    let interrupt = std::thread::spawn(move || {
        let start = Instant::now();
        while !watch.join("distributed-http.stdout.log").exists()
            && start.elapsed() < Duration::from_secs(30)
        {
            std::thread::sleep(Duration::from_millis(20));
        }
        std::thread::sleep(Duration::from_millis(400));
        trigger.interrupt();
    });
    let mut spec = process::ProcessSpec {
        command: command(&cancelled_root),
        cwd: parent.to_path_buf(),
        environment: BTreeMap::from([
            ("LANG".to_owned(), "C".to_owned()),
            ("PATH".to_owned(), "/usr/bin:/bin".to_owned()),
        ]),
        timeout: Duration::from_secs(60),
        maximum_stdout_bytes: MAXIMUM_OUTPUT_BYTES,
        maximum_stderr_bytes: MAXIMUM_OUTPUT_BYTES,
        stdout_path: parent.join("cancelled-transfer.stdout"),
        stderr_path: parent.join("cancelled-transfer.stderr"),
        unavailable_exit_code: Some(2),
    };
    let cancelled = process::run_supervised(&spec, parent, Some(&control));
    interrupt.join().expect("joined signal sender");
    assert_eq!(
        cancelled.status,
        process::ProcessStatus::Failed,
        "{cancelled:?}"
    );
    let failed: Receipt = serde_json::from_slice(
        &fs::read(cancelled_root.join("receipt.json")).expect("retained failed aggregate"),
    )
    .expect("failed receipt");
    assert_eq!(failed.status, Status::Failed);
    assert!(
        failed
            .children
            .iter()
            .skip(1)
            .all(|child| child.status == Status::NotRun)
    );
    assert!(
        fs::read_dir(&cancelled_root)
            .expect("cancelled evidence")
            .all(|entry| !entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .starts_with(".child-state-"))
    );
    spec.command = command(&healthy_root);
    spec.timeout = Duration::from_secs(5700);
    spec.stdout_path = parent.join("healthy-transfer.stdout");
    spec.stderr_path = parent.join("healthy-transfer.stderr");
    let healthy = process::run_supervised(&spec, parent, None);
    assert_eq!(
        healthy.status,
        process::ProcessStatus::Passed,
        "{healthy:?}"
    );
    let passed: Receipt = serde_json::from_slice(
        &fs::read(healthy_root.join("receipt.json")).expect("healthy aggregate"),
    )
    .expect("healthy receipt");
    assert_eq!(passed.status, Status::FreshPassed);
    assert!(passed.cleanup_complete);
    evidence::publish_json(&parent.join("transferred-cancellation-results.json"), &serde_json::json!({
        "cancelled_root":cancelled_root,"cancelled_process":cancelled,"cancelled_status":failed.status,
        "healthy_root":healthy_root,"healthy_process":healthy,"healthy_status":passed.status,"cleanup_complete":true
    })).expect("retained cancellation/recovery evidence");
}
