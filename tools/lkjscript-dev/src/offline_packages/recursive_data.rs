//! Restarted copied services, with independent reads of explicitly disposable data.
use super::*;
use lkjscript::platform::data::{DataLimits, DataScanDirection, DataStore};
use serde_json::{Value, json};

pub(super) struct Fixture {
    package: Package,
    standalone: PathBuf,
    original_bytes: Vec<u8>,
}

fn persisted(standalone: &Path) -> Result<Vec<u8>, DevError> {
    let store = DataStore::open(&standalone.join("data"), "recursive", DataLimits::default())
        .map_err(|e| DevError::corrupt(e.to_string()))?;
    let tx = store
        .begin()
        .map_err(|e| DevError::corrupt(e.to_string()))?;
    let scan = tx
        .scan(
            "trees",
            &[],
            DataScanDirection::Forward,
            2,
            1_048_576,
            1000,
            None,
        )
        .map_err(|e| DevError::corrupt(e.to_string()))?;
    require(
        scan.items.len() == 1 && scan.continuation.is_none(),
        "recursive store must have exactly one complete entry",
    )?;
    Ok(scan.items[0].value.clone())
}

fn round(
    context: &Context,
    standalone: &Path,
    label: &str,
    steps: &[(&str, Value, u16, Option<Value>)],
) -> Result<Value, DevError> {
    crate::stateful_http::recursive_round(
        &context.binary,
        standalone,
        &context.evidence,
        label,
        |address| {
            let mut observations = Vec::new();
            for (mode, tree, status, expected) in steps {
                let body = json!({"mode":mode,"value":tree});
                let response = crate::http_probe::request(
                    address,
                    "GET",
                    "/",
                    &serde_json::to_vec(&body)?,
                    &[],
                )?;
                require(
                    response.status == *status,
                    &format!(
                        "recursive HTTP {mode} status {} differs from {status}: {:?}",
                        response.status, response.headers
                    ),
                )?;
                let value: Value = if expected.is_some() {
                    serde_json::from_slice(&response.body)?
                } else {
                    json!(String::from_utf8_lossy(&response.body))
                };
                require(
                    expected.as_ref().is_none_or(|expected| &value == expected),
                    "recursive persisted HTTP full value differs",
                )?;
                observations.push(json!({"request":body,"status":response.status,"value":value,"elapsed_nanoseconds":response.elapsed_nanoseconds,"failure":response.headers.get("x-lkjscript-failure-code")}));
            }
            Ok(json!(observations))
        },
    )
}

fn build(context: &mut Context, fixture: &Fixture, label: &str) -> Result<(), DevError> {
    let candidate = context
        .evidence
        .join(format!("recursive-data-{label}.lkja"));
    context.cli(Some(&fixture.package.path), &["check"], true)?;
    context.cli(
        Some(&fixture.package.path),
        &["build", "--output", &candidate.display().to_string()],
        true,
    )?;
    fs::copy(candidate, fixture.standalone.join("application.lkja"))?;
    Ok(())
}

