//! Explicitly selected iteration through the copied public executable; no acceptance receipt.
use super::*;

fn context() -> Context {
    let evidence = PathBuf::from(
        std::env::var_os("LKJSCRIPT_EFFECTS_EVIDENCE").expect("absent evidence directory"),
    );
    fs::create_dir(&evidence).unwrap();
    let root = tempfile::Builder::new()
        .prefix("lkjscript-effects-")
        .tempdir()
        .unwrap()
        .keep();
    let binary = root.join("lkjscript");
    fs::copy(
        std::env::var_os("LKJSCRIPT_EFFECTS_CANDIDATE").expect("frozen candidate path"),
        &binary,
    )
    .unwrap();
    let digest = digest_file(&binary, MAXIMUM_EXECUTABLE_BYTES).unwrap();
    Context {
        root: root.clone(),
        evidence: evidence.clone(),
        binary,
        receipt: Receipt {
            schema: "iteration-only".into(),
            status: "not an acceptance receipt".into(),
            candidate_sha256: digest.clone(),
            verifier_sha256: digest_file(
                &std::env::var_os("LKJSCRIPT_EFFECTS_VERIFIER")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| std::env::current_exe().unwrap()),
                // Test binaries retain debug sections; this observer admission is separate from
                // the product/release executable limit, which remains unchanged.
                1_073_741_824,
            )
            .unwrap(),
            copied_candidate_sha256: digest,
            pinned_runtime_path: root.join("lkjscript").display().to_string(),
            isolated_root: root.display().to_string(),
            evidence_root: evidence.display().to_string(),
            environment_names: vec!["LANG".into()],
            elapsed_nanoseconds: 0,
            commands: vec![],
            runners: vec![],
            inventories: vec![],
            producer_inventories: vec![],
            transport_digests: vec![],
            observations: BTreeMap::new(),
            nominal: Default::default(),
            recursive: Default::default(),
            effects: Default::default(),
            files: vec![],
            cleanup_complete: false,
            failure: None,
        },
    }
}

#[test]
#[ignore = "requires an explicitly frozen candidate and a fresh evidence directory"]
fn copied_named_reference_iteration() {
    let mut context = context();
    let builtin = context
        .cli(None, &["package", "builtin", "inspect"], true)
        .unwrap();
    let standard = Package {
        path: PathBuf::new(),
        id: field(&builtin, "package", "id").unwrap(),
        revision: field(&builtin, "package", "revision").unwrap(),
        logical: field(&builtin, "package", "package-revision").unwrap(),
        transport: field(&builtin, "package", "transport").unwrap(),
        container: context.root.join("standard.lkjp"),
        symbols: BTreeMap::new(),
    };
    context
        .cli(
            None,
            &[
                "package",
                "builtin",
                "export",
                "--kind",
                "transport",
                "--output",
                &standard.container.display().to_string(),
            ],
            true,
        )
        .unwrap();
    let result = super::named::workflow(&mut context, &standard);
    fs::write(
        context.evidence.join("iteration.json"),
        serde_json::to_vec_pretty(&context.receipt).unwrap(),
    )
    .unwrap();
    fs::remove_dir_all(&context.root).unwrap();
    result.unwrap();
}

fn standard(context: &mut Context) -> Package {
    let builtin = context
        .cli(None, &["package", "builtin", "inspect"], true)
        .unwrap();
    let mut standard = Package {
        path: PathBuf::new(),
        id: field(&builtin, "package", "id").unwrap(),
        revision: field(&builtin, "package", "revision").unwrap(),
        logical: field(&builtin, "package", "package-revision").unwrap(),
        transport: field(&builtin, "package", "transport").unwrap(),
        container: context.root.join("standard.lkjp"),
        symbols: BTreeMap::new(),
    };
    context
        .cli(None, &["capabilities", "change"], true)
        .unwrap();
    for name in [
        "add",
        "subtract",
        "divide",
        "i64-equal",
        "less",
        "list-get",
        "list-length",
        "list-append",
    ] {
        let owners = context
            .cli(
                None,
                &["package", "builtin", "query", "owners", "--name", name],
                true,
            )
            .unwrap();
        standard
            .symbols
            .insert(name.into(), field(&owners, "owner", "reference").unwrap());
    }
    standard
}

