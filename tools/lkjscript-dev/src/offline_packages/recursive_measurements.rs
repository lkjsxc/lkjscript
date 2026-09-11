//! An explicitly selected predecessor/candidate comparison, using the existing copied CLI driver.
//! No accepted graph is constructed by the verifier. This is cost evidence, not a release gate.
use super::*;
use crate::pure_tail_program::Request;
use serde_json::json;

fn predecessor_apply(context: &mut Context, package: &mut Package, changes: &str) {
    // The current plan reader correctly rejects another product version. For the baseline,
    // its copied executable owns both plan production and apply-token validation.
    let path = context.evidence.join("predecessor-request.lkjc");
    fs::write(
        &path,
        format!("request base={}\n{changes}", package.revision),
    )
    .unwrap();
    let review = context.evidence.join("predecessor-review.lkjplan");
    let plan = context
        .cli(
            Some(&package.path),
            &[
                "change",
                "plan",
                "--input-file",
                &path.display().to_string(),
                "--output",
                &review.display().to_string(),
            ],
            true,
        )
        .unwrap();
    let token = field(&plan, "plan", "token").unwrap();
    let rejection =
        decode_logical_change_plan(std::io::BufReader::new(fs::File::open(&review).unwrap()))
            .unwrap_err();
    assert_eq!(rejection.code, "change_plan_file_product");
    let applied = context
        .cli(
            Some(&package.path),
            &[
                "change",
                "apply",
                "--input-file",
                &path.display().to_string(),
                "--plan",
                &token,
            ],
            true,
        )
        .unwrap();
    package.revision = field(&applied, "revision", "result").unwrap();
    for record in applied.iter().filter(|r| r.operation == "identity") {
        package.symbols.insert(
            record_field(record, "symbol").unwrap(),
            record_field(record, "id").unwrap(),
        );
    }
}

fn program(standard: &BTreeMap<String, String>) -> String {
    let mut r = Request::default();
    r.text.push_str("create.variant as=$tree module=$module name=tree visibility=public\ntype.named as=@Tree declaration=$tree\ntype.list as=@Trees item=@Tree\nadd.case as=$leaf variant=$tree name=leaf payload=i64\nadd.case as=$branch variant=$tree name=branch payload=@Trees\ncreate.component as=$component module=$module name=commands visibility=private\n");
    for mapped in [false, true] {
        let name = if mapped { "mapped" } else { "sum" };
        let helper = format!("{name}-children");
        let value = r.local(&format!("${name}-leaf"));
        let leaf = if mapped {
            let three = r.integer(3);
            let five = r.integer(5);
            let product = r.call(&standard["multiply"], &[], &[three, value]);
            let value = r.call(&standard["add"], &[], &[product, five]);
            r.expression("variant", &format!("case=$leaf payload={value}"))
        } else {
            value
        };
        let children = r.local(&format!("${name}-branch"));
        let zero = r.integer(0);
        let initial = if mapped {
            r.expression("list", "item=@Tree")
        } else {
            r.integer(0)
        };
        let branch = r.call(&format!("${helper}"), &[], &[children, zero, initial]);
        let branch = if mapped {
            r.expression("variant", &format!("case=$branch payload={branch}"))
        } else {
            branch
        };
        let value = r.local(&format!("${name}_tree"));
        let body = r.expression("match", &format!("value={value}"));
        r.text.push_str(&format!("expression.match-arm parent={body} index=0 case=$leaf as=${name}-leaf name=leaf type=i64 body={leaf}\nexpression.match-arm parent={body} index=1 case=$branch as=${name}-branch name=children type=@Trees body={branch}\n"));
        r.function(
            name,
            if mapped { "@Tree" } else { "i64" },
            &body,
            &[("tree", "@Tree")],
        );
        let items = r.local(&format!("${helper}_items"));
        let length = r.call(&standard["list-length"], &["@Tree"], &[items]);
        let index = r.local(&format!("${helper}_index"));
        let done = r.call(&standard["i64-equal"], &[], &[index, length]);
        let result = r.local(&format!("${helper}_result"));
        let items = r.local(&format!("${helper}_items"));
        let index = r.local(&format!("${helper}_index"));
        let child = r.call(&standard["list-get"], &["@Tree"], &[items, index]);
        let child = r.call(&format!("${name}"), &[], &[child]);
        let accumulated = r.local(&format!("${helper}_result"));
        let advanced = if mapped {
            r.call(&standard["list-append"], &["@Tree"], &[accumulated, child])
        } else {
            r.call(&standard["add"], &[], &[accumulated, child])
        };
        let items = r.local(&format!("${helper}_items"));
        let index = r.local(&format!("${helper}_index"));
        let one = r.integer(1);
        let next = r.call(&standard["add"], &[], &[index, one]);
        let recur = r.call(&format!("${helper}"), &[], &[items, next, advanced]);
        let body = r.expression(
            "if",
            &format!("condition={done} when-true={result} when-false={recur}"),
        );
        let result_type = if mapped { "@Trees" } else { "i64" };
        r.function(
            &helper,
            result_type,
            &body,
            &[
                ("items", "@Trees"),
                ("index", "i64"),
                ("result", result_type),
            ],
        );
    }
    let start = r.local("$balanced_start");
    let leaf = r.expression("variant", &format!("case=$leaf payload={start}"));
    let count = r.local("$balanced_count");
    let two = r.integer(2);
    let half = r.call(&standard["divide"], &[], &[count, two]);
    let start = r.local("$balanced_start");
    let half_value = r.local("$half");
    let left = r.call("$balanced", &[], &[start, half_value]);
    let start = r.local("$balanced_start");
    let half_value = r.local("$half");
    let next = r.call(&standard["add"], &[], &[start, half_value]);
    let count = r.local("$balanced_count");
    let half_value = r.local("$half");
    let remaining = r.call(&standard["subtract"], &[], &[count, half_value]);
    let right = r.call("$balanced", &[], &[next, remaining]);
    let children = r.expression("list", "item=@Tree");
    r.arguments(&children, &[left, right]);
    let branch = r.expression("variant", &format!("case=$branch payload={children}"));
    let split = r.expression("let", &format!("body={branch}"));
    r.text.push_str(&format!(
        "expression.binding parent={split} index=0 as=$half name=half value={half} type=i64\n"
    ));
    let count = r.local("$balanced_count");
    let one = r.integer(1);
    let single = r.call(&standard["i64-equal"], &[], &[count, one]);
    let body = r.expression(
        "if",
        &format!("condition={single} when-true={leaf} when-false={split}"),
    );
    r.function(
        "balanced",
        "@Tree",
        &body,
        &[("start", "i64"), ("count", "i64")],
    );
    for target in ["scale-sum", "scale-mapped-sum"] {
        let start = r.local(&format!("${target}_start"));
        let count = r.local(&format!("${target}_count"));
        let tree = r.call("$balanced", &[], &[start, count]);
        let tree = if target.contains("mapped") {
            r.call("$mapped", &[], &[tree])
        } else {
            tree
        };
        let result = r.call("$sum", &[], &[tree]);
        r.function(
            target,
            "i64",
            &result,
            &[("start", "i64"), ("count", "i64")],
        );
        r.target(target, "i64", &["i64", "i64"]);
    }
    r.text
}

