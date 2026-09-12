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
                MAXIMUM_EXECUTABLE_BYTES,
            )
            .unwrap(),
            copied_candidate_sha256: digest,
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

fn measure_plain(
    context: &Context,
    standalone: &Path,
    candidate: PathBuf,
) -> Vec<serde_json::Value> {
    let mut rounds = Vec::new();
    for (name, binary) in [
        ("predecessor", context.binary.clone()),
        ("candidate", candidate),
    ] {
        let mut round = crate::stateful_http::recursive_round(&binary,standalone,&context.evidence,&format!("plain-{name}"),|address| {
            let mut cells = Vec::new();
            for count in [32_i64,4097,8192] {
                for sample in 0..4 {
                    let response = crate::http_probe::request(address,"GET","/",count.to_string().as_bytes(),&[])?;
                    let expected_failure = name=="predecessor" && count>4096;
                    let failure = response.headers.get("x-lkjscript-failure-code");
                    require(response.status==if expected_failure {500}else{200},"plain task comparison status differs")?;
                    let result = if expected_failure {
                        require(failure.map(String::as_str)==Some("normalized_call_depth"),"baseline failed for another policy")?;
                        serde_json::Value::Null
                    } else {
                        let result:serde_json::Value=serde_json::from_slice(&response.body)?;
                        require(result==count*(count+1)/2 && failure.is_none(),"plain task independently expected sum differs")?;
                        result
                    };
                    cells.push(serde_json::json!({"count":count,"warmup":sample==0,"sample":sample,"status":response.status,"failure":failure,"result":result,"elapsed_nanoseconds":response.elapsed_nanoseconds}));
                }
            }
            Ok(serde_json::json!(cells))
        }).unwrap();
        round["binary_sha256"] =
            serde_json::json!(digest_file(&binary, MAXIMUM_EXECUTABLE_BYTES).unwrap());
        round["version"] = serde_json::json!(name);
        rounds.push(round);
    }
    rounds
}