#[test]
#[ignore = "requires an explicitly selected maintained project, frozen candidate and absent evidence path"]
fn author_task_standard_through_public_change() {
    let mut context = context();
    let mut standard = standard(&mut context);
    standard.path = PathBuf::from(
        std::env::var_os("LKJSCRIPT_EFFECTS_STANDARD_PROJECT").expect("exact standard project"),
    );
    let status = context
        .cli(Some(&standard.path), &["status"], true)
        .unwrap();
    standard.revision = field(&status, "revision", "id").unwrap();
    let module = context
        .cli(
            Some(&standard.path),
            &["query", "find", "module", "core"],
            true,
        )
        .unwrap();
    let module = field(&module, "owner", "id").unwrap();
    let request = super::effects_program::standard(&standard.symbols)
        .replace("module=$module", &format!("module={module}"));
    context.apply(&mut standard, &request).unwrap();
    context.cli(Some(&standard.path), &["check"], true).unwrap();
    context.export(&mut standard).unwrap();
    fs::write(
        context.evidence.join("iteration.json"),
        serde_json::to_vec_pretty(&context.receipt).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "requires a frozen candidate and absent evidence path; retains its isolated projects"]
fn author_transported_task_library_through_public_change() {
    let mut context = context();
    let standard = standard(&mut context);
    context
        .cli(
            None,
            &[
                "package",
                "builtin",
                "export",
                "--kind",
                "transport",
                "--output",
                &standard.container.display().to_string(),
            ],
            true,
        )
        .unwrap();
    let mut library = context.new_package("library").unwrap();
    context.stage(&library, &standard).unwrap();
    context
        .apply(
            &mut library,
            &format!(
                "{}{}{}",
                binding("add", &standard),
                module(),
                super::effects_program::library(&standard.symbols)
            ),
        )
        .unwrap();
    context.cli(Some(&library.path), &["check"], true).unwrap();
    context.export(&mut library).unwrap();
    fs::write(
        context.evidence.join("iteration.json"),
        serde_json::to_vec_pretty(&context.receipt).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "requires frozen candidate and absent evidence path; retains disposable projects for diagnosis"]
fn copied_task_library_foreground_iteration() {
    let mut context = context();
    let mut standard = standard(&mut context);
    let result = (|| -> Result<(), DevError> {
        let (mut library, _) = super::effects::prepare_library(&mut context, &mut standard)?;
        let mut consumers = super::foreground::prepare(&mut context, &standard, &library)?;
        let recovery = context.root.join("foreground-library-recovery");
        fs::rename(&library.path, &recovery)?;
        super::foreground::initial(&mut context, &consumers)?;
        fs::rename(&recovery, &library.path)?;
        let request = super::effects::duplicate_map_output(&standard.symbols, &library);
        context.apply(&mut library, &request)?;
        context.cli(Some(&library.path), &["check"], true)?;
        context.export(&mut library)?;
        super::foreground::replace(&mut context, &mut consumers, &library)?;
        fs::rename(&library.path, &recovery)?;
        super::foreground::recovery(&mut context, &mut consumers)?;
        super::foreground::validate(&context.receipt, &context.evidence)
    })();
    context.receipt.failure = result.as_ref().err().map(ToString::to_string);
    fs::write(
        context.evidence.join("iteration.json"),
        serde_json::to_vec_pretty(&context.receipt).unwrap(),
    )
    .unwrap();
    result.unwrap();
}

#[test]
#[ignore = "requires frozen candidate and absent evidence path; retains disposable projects for diagnosis"]
fn copied_task_library_http_iteration() {
    let mut context = context();
    let mut standard = standard(&mut context);
    let result = super::effects::workflow(&mut context, &mut standard)
        .and_then(|()| super::effects::validate(&context.receipt, &context.evidence));
    context.receipt.failure = result.as_ref().err().map(ToString::to_string);
    fs::write(
        context.evidence.join("iteration.json"),
        serde_json::to_vec_pretty(&context.receipt).unwrap(),
    )
    .unwrap();
    result.unwrap();
}

#[test]
#[ignore = "requires authentic LKJSCRIPT_ITERATION_READER_FIXTURE from the public effect workflow"]
fn iteration_reader_rejects_consistently_rehashed_false_observations() {
    let root = PathBuf::from(
        std::env::var_os("LKJSCRIPT_ITERATION_READER_FIXTURE").expect("authentic public fixture"),
    );
    let receipt: Receipt =
        serde_json::from_slice(&fs::read(root.join("iteration.json")).unwrap()).unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let public = scratch.path().join("task-iteration.json");
    let validate = |receipt: &Receipt| {
        fs::write(
            &public,
            evidence::encode_json(&receipt.effects.iteration).unwrap(),
        )
        .unwrap();
        super::effects_iteration_evidence::validate(receipt, scratch.path())
    };
    validate(&receipt).unwrap();
    let mut results = Vec::new();
    for (pointer, value) in [
        (
            "/iteration/rounds/0/observations/0/result/total",
            serde_json::json!(1),
        ),
        (
            "/iteration/rounds/3/observations/0/result/total",
            serde_json::json!(0),
        ),
        (
            "/iteration/rounds/8/observations/0/status",
            serde_json::json!(200),
        ),
        (
            "/iteration/rounds/9/observations/3/result",
            serde_json::json!(1),
        ),
        ("/resources/iteration/authority", serde_json::json!([])),
        (
            "/resources/iteration/authority/1/events",
            serde_json::json!(["stride", "threshold", "tick"]),
        ),
        (
            "/resources/iteration/rows/0/output/callbacks",
            serde_json::json!(0),
        ),
        (
            "/resources/iteration/rows/4/events/0",
            serde_json::json!("threshold"),
        ),
        (
            "/resources/iteration/rows/4/work/maximum_live_locals",
            serde_json::json!(8193),
        ),
        (
            "/resources/iteration/rows/4/work/value_work/capture_admission_nodes",
            serde_json::json!(8193),
        ),
        (
            "/resources/iteration/rows/24/work/maximum_live_effect_bindings",
            serde_json::json!(8193),
        ),
        (
            "/resources/iteration/rows/11/output",
            serde_json::json!({"position":3,"total":3}),
        ),
    ] {
        let mut altered = serde_json::to_value(&receipt.effects).unwrap();
        let selected = altered
            .pointer_mut(pointer)
            .unwrap_or_else(|| panic!("required independently selected field {pointer}"));
        if pointer.ends_with("/output") {
            selected
                .as_object_mut()
                .unwrap()
                .insert("value".into(), value);
        } else {
            *selected = value;
        }
        let mut changed = receipt.clone();
        changed.effects = serde_json::from_value(altered).unwrap();
        // Both copies and every enclosing hash are self-consistent. Rejection must come from the
        // typed expected behavior, not stale bytes or a parent/child disagreement.
        fs::write(
            scratch.path().join("effect-resources.stdout"),
            serde_json::to_vec(&changed.effects.resources).unwrap(),
        )
        .unwrap();
        let result = validate(&changed);
        let encoded = evidence::encode_json(&changed).unwrap();
        fs::write(scratch.path().join("parent.json"), &encoded).unwrap();
        results.push(serde_json::json!({"pointer":pointer,"child_sha256":digest_file(&public,MAXIMUM_OUTPUT_BYTES).unwrap(),"resources_sha256":digest_file(&scratch.path().join("effect-resources.stdout"),MAXIMUM_OUTPUT_BYTES).unwrap(),"parent_sha256":digest_file(&scratch.path().join("parent.json"),64*1024*1024).unwrap(),"rejection":result.unwrap_err().to_string()}));
    }
    validate(&receipt).unwrap();
    fs::write(root.join("iteration-reader-faults.json"),evidence::encode_json(&serde_json::json!({"faults":results,"baseline_and_recovery_passed":true,"authentic_source_unchanged":true})).unwrap()).unwrap();
}