#[test]
#[ignore = "requires explicit frozen predecessor/candidate and create-new evidence paths"]
fn recursive_monomorphic_predecessor_cost_comparison() {
    let evidence = PathBuf::from(
        std::env::var_os("LKJSCRIPT_RECURSIVE_COMPARISON_ROOT")
            .expect("owned absent evidence root"),
    );
    fs::create_dir(&evidence).unwrap();
    let mut predecessor: Option<(Package, Package)> = None;
    for (label, variable) in [
        ("baseline", "LKJSCRIPT_RECURSIVE_BASELINE"),
        ("candidate", "LKJSCRIPT_RECURSIVE_CANDIDATE"),
    ] {
        let original =
            PathBuf::from(std::env::var_os(variable).expect("explicit frozen candidate"));
        let isolated = tempfile::Builder::new()
            .prefix("lkjscript-recursive-monomorphic-")
            .tempdir()
            .unwrap();
        let binary = isolated.path().join("lkjscript");
        fs::copy(&original, &binary).unwrap();
        let root = evidence.join(label);
        fs::create_dir(&root).unwrap();
        let digest = digest_file(&binary, MAXIMUM_EXECUTABLE_BYTES).unwrap();
        let mut context = Context {
            root: isolated.path().into(),
            evidence: root.clone(),
            binary,
            receipt: Receipt {
                schema: "measurement-only".into(),
                status: "not an acceptance receipt".into(),
                candidate_sha256: digest.clone(),
                verifier_sha256: digest_file(
                    &std::env::current_exe().unwrap(),
                    MAXIMUM_EXECUTABLE_BYTES,
                )
                .unwrap(),
                copied_candidate_sha256: digest,
                isolated_root: isolated.path().display().to_string(),
                evidence_root: root.display().to_string(),
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
                files: vec![],
                cleanup_complete: false,
                failure: None,
            },
        };
        let builtin = context
            .cli(None, &["package", "builtin", "inspect"], true)
            .unwrap();
        let mut standard = Package {
            path: PathBuf::new(),
            id: field(&builtin, "package", "id").unwrap(),
            revision: field(&builtin, "package", "revision").unwrap(),
            logical: field(&builtin, "package", "package-revision").unwrap(),
            transport: field(&builtin, "package", "transport").unwrap(),
            container: isolated.path().join("standard.lkjp"),
            symbols: BTreeMap::new(),
        };
        context.cli(None, &["capabilities"], true).unwrap();
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
        for name in [
            "add",
            "multiply",
            "subtract",
            "divide",
            "i64-equal",
            "list-get",
            "list-length",
            "list-append",
        ] {
            let records = context
                .cli(
                    None,
                    &["package", "builtin", "query", "owners", "--name", name],
                    true,
                )
                .unwrap();
            standard
                .symbols
                .insert(name.into(), field(&records, "owner", "reference").unwrap());
        }
        let mut package = context.new_package("monomorphic").unwrap();
        context.stage(&package, &standard).unwrap();
        let changes = format!(
            "{}{}{}",
            binding("add", &standard),
            module(),
            program(&standard.symbols)
        );
        if label == "baseline" {
            predecessor_apply(&mut context, &mut package, &changes);
        } else {
            context.apply(&mut package, &changes).unwrap();
        }
        context.cli(Some(&package.path), &["check"], true).unwrap();
        context
            .cli(
                Some(&package.path),
                &[
                    "build",
                    "--output",
                    &root.join("candidate.lkja").display().to_string(),
                ],
                true,
            )
            .unwrap();
        let mut observations = Vec::new();
        for count in [32_i64, 4096] {
            for target in ["scale-sum", "scale-mapped-sum"] {
                let expected = if target == "scale-sum" {
                    count * (count + 1) / 2
                } else {
                    3 * count * (count + 1) / 2 + 5 * count
                };
                for repeat in 0..3 {
                    let result = context
                        .cli(
                            Some(&package.path),
                            &["run", target, "--arguments", &json!([1, count]).to_string()],
                            true,
                        )
                        .unwrap();
                    assert_eq!(
                        field(&result, "execution", "value").unwrap(),
                        expected.to_string()
                    );
                    observations.push(json!({"leaves":count,"target":target,"repeat":repeat,"expected":expected,"command":context.receipt.commands.len()-1}));
                }
            }
        }
        // Preserve complete predecessor source separately from derived compilation proofs.
        // A current consumer must revalidate and rebuild it, without retaining the old producer.
        fs::copy(&standard.container, root.join("standard.lkjp")).unwrap();
        standard.container = root.join("standard.lkjp");
        package.container = root.join("source.lkjp");
        let exported = context
            .cli(
                Some(&package.path),
                &[
                    "package",
                    "current",
                    "export",
                    "--kind",
                    "transport",
                    "--output",
                    &package.container.display().to_string(),
                ],
                true,
            )
            .unwrap();
        package.logical = field(&exported, "package", "package-revision").unwrap();
        package.transport = field(&exported, "package", "transport").unwrap();
        let mut recovery = json!(null);
        if let Some((old_standard, old_package)) = &predecessor {
            assert!(
                !old_package.path.exists(),
                "old producer must be removed before recovery"
            );
            let obsolete = fs::read(evidence.join("baseline/candidate.lkja")).unwrap();
            let rejected =
                lkjscript::platform::contributor::strict_artifact_admission_probe(&obsolete)
                    .unwrap_err();
            let mut recovered = context.new_package("recovered-predecessor").unwrap();
            context.stage(&recovered, old_standard).unwrap();
            context.stage(&recovered, old_package).unwrap();
            let mut request = Request::default();
            request.text.push_str(
                "create.component as=$component module=$module name=commands visibility=private\n",
            );
            let start = request.integer(1);
            let count = request.integer(4096);
            let body = request.call(
                &reference(old_package, "$scale-sum").unwrap(),
                &[],
                &[start, count],
            );
            request.function("recovered", "i64", &body, &[]);
            request.target("recovered", "i64", &[]);
            context
                .apply(
                    &mut recovered,
                    &format!(
                        "{}{}{}{}",
                        binding("add", old_standard),
                        binding("add", old_package),
                        module(),
                        request.text
                    ),
                )
                .unwrap();
            context
                .cli(Some(&recovered.path), &["check"], true)
                .unwrap();
            let result = context
                .cli(
                    Some(&recovered.path),
                    &["run", "recovered", "--arguments", "[]"],
                    true,
                )
                .unwrap();
            assert_eq!(field(&result, "execution", "value").unwrap(), "8390656");
            recovery = json!({"predecessor_source_package":old_package.id,"predecessor_source_revision":old_package.revision,"old_artifact_rejection":rejected,"fresh_current_execution":8390656,"old_producer_removed":true});
        }
        if label == "baseline" {
            predecessor = Some((standard, package));
        }
        assert_eq!(
            digest_file(&original, MAXIMUM_EXECUTABLE_BYTES).unwrap(),
            context.receipt.candidate_sha256
        );
        isolated.close().unwrap();
        evidence::publish_json(&root.join("comparison.json"),&json!({"classification":"fresh passed measurement","candidate_sha256":context.receipt.candidate_sha256,"verifier_sha256":context.receipt.verifier_sha256,"commands":context.receipt.commands,"observations":observations,"predecessor_recovery":recovery,"cleanup_complete":!context.root.exists(),"timing_scope":"complete public CLI preparation, both evaluators and materialization; three repeats per case; no speedup claim"})).unwrap();
    }
}