#[test]
#[ignore = "requires frozen predecessor-authored evidence and release-profile executables; run without concurrent owned builds"]
fn measure_frozen_plain_task_loop() {
    let context = context();
    let evidence = PathBuf::from(
        std::env::var_os("LKJSCRIPT_ITERATION_FROZEN_PLAIN_EVIDENCE")
            .expect("retained exact predecessor-authored comparison"),
    );
    let source: serde_json::Value =
        serde_json::from_slice(&fs::read(evidence.join("comparison.json")).unwrap()).unwrap();
    let authoring: Receipt =
        serde_json::from_slice(&fs::read(evidence.join("authoring.json")).unwrap()).unwrap();
    let standalone = context.root.join("plain-task-standalone");
    fs::create_dir(&standalone).unwrap();
    let artifact = standalone.join("application.lkja");
    fs::copy(evidence.join("plain-task.lkja"), &artifact).unwrap();
    assert_eq!(
        source["same_artifact"],
        digest_file(&artifact, MAXIMUM_CONTAINER_BYTES).unwrap()
    );
    assert_eq!(
        source["rounds"][0]["binary_sha256"],
        digest_file(&context.binary, MAXIMUM_EXECUTABLE_BYTES).unwrap()
    );
    assert!(
        !Path::new(&authoring.isolated_root)
            .join("plain-task-producer")
            .exists()
    );
    let descriptor = standalone.join("service.deployment.json");
    fs::copy(
        Path::new(&authoring.isolated_root).join("plain-task-standalone/service.deployment.json"),
        &descriptor,
    )
    .unwrap();
    let candidate = PathBuf::from(
        std::env::var_os("LKJSCRIPT_ITERATION_COMPARISON_CANDIDATE")
            .expect("frozen release candidate"),
    );
    let rounds = measure_plain(&context, &standalone, candidate);
    fs::write(context.evidence.join("comparison.json"),evidence::encode_json(&serde_json::json!({"source_workload":source,"descriptor_sha256":digest_file(&descriptor,MAXIMUM_OUTPUT_BYTES).unwrap(),"source_commit":std::env::var("LKJSCRIPT_ITERATION_MEASURE_SOURCE").expect("exact candidate source"),"rounds":rounds,"profile":"release","concurrent_owned_builds":false,"default_policy":true,"producer_unavailable":true})).unwrap()).unwrap();
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
        let (mut library, names) = super::effects::prepare_library(&mut context, &mut standard)?;
        let mut consumers = super::foreground::prepare(&mut context, &standard, &library, &names)?;
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

#[test]
#[ignore = "explicit frozen predecessor/candidate comparison; author and build the identical workload once with the predecessor"]
fn compare_predecessor_authored_plain_task_loop() {
    use crate::pure_tail_program::Request;
    let mut context = context();
    let mut standard = standard(&mut context);
    super::effects::discover(&mut context, &mut standard).unwrap();
    let s = &standard.symbols;
    let path = context.root.join("plain-task-producer");
    let created = context
        .cli(
            None,
            &[
                "new",
                &path.display().to_string(),
                "--template",
                "http",
                "--name",
                "plain-task-producer",
            ],
            true,
        )
        .unwrap();
    let mut producer = Package {
        path,
        id: field(&created, "package", "id").unwrap(),
        revision: field(&created, "revision", "id").unwrap(),
        logical: String::new(),
        transport: String::new(),
        container: PathBuf::new(),
        symbols: BTreeMap::new(),
    };
    let module = context
        .cli(
            Some(&producer.path),
            &["query", "find", "module", "application"],
            true,
        )
        .unwrap();
    let module = field(&module, "owner", "id").unwrap();
    let handle = context
        .cli(
            Some(&producer.path),
            &[
                "query",
                "find",
                "declaration",
                "handle",
                "--parent",
                &module,
            ],
            true,
        )
        .unwrap();
    let handle = field(&handle, "owner", "id").unwrap();
    let definition = context
        .cli(
            Some(&producer.path),
            &[
                "inspect",
                "owner",
                "task_function",
                &handle,
                "--detail",
                "definition",
                "--limit",
                "1000",
                "--bytes",
                "1048576",
            ],
            true,
        )
        .unwrap();
    let parameter = field(&definition, "definition.parameter", "id").unwrap();
    let stream = definition
        .iter()
        .find(|r| {
            r.operation == "definition.reference"
                && record_field(r, "role").is_ok_and(|role| role == "function_requirement")
        })
        .unwrap();
    let stream = record_field(stream, "target").unwrap();
    let mut r = Request::default();
    let n = r.local("$loop-n");
    let zero = r.integer(0);
    let done = r.call(&s["i64-equal"], &[], &[n, zero]);
    let result = r.local("$loop-sum");
    let n = r.local("$loop-n");
    let one = r.integer(1);
    let next = r.call(&s["subtract"], &[], &[n, one]);
    let total = r.local("$loop-sum");
    let n = r.local("$loop-n");
    let total = r.call(&s["add"], &[], &[total, n]);
    let again = r.call("$loop", &[], &[next, total]);
    let body = r.choose(&done, &result, &again);
    r.text.push_str(&format!("create.function as=$loop module={module} name=plain-task-loop visibility=private result=i64 effect=task body={body}\nadd.parameter as=$loop-n function=$loop name=n type=i64\nadd.parameter as=$loop-sum function=$loop name=sum type=i64\n"));
    r.text.push_str("type.structural-record as=@Header\ntype.field parent=@Header index=0 name=name type=text\ntype.field parent=@Header index=1 name=value type=bytes\ntype.list as=@Headers item=@Header\ntype.structural-record as=@Response\ntype.field parent=@Response index=0 name=body type=bytes\ntype.field parent=@Response index=1 name=headers type=@Headers\ntype.field parent=@Response index=2 name=status type=i64\n");
    let input = r.local(&parameter);
    let body = r.field(&input, "body");
    let limit = r.integer(1024);
    let bytes = r.capability(&stream, &s["ByteStream.read-all"], &[body, limit]);
    let fallback = r.integer(0);
    let decoded = r.call(&s["json-decode-or"], &["i64"], &[bytes, fallback]);
    let n = r.field(&decoded, "value");
    let zero = r.integer(0);
    let result = r.call("$loop", &[], &[n, zero]);
    let response = super::effects_consumer::response(&mut r, s, &result, "i64");
    r.text
        .push_str(&format!("replace.body function={handle} body={response}\n"));
    // Capture the predecessor's public review, whose capabilities identity deliberately differs
    // from today's diagnostic catalog. Its own executable rechecks the full plan on apply; this
    // comparison does not assert that today's strict plan-file reader admits an old contract.
    let request = context.evidence.join("plain-request.lkjc");
    let review = context.evidence.join("plain-review.lkjplan");
    fs::write(
        &request,
        format!("request base={}\n{}", producer.revision, r.text),
    )
    .unwrap();
    let plan = context
        .cli(
            Some(&producer.path),
            &[
                "change",
                "plan",
                "--input-file",
                &request.display().to_string(),
                "--output",
                &review.display().to_string(),
            ],
            true,
        )
        .unwrap();
    let token = field(&plan, "plan", "token").unwrap();
    let records =
        parse_records(&review.display().to_string(), &fs::read(&review).unwrap()).unwrap();
    assert_eq!(
        field(&records, "logical-plan.digest", "token").unwrap(),
        token
    );
    assert_eq!(
        field(&records, "logical-plan.authority", "base").unwrap(),
        producer.revision
    );
    let applied = context
        .cli(
            Some(&producer.path),
            &[
                "change",
                "apply",
                "--input-file",
                &request.display().to_string(),
                "--plan",
                &token,
            ],
            true,
        )
        .unwrap();
    producer.revision = field(&applied, "revision", "result").unwrap();
    context.cli(Some(&producer.path), &["check"], true).unwrap();
    context.export(&mut producer).unwrap();
    let standalone = context.root.join("plain-task-standalone");
    fs::create_dir(&standalone).unwrap();
    let artifact = standalone.join("application.lkja");
    context
        .cli(
            Some(&producer.path),
            &["build", "--output", &artifact.display().to_string()],
            true,
        )
        .unwrap();
    let mut deployment: serde_json::Value =
        serde_json::from_slice(&fs::read(producer.path.join("service.deployment.json")).unwrap())
            .unwrap();
    deployment["artifact"] = serde_json::json!("application.lkja");
    deployment["listen"] = serde_json::json!("127.0.0.1:0");
    fs::write(
        standalone.join("service.deployment.json"),
        evidence::encode_json(&deployment).unwrap(),
    )
    .unwrap();
    fs::copy(&artifact, context.evidence.join("plain-task.lkja")).unwrap();
    fs::rename(&producer.path, context.root.join("plain-task-recovery")).unwrap();
    let candidate = PathBuf::from(
        std::env::var_os("LKJSCRIPT_ITERATION_COMPARISON_CANDIDATE")
            .expect("frozen comparison candidate"),
    );
    let rounds = measure_plain(&context, &standalone, candidate);
    fs::write(context.evidence.join("comparison.json"),evidence::encode_json(&serde_json::json!({"author":"copied frozen predecessor","same_artifact":digest_file(&artifact,MAXIMUM_CONTAINER_BYTES).unwrap(),"package":producer.id,"revision":producer.revision,"package_revision":producer.logical,"transport":producer.transport,"rounds":rounds,"default_policy":true,"producer_unavailable":true})).unwrap()).unwrap();
    fs::write(
        context.evidence.join("authoring.json"),
        evidence::encode_json(&context.receipt).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "explicit maintained graph cutover through copied-executable plan/apply"]
fn cutover_task_iteration_standard_through_public_change() {
    let mut context = context();
    let mut standard = standard(&mut context);
    standard.path = PathBuf::from(std::env::var_os("LKJSCRIPT_EFFECTS_STANDARD_PROJECT").unwrap());
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
    let mut old = BTreeMap::new();
    for (key, name) in [
        ("function", "task-fold-left"),
        ("range", "task-fold-left-range"),
    ] {
        let found = context
            .cli(
                Some(&standard.path),
                &["query", "find", "declaration", name, "--parent", &module],
                true,
            )
            .unwrap();
        old.insert(key.to_owned(), field(&found, "owner", "id").unwrap());
    }
    for (kind, names) in [
        ("type_parameter", vec!["Item", "State"]),
        ("effect_parameter", vec!["E"]),
        ("parameter", vec!["items", "state", "step"]),
    ] {
        for name in names {
            let found = context
                .cli(
                    Some(&standard.path),
                    &["query", "find", kind, name, "--parent", &old["function"]],
                    true,
                )
                .unwrap();
            let id = field(&found, "owner", "id").unwrap();
            old.insert(
                name.to_owned(),
                if kind == "effect_parameter" {
                    format!("{}/{id}", standard.id)
                } else {
                    id
                },
            );
        }
    }
    let mut r = crate::pure_tail_program::Request::default();
    super::effects_traversal::fold(&mut r, &standard.symbols, Some(&old));
    super::effects_traversal::iteration(&mut r);
    let request = r
        .text
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
#[ignore = "explicit exact maintained dependency cutover through copied-executable plan/apply"]
fn cutover_task_iteration_application_through_public_change() {
    let mut context = context();
    let mut standard = standard(&mut context);
    standard.path = PathBuf::from(std::env::var_os("LKJSCRIPT_EFFECTS_STANDARD_PROJECT").unwrap());
    let status = context
        .cli(Some(&standard.path), &["status"], true)
        .unwrap();
    standard.revision = field(&status, "revision", "id").unwrap();
    context.export(&mut standard).unwrap();
    let path = PathBuf::from(std::env::var_os("LKJSCRIPT_EFFECTS_APPLICATION_PROJECT").unwrap());
    let status = context.cli(Some(&path), &["status"], true).unwrap();
    let mut application = Package {
        path,
        id: field(&status, "package", "id").unwrap(),
        revision: field(&status, "revision", "id").unwrap(),
        logical: String::new(),
        transport: String::new(),
        container: PathBuf::new(),
        symbols: BTreeMap::new(),
    };
    context.stage(&application, &standard).unwrap();
    context
        .apply(&mut application, &binding("replace", &standard))
        .unwrap();
    context
        .cli(Some(&application.path), &["check"], true)
        .unwrap();
    let output = context.evidence.join("lkjournal.lkja");
    context
        .cli(
            Some(&application.path),
            &["build", "--output", &output.display().to_string()],
            true,
        )
        .unwrap();
    context.export(&mut application).unwrap();
    fs::write(
        context.evidence.join("iteration.json"),
        serde_json::to_vec_pretty(&context.receipt).unwrap(),
    )
    .unwrap();
}