pub(super) fn start(
    context: &mut Context,
    standard: &mut Package,
    library: &Package,
    names: &BTreeMap<String, String>,
) -> Result<Fixture, DevError> {
    for name in [
        "data-encode",
        "data-decode-or",
        "json-decode-or",
        "bytes-equal",
        "DataKeyPart",
        "DataExpectation",
        "DataEntry",
        "DataStore",
    ] {
        let records = context.cli(
            None,
            &["package", "builtin", "query", "owners", "--name", name],
            true,
        )?;
        standard
            .symbols
            .insert(name.into(), field(&records, "owner", "reference")?);
    }
    for (name, kind, member) in [
        ("DataKeyPart", "variant", "case"),
        ("DataExpectation", "variant", "case"),
        ("DataEntry", "record", "field"),
        ("DataStore", "interface", "operation"),
    ] {
        let owner = standard.symbols[name]
            .rsplit('/')
            .next()
            .ok_or_else(|| DevError::corrupt("standard owner id absent"))?
            .to_owned();
        let records = context.cli(
            None,
            &["package", "builtin", "inspect", "owner", kind, &owner],
            true,
        )?;
        for record in records.iter().filter(|record| record.operation == "owner") {
            if record_field(record, "kind")? == member {
                standard.symbols.insert(
                    format!("{name}.{}", record_field(record, "name")?),
                    record_field(record, "reference")?,
                );
            }
        }
    }
    let path = context.root.join("recursive-data-producer");
    let created = context.cli(
        None,
        &[
            "new",
            &path.display().to_string(),
            "--template",
            "http",
            "--name",
            "recursive-data",
        ],
        true,
    )?;
    let mut package = Package {
        path,
        id: field(&created, "package", "id")?,
        revision: field(&created, "revision", "id")?,
        logical: String::new(),
        transport: String::new(),
        container: PathBuf::new(),
        symbols: BTreeMap::new(),
    };
    context.stage(&package, library)?;
    let module = context.cli(
        Some(&package.path),
        &["query", "find", "module", "application"],
        true,
    )?;
    let module = field(&module, "owner", "id")?;
    let function = context.cli(
        Some(&package.path),
        &[
            "query",
            "find",
            "declaration",
            "handle",
            "--parent",
            &module,
        ],
        true,
    )?;
    let function = field(&function, "owner", "id")?;
    let component = context.cli(
        Some(&package.path),
        &[
            "query",
            "find",
            "declaration",
            "application",
            "--parent",
            &module,
        ],
        true,
    )?;
    let component = field(&component, "owner", "id")?;
    let definition = context.cli(
        Some(&package.path),
        &[
            "inspect",
            "owner",
            "task_function",
            &function,
            "--detail",
            "definition",
            "--limit",
            "1000",
            "--bytes",
            "1048576",
        ],
        true,
    )?;
    let requirement = definition
        .iter()
        .find(|r| {
            r.operation == "definition.reference"
                && record_field(r, "role").is_ok_and(|role| role == "function_requirement")
        })
        .ok_or_else(|| DevError::corrupt("HTTP stream requirement absent"))?;
    let bindings = BTreeMap::from([
        ("module".into(), module),
        ("function".into(), function),
        ("component".into(), component),
        (
            "parameter".into(),
            field(&definition, "definition.parameter", "id")?,
        ),
        ("streams".into(), record_field(requirement, "target")?),
    ]);
    context.apply(
        &mut package,
        &format!(
            "{}{}",
            binding("add", library),
            super::recursive_data_program::program(&standard.symbols, &bindings, names)
        ),
    )?;
    context.export(&mut package)?;
    let standalone = context.root.join("recursive-data-standalone");
    fs::create_dir(&standalone)?;
    let mut descriptor: Value = serde_json::from_slice(&process::read_bounded(
        &package.path.join("service.deployment.json"),
        MAXIMUM_OUTPUT_BYTES,
    )?)?;
    descriptor["artifact"] = json!("application.lkja");
    descriptor["listen"] = json!("127.0.0.1:0");
    let limits = DataLimits {
        maximum_live_transactions: 1,
        ..Default::default()
    };
    descriptor["grants"].as_array_mut().ok_or_else(||DevError::corrupt("HTTP grants absent"))?.push(json!({"requirement":"data","sharing_domain":"recursive-data","authority_revision":"88".repeat(32),"adapter":{"kind":"data","root":"data","namespace":"recursive","limits":limits}}));
    fs::write(
        standalone.join("service.deployment.json"),
        evidence::encode_json(&descriptor)?,
    )?;
    fs::write(
        context.evidence.join("recursive-data.deployment.json"),
        evidence::encode_json(&descriptor)?,
    )?;
    context.cli(
        None,
        &[
            "data",
            "initialize",
            "--root",
            &standalone.join("data").display().to_string(),
        ],
        true,
    )?;
    let mut fixture = Fixture {
        package,
        standalone,
        original_bytes: Vec::new(),
    };
    build(context, &fixture, "initial")?;
    let original = super::recursive::small();
    let changed = super::recursive::map_expected(&original, 3, 5);
    let first = round(
        context,
        &fixture.standalone,
        "recursive-data-initial",
        &[
            ("write", original.clone(), 200, Some(original.clone())),
            ("trap", changed, 500, None),
            ("read", original.clone(), 200, Some(original.clone())),
        ],
    )?;
    fixture.original_bytes = persisted(&fixture.standalone)?;
    let head = digest_file(&fixture.standalone.join("data/HEAD"), MAXIMUM_OUTPUT_BYTES)?;
    let spec = process::ProcessSpec {
        command: vec![
            std::env::current_exe()?.display().to_string(),
            "recursive-transaction-probe".into(),
            fixture
                .standalone
                .join("service.deployment.json")
                .display()
                .to_string(),
            fixture.package.symbols["$write"].clone(),
        ],
        cwd: fixture.standalone.clone(),
        environment: BTreeMap::from([("LANG".into(), "C.UTF-8".into())]),
        timeout: Duration::from_secs(120),
        maximum_stdout_bytes: MAXIMUM_OUTPUT_BYTES,
        maximum_stderr_bytes: MAXIMUM_OUTPUT_BYTES,
        stdout_path: context.evidence.join("recursive-transaction.stdout"),
        stderr_path: context.evidence.join("recursive-transaction.stderr"),
        unavailable_exit_code: None,
    };
    let observation = process::run(&spec, &context.evidence);
    require(
        observation.status == process::ProcessStatus::Passed,
        "recursive transaction cancellation probe failed",
    )?;
    require(
        persisted(&fixture.standalone)? == fixture.original_bytes
            && digest_file(&fixture.standalone.join("data/HEAD"), MAXIMUM_OUTPUT_BYTES)? == head,
        "cancelled recursive write changed stored authority",
    )?;
    context.receipt.recursive.transaction_cancellation = serde_json::from_slice(
        &process::read_bounded(&spec.stdout_path, MAXIMUM_OUTPUT_BYTES)?,
    )?;
    context.receipt.runners.push(observation);
    let expected = expected_bytes(&fixture.package, library, &original)?;
    require(
        fixture.original_bytes == expected,
        "recursive persisted bytes differ from independent layout and payload",
    )?;
    fs::write(
        context.evidence.join("recursive-data-independent.bin"),
        &expected,
    )?;
    let restarted = round(
        context,
        &fixture.standalone,
        "recursive-data-restarted",
        &[
            ("read", original.clone(), 200, Some(original.clone())),
            ("write", original.clone(), 200, Some(original)),
        ],
    )?;
    require(
        persisted(&fixture.standalone)? == expected,
        "recursive restart changed persisted bytes",
    )?;
    context.receipt.recursive.persistence.push(first);
    context.receipt.recursive.persistence.push(restarted);
    context.receipt.recursive.persisted_bytes_sha256 = Sha256::digest(&expected)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Ok(fixture)
}

