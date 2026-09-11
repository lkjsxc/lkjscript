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