pub(super) fn finish(
    context: &mut Context,
    mut fixture: Fixture,
    library: &Package,
) -> Result<(), DevError> {
    context.stage(&fixture.package, library)?;
    context.apply(&mut fixture.package, &binding("replace", library))?;
    build(context, &fixture, "body-only")?;
    let original = super::recursive::small();
    let body = round(
        context,
        &fixture.standalone,
        "recursive-data-body-only",
        &[("read", original.clone(), 200, Some(original.clone()))],
    )?;
    require(
        persisted(&fixture.standalone)? == fixture.original_bytes,
        "body-only dependency rebuild changed layout or bytes",
    )?;
    let field = fixture.package.symbols["$stored-value"].clone();
    context.apply(
        &mut fixture.package,
        &format!("rename.owner owner={field} name=renamed\n"),
    )?;
    build(context, &fixture, "layout-change")?;
    let mismatch = round(
        context,
        &fixture.standalone,
        "recursive-data-layout-change",
        &[("read", original, 409, Some(json!("layout-mismatch")))],
    )?;
    require(
        persisted(&fixture.standalone)? == fixture.original_bytes,
        "layout mismatch overwrote existing record",
    )?;
    context.receipt.recursive.persistence.push(body);
    context.receipt.recursive.persistence.push(mismatch);
    context.cli(
        None,
        &[
            "data",
            "verify",
            "--root",
            &fixture.standalone.join("data").display().to_string(),
        ],
        true,
    )?;
    fs::remove_dir_all(&fixture.package.path)?;
    fs::remove_file(&fixture.package.container)?;
    fs::remove_dir_all(&fixture.standalone)?;
    Ok(())
}

// The independent format oracle shares only canonical TypeObject identity, never either
// evaluator's instantiated layouts or either typed codec's traversal and payload encoder.
fn expected_bytes(stored: &Package, library: &Package, value: &Value) -> Result<Vec<u8>, DevError> {
    fn ty(form: Value) -> Result<([u8; 32], String), DevError> {
        lkjscript::platform::contributor::canonical_type_object_identity(&serde_json::to_vec(
            &form,
        )?)
        .map_err(|e| DevError::corrupt(e.to_string()))
    }
    fn blob(out: &mut Vec<u8>, text: &str) {
        out.extend_from_slice(&(text.len() as u32).to_be_bytes());
        out.extend_from_slice(text.as_bytes());
    }
    fn hash(domain: &str, bytes: &[u8]) -> [u8; 32] {
        let mut h = blake3::Hasher::new_derive_key(domain);
        h.update(&(bytes.len() as u64).to_be_bytes());
        h.update(bytes);
        *h.finalize().as_bytes()
    }
    let integer = ty(json!({"kind":"i64"}))?;
    let tree = ty(
        json!({"kind":"applied","declaration":{"package":library.id,"declaration":library.symbols["$tree"]},"arguments":[integer.1]}),
    )?;
    let list = ty(json!({"kind":"list","item":tree.1}))?;
    let root = ty(
        json!({"kind":"named","declaration":{"package":stored.id,"declaration":stored.symbols["$Stored"]}}),
    )?;
    let mut header = tree.0.to_vec();
    header.push(9);
    header.extend_from_slice(&1_u32.to_be_bytes());
    header.extend_from_slice(&integer.0);
    header.push(2);
    blob(&mut header, &library.id);
    blob(&mut header, &library.symbols["$tree"]);
    let mut tree_description = header.clone();
    tree_description.extend_from_slice(&[1, 1]);
    tree_description.extend_from_slice(&2_u32.to_be_bytes());
    let cases = BTreeMap::from([
        (library.symbols["$leaf"].clone(), "leaf"),
        (library.symbols["$branch"].clone(), "branch"),
    ]);
    let mut indices = BTreeMap::new();
    for (index, (id, name)) in cases.into_iter().enumerate() {
        indices.insert(name, index as u32);
        blob(&mut tree_description, &library.id);
        blob(&mut tree_description, &id);
        blob(&mut tree_description, name);
        tree_description.push(1);
        if name == "leaf" {
            tree_description.extend_from_slice(&integer.0);
            tree_description.push(2);
        } else {
            tree_description.extend_from_slice(&list.0);
            tree_description.push(7);
            tree_description.extend_from_slice(&header);
            tree_description.push(0);
        }
    }
    let mut description = root.0.to_vec();
    description.push(5);
    blob(&mut description, &stored.id);
    blob(&mut description, &stored.symbols["$Stored"]);
    description.extend_from_slice(&[1, 0]);
    description.extend_from_slice(&1_u32.to_be_bytes());
    blob(&mut description, &stored.id);
    blob(&mut description, &stored.symbols["$stored-value"]);
    blob(&mut description, "value");
    description.extend_from_slice(&tree_description);
    let mut out = b"LKJDVAL1".to_vec();
    out.extend_from_slice(&1_u16.to_be_bytes());
    out.extend_from_slice(&hash("lkjscript.data.typed-layout.v1", &description));
    let mut pending = vec![value];
    while let Some(value) = pending.pop() {
        let case = value["case"]
            .as_str()
            .ok_or_else(|| DevError::corrupt("expected tree case absent"))?;
        out.extend_from_slice(&indices[case].to_be_bytes());
        if case == "leaf" {
            out.extend_from_slice(
                &value["value"]
                    .as_i64()
                    .ok_or_else(|| DevError::corrupt("expected integer absent"))?
                    .to_be_bytes(),
            );
        } else {
            let children = value["value"]
                .as_array()
                .ok_or_else(|| DevError::corrupt("expected branch absent"))?;
            out.extend_from_slice(&(children.len() as u32).to_be_bytes());
            pending.extend(children.iter().rev());
        }
    }
    out.extend_from_slice(&hash("lkjscript.data.typed-value-envelope.v1", &out));
    Ok(out)
}
