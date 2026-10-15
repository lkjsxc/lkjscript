use super::*;

const PRODUCER: &str = "create.module as=$module name=boundary-producer\ncreate.function as=$answer-function module=$module name=answer visibility=public result=i64 effect=pure body=$answer\nexpression.i64 as=$answer value=42\n";
const TARGET: &str = "create.module as=$module name=boundary-consumer\ncreate.component as=$component module=$module name=application visibility=public\nexpression.call as=$value function=$answer-function\ncreate.function as=$entry module=$module name=entry visibility=public result=i64 effect=pure body=$value\ntype.function as=@Entry result=i64\nadd.port as=$port component=$component name=main type=@Entry function=$entry\ncreate.target as=$target name=main component=$component port=$port runner=command\n";

fn body(package: &str, revision: &str, logical: &str) -> String {
    format!(
        "add.dependency package={package} semantic-revision={revision} package-revision={logical}\nreference.package as=$producer package={package} package-revision={logical}\nreference.owner as=$answer-function package=$producer class=declaration name=answer\n{TARGET}"
    )
}

fn author(
    context: &mut BoundaryContext,
    name: &str,
    project: &Path,
    base: &str,
    body: &str,
) -> Result<Vec<CompactRecord>, DevError> {
    let request = context.evidence.join(format!("{name}.lkjc"));
    fs::write(&request, format!("request base={base}\n{body}"))?;
    let review = context.evidence.join(format!("{name}.lkjplan"));
    let planned = context.cli(
        &format!("{name}-plan"),
        Some(project),
        &[
            "change",
            "plan",
            "--input-file",
            &request.display().to_string(),
            "--output",
            &review.display().to_string(),
        ],
        true,
    )?;
    let token = field(&planned, "plan", "token")?;
    let decoded = decode_logical_change_plan(std::io::BufReader::new(fs::File::open(&review)?))
        .map_err(|error| DevError::corrupt(error.to_string()))?;
    require(
        decoded.token == token,
        "final authoring review and apply token differ",
    )?;
    context.cli(
        &format!("{name}-apply"),
        Some(project),
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
}

pub(super) fn workflow(context: &mut BoundaryContext) -> Result<(), DevError> {
    context.cli("capabilities", None, &["capabilities"], true)?;
    context.cli(
        "builtin-inspect",
        None,
        &["package", "builtin", "inspect"],
        true,
    )?;
    let producer = context.root.join("producer");
    let created = context.cli(
        "producer-new",
        None,
        &[
            "new",
            &producer.display().to_string(),
            "--template",
            "minimal",
            "--name",
            "boundary-producer",
        ],
        true,
    )?;
    let applied = author(
        context,
        "producer",
        &producer,
        &field(&created, "revision", "id")?,
        PRODUCER,
    )?;
    let producer_id = field(&created, "package", "id")?;
    let revision = field(&applied, "revision", "result")?;
    let transport_file = context.evidence.join("producer.lkjp");
    let exported = context.cli(
        "producer-export",
        Some(&producer),
        &[
            "package",
            "current",
            "export",
            "--kind",
            "transport",
            "--output",
            &transport_file.display().to_string(),
        ],
        true,
    )?;
    let transport = field(&exported, "package", "transport")?;
    let logical = field(&exported, "package", "package-revision")?;
    let consumer = context.root.join("consumer");
    let created = context.cli(
        "consumer-new",
        None,
        &[
            "new",
            &consumer.display().to_string(),
            "--template",
            "minimal",
            "--name",
            "boundary-consumer",
        ],
        true,
    )?;
    let before_stage = super::super::pack_files(&consumer)?;
    let stage_authority = crate::authority::observe_graph_authority(&consumer)?;
    context.cli(
        "consumer-stage",
        Some(&consumer),
        &[
            "package",
            "dependency",
            "stage",
            "--transport",
            &transport,
            "--input-file",
            &transport_file.display().to_string(),
        ],
        true,
    )?;
    require(
        stage_authority.head_sha256
            == crate::authority::observe_graph_authority(&consumer)?.head_sha256,
        "final transport staging advanced accepted HEAD",
    )?;
    let installed = super::super::pack_files(&consumer)?
        .difference(&before_stage)
        .cloned()
        .collect::<Vec<_>>();
    require(
        installed.len() == 1,
        "final one-package transport did not install one source pack",
    )?;
    author(
        context,
        "consumer",
        &consumer,
        &field(&created, "revision", "id")?,
        &body(&producer_id, &revision, &logical),
    )?;
    context.cli(
        "consumer-export",
        Some(&consumer),
        &[
            "package",
            "current",
            "export",
            "--kind",
            "transport",
            "--output",
            &context.evidence.join("consumer.lkjp").display().to_string(),
        ],
        true,
    )?;
    let head = digest_file(&consumer.join("HEAD"), 1024 * 1024)?;
    let mut builds = Vec::new();
    for name in [
        "build-cold",
        "build-warm",
        "build-missing-cache",
        "build-corrupt-cache",
    ] {
        let current = consumer.join("derived/compiler/CURRENT");
        if name == "build-missing-cache" {
            fs::remove_file(&current)?;
        }
        if name == "build-corrupt-cache" {
            fs::write(&current, b"invalid disposable compiler cache")?;
        }
        let path = context.evidence.join(format!("{name}.lkja"));
        context.cli(
            name,
            Some(&consumer),
            &["build", "--output", &path.display().to_string()],
            true,
        )?;
        builds.push(digest_file(&path, MAXIMUM_CONTAINER_BYTES)?);
        require(
            digest_file(&consumer.join("HEAD"), 1024 * 1024)? == head,
            "final cached build advanced semantic HEAD",
        )?;
    }
    require(
        builds.windows(2).all(|pair| pair[0] == pair[1]),
        "final cache state changed exact artifact bytes",
    )?;
    context.receipt.assertions.insert("cache-builds".into(),json!({"sha256":builds,"head_before":head,"head_after":digest_file(&consumer.join("HEAD"),1024*1024)?}));
    context.cli("check", Some(&consumer), &["check"], true)?;
    let result = context.cli(
        "project-run",
        Some(&consumer),
        &["run", "main", "--arguments", "[]"],
        true,
    )?;
    require(
        field(&result, "execution", "value")? == "42"
            && field(&result, "execution", "differential")? == "equal",
        "final source public run differs from literal 42",
    )?;
    let input = context.evidence.join("batch.input.jsonl");
    fs::write(&input, batch_input())?;
    let result_path = context.evidence.join("batch.result.jsonl");
    context.cli(
        "source-batch",
        Some(&consumer),
        &[
            "run",
            "main",
            "--batch-input-file",
            &input.display().to_string(),
            "--batch-result-file",
            &result_path.display().to_string(),
        ],
        true,
    )?;
    let before = crate::authority::observe_graph_authority(&consumer)?;
    let bad = context.evidence.join("bad-transport.lkjp");
    fs::write(&bad, b"invalid current transport")?;
    let rejected = context.cli(
        "bad-transport",
        Some(&consumer),
        &[
            "package",
            "dependency",
            "stage",
            "--transport",
            &transport,
            "--input-file",
            &bad.display().to_string(),
        ],
        false,
    )?;
    let bad_transport = field(&rejected, "diagnostic", "code")?;
    require(
        bad_transport == "package_container_contract",
        "invalid transport changed its exact current refusal",
    )?;
    require(
        crate::authority::observe_graph_authority(&consumer)? == before,
        "final rejected transport changed accepted authority",
    )?;
    let inventory = offline_package_inventory(
        &process::read_bounded(&transport_file, MAXIMUM_CONTAINER_BYTES)?,
        &transport,
    )
    .map_err(|error| DevError::corrupt(error.to_string()))?;
    let literal = applied
        .iter()
        .filter(|record| record.operation == "identity")
        .find(|record| record_field(record, "symbol").is_ok_and(|symbol| symbol == "$answer"))
        .ok_or_else(|| DevError::corrupt("final producer literal allocation absent"))?;
    let owner = record_field(literal, "id")?;
    let object = inventory
        .packages
        .iter()
        .flat_map(|package| &package.owners)
        .find(|(id, _, _)| id == &owner)
        .ok_or_else(|| DevError::corrupt("final producer literal missing from transport"))?;
    let body = offline_package_owner_bytes(
        &process::read_bounded(&transport_file, MAXIMUM_CONTAINER_BYTES)?,
        &transport,
        &object.2,
    )
    .map_err(|error| DevError::corrupt(error.to_string()))?;
    let pack = &installed[0];
    let original = process::read_bounded(pack, MAXIMUM_CONTAINER_BYTES)?;
    require(!body.is_empty(), "canonical literal body is empty")?;
    let positions = original
        .windows(body.len())
        .enumerate()
        .filter_map(|(i, bytes)| (bytes == body).then_some(i))
        .collect::<Vec<_>>();
    require(
        positions.len() == 1 && !body.is_empty(),
        "final canonical fault did not locate one literal source body",
    )?;
    let mut corrupt = original.clone();
    corrupt[positions[0] + body.len() - 1] ^= 1;
    fs::write(pack, corrupt)?;
    let rejected = context.cli(
        "canonical-source-refusal",
        Some(&consumer),
        &["check"],
        false,
    );
    fs::write(pack, &original)?;
    let rejected = rejected?;
    require(
        field(&rejected, "diagnostic", "code")? == "pack_entry_checksum"
            && crate::authority::observe_graph_authority(&consumer)? == before,
        "final canonical corruption became a cache miss or mutated authority",
    )?;
    context.receipt.assertions.insert("source-refusals".into(),json!({"transport":bad_transport,"canonical":"pack_entry_checksum","authority_before":before,"authority_after":crate::authority::observe_graph_authority(&consumer)?}));
    let bundle = context.root.join("detached");
    fs::create_dir(&bundle)?;
    fs::copy(
        context.evidence.join("build-cold.lkja"),
        bundle.join("application.lkja"),
    )?;
    let descriptor = descriptor("application.lkja");
    fs::write(
        bundle.join("command.deployment.json"),
        evidence::encode_json(&descriptor)?,
    )?;
    fs::write(
        context.evidence.join("detached.deployment.json"),
        evidence::encode_json(&descriptor)?,
    )?;
    let bad = bundle.join("bad.lkja");
    fs::write(&bad, b"invalid current artifact")?;
    fs::write(
        bundle.join("bad.deployment.json"),
        evidence::encode_json(&self::descriptor("bad.lkja"))?,
    )?;
    fs::remove_dir_all(&producer)?;
    fs::remove_dir_all(&consumer)?;
    let unrelated = context.root.join("unrelated");
    fs::create_dir(&unrelated)?;
    let bad_descriptor = bundle.join("bad.deployment.json");
    let spec = context.specification(
        "bad-artifact",
        &unrelated,
        &strings(&[
            "run",
            "--deployment",
            &bad_descriptor.display().to_string(),
            "--arguments",
            "[]",
        ]),
    );
    let observation = process::run(&spec, &context.evidence);
    let rejected = context.retain("bad-artifact", &spec, false, observation)?;
    context.receipt.assertions.insert(
        "bad-artifact-code".into(),
        json!(field(&rejected, "diagnostic", "code")?),
    );
    let descriptor = bundle.join("command.deployment.json");
    let result = context.evidence.join("detached.result.json");
    let args = strings(&[
        "run",
        "--deployment",
        &descriptor.display().to_string(),
        "--arguments",
        "[]",
        "--result-file",
        &result.display().to_string(),
    ]);
    let spec = context.specification("detached-result", &unrelated, &args);
    let observation = process::run(&spec, &context.evidence);
    context.retain("detached-result", &spec, true, observation)?;
    require(
        process::read_bounded(&result, 1024)? == b"42",
        "detached actual result file differs from literal 42",
    )?;
    let spec = context.specification("existing-result", &unrelated, &args);
    let observation = process::run(&spec, &context.evidence);
    let rejected = context.retain("existing-result", &spec, false, observation)?;
    require(
        field(&rejected, "diagnostic", "code")? == "output_conflict"
            && process::read_bounded(&result, 1024)? == b"42",
        "existing result file was overwritten or accepted",
    )?;
    context.receipt.assertions.insert("detached".into(),json!({"checkout_unavailable":!producer.exists() && !consumer.exists(),"result":42,"existing_bytes_unchanged":true}));
    Ok(())
}

fn descriptor(artifact: &str) -> Value {
    json!({"artifact":artifact,"target":"main","listen":null,"http":null,"session":null,"worker":null,"streams":lkjscript::platform::stream::StreamLimits::default(),"configuration":{},"secrets":[],"grants":[]})
}
fn batch_input() -> Vec<u8> {
    [
        ("success", "[]"),
        ("arity-refusal", "[1]"),
        ("post-refusal-recovery", "[]"),
    ]
    .into_iter()
    .flat_map(|(id, arguments)| {
        format!(
            "{}\n",
            json!({"id":id,"target":"main","arguments_json":arguments})
        )
        .into_bytes()
    })
    .collect()
}

fn validate_author(
    receipt: &BoundaryReceipt,
    root: &Path,
    name: &str,
    base: &str,
    body: &str,
) -> Result<Vec<CompactRecord>, DevError> {
    let request = root.join(format!("{name}.lkjc"));
    let review = root.join(format!("{name}.lkjplan"));
    require(
        process::read_bounded(&request, MAXIMUM_OUTPUT_BYTES)?
            == format!("request base={base}\n{body}").as_bytes(),
        "final authoring literal or base substituted",
    )?;
    let planned = records(
        receipt,
        root,
        &format!("{name}-plan"),
        Some(name),
        &strings(&[
            "change",
            "plan",
            "--input-file",
            &request.display().to_string(),
            "--output",
            &review.display().to_string(),
        ]),
        true,
    )?;
    let token = field(&planned, "plan", "token")?;
    let decoded = decode_logical_change_plan(std::io::BufReader::new(fs::File::open(review)?))
        .map_err(|error| DevError::corrupt(error.to_string()))?;
    require(
        decoded.token == token,
        "final retained review token substituted",
    )?;
    records(
        receipt,
        root,
        &format!("{name}-apply"),
        Some(name),
        &strings(&[
            "change",
            "apply",
            "--input-file",
            &request.display().to_string(),
            "--plan",
            &token,
        ]),
        true,
    )
}

pub(super) fn validate(receipt: &BoundaryReceipt, root: &Path) -> Result<(), DevError> {
    let capabilities = records(
        receipt,
        root,
        "capabilities",
        None,
        &strings(&["capabilities"]),
        true,
    )?;
    require(
        field(&capabilities, "result", "status")? == "success"
            && field(&capabilities, "result", "command")? == "capabilities",
        "final capabilities response substituted",
    )?;
    let builtin = records(
        receipt,
        root,
        "builtin-inspect",
        None,
        &strings(&["package", "builtin", "inspect"]),
        true,
    )?;
    require(
        field(&builtin, "result", "status")? == "success"
            && field(&builtin, "result", "command")? == "package.builtin.inspect",
        "final builtin discovery response substituted",
    )?;
    let project = |name| Path::new(&receipt.isolated_root).join(name);
    let created = records(
        receipt,
        root,
        "producer-new",
        None,
        &strings(&[
            "new",
            &project("producer").display().to_string(),
            "--template",
            "minimal",
            "--name",
            "boundary-producer",
        ]),
        true,
    )?;
    let producer = field(&created, "package", "id")?;
    let applied = validate_author(
        receipt,
        root,
        "producer",
        &field(&created, "revision", "id")?,
        PRODUCER,
    )?;
    let revision = field(&applied, "revision", "result")?;
    let transport_file = root.join("producer.lkjp");
    let exported = records(
        receipt,
        root,
        "producer-export",
        Some("producer"),
        &strings(&[
            "package",
            "current",
            "export",
            "--kind",
            "transport",
            "--output",
            &transport_file.display().to_string(),
        ]),
        true,
    )?;
    let logical = field(&exported, "package", "package-revision")?;
    let transport = field(&exported, "package", "transport")?;
    let inventory = offline_package_inventory(
        &process::read_bounded(&transport_file, MAXIMUM_CONTAINER_BYTES)?,
        &transport,
    )
    .map_err(|error| DevError::corrupt(error.to_string()))?;
    require(
        inventory.packages.len() == 1
            && inventory.packages[0].package == producer
            && inventory.packages[0].semantic_revision == revision,
        "final exported source identity substituted",
    )?;
    let created = records(
        receipt,
        root,
        "consumer-new",
        None,
        &strings(&[
            "new",
            &project("consumer").display().to_string(),
            "--template",
            "minimal",
            "--name",
            "boundary-consumer",
        ]),
        true,
    )?;
    records(
        receipt,
        root,
        "consumer-stage",
        Some("consumer"),
        &strings(&[
            "package",
            "dependency",
            "stage",
            "--transport",
            &transport,
            "--input-file",
            &transport_file.display().to_string(),
        ]),
        true,
    )?;
    let consumer_applied = validate_author(
        receipt,
        root,
        "consumer",
        &field(&created, "revision", "id")?,
        &body(&producer, &revision, &logical),
    )?;
    let consumer_export = records(
        receipt,
        root,
        "consumer-export",
        Some("consumer"),
        &strings(&[
            "package",
            "current",
            "export",
            "--kind",
            "transport",
            "--output",
            &root.join("consumer.lkjp").display().to_string(),
        ]),
        true,
    )?;
    let consumer_transport = field(&consumer_export, "package", "transport")?;
    let consumer_binding = lkjscript::platform::contributor::strict_artifact_source_probe(
        &process::read_bounded(&root.join("build-cold.lkja"), MAXIMUM_CONTAINER_BYTES)?,
        &process::read_bounded(&root.join("consumer.lkjp"), MAXIMUM_CONTAINER_BYTES)?,
        &consumer_transport,
    )
    .map_err(|error| DevError::corrupt(error.to_string()))?;
    require(
        consumer_binding["package"] == field(&created, "package", "id")?
            && consumer_binding["revision"] == field(&consumer_applied, "revision", "result")?
            && consumer_binding["package"] == field(&consumer_export, "package", "id")?
            && consumer_binding["revision"] == field(&consumer_export, "package", "revision")?
            && consumer_binding["package_revision"]
                == field(&consumer_export, "package", "package-revision")?
            && consumer_binding["packages"] == 2,
        "final fresh consumer artifact differs from its actual exported source closure",
    )?;
    let mut builds = Vec::new();
    for name in [
        "build-cold",
        "build-warm",
        "build-missing-cache",
        "build-corrupt-cache",
    ] {
        let path = root.join(format!("{name}.lkja"));
        let built = records(
            receipt,
            root,
            name,
            Some("consumer"),
            &strings(&["build", "--output", &path.display().to_string()]),
            true,
        )?;
        let artifact = lkjscript::platform::contributor::strict_artifact_identity_probe(
            &process::read_bounded(&path, MAXIMUM_CONTAINER_BYTES)?,
        )
        .map_err(|error| DevError::corrupt(error.to_string()))?;
        require(
            field(&built, "artifact", "bundle")? == artifact
                && field(&built, "artifact", "bytes")? == fs::metadata(&path)?.len().to_string(),
            "final build receipt changed actual artifact identity or bytes",
        )?;
        builds.push(digest_file(&path, MAXIMUM_CONTAINER_BYTES)?);
    }
    let cache = &receipt.assertions["cache-builds"];
    require(
        builds.windows(2).all(|pair| pair[0] == pair[1])
            && cache["sha256"] == json!(builds)
            && cache["head_before"]
                .as_str()
                .is_some_and(|head| head.len() == 64)
            && cache["head_after"] == cache["head_before"],
        "final cache bytes or HEAD preservation substituted",
    )?;
    let checked = records(
        receipt,
        root,
        "check",
        Some("consumer"),
        &strings(&["check"]),
        true,
    )?;
    require(
        field(&checked, "result", "status")? == "success"
            && field(&checked, "result", "command")? == "check"
            && field(&checked, "tests", "passed")? == "0"
            && field(&checked, "tests", "failed")? == "0"
            && field(&checked, "tests", "differential")? == "equal"
            && field(&checked, "artifact", "packages")? == "2",
        "final check route changed its exact test/closure outcome",
    )?;
    let result = records(
        receipt,
        root,
        "project-run",
        Some("consumer"),
        &strings(&["run", "main", "--arguments", "[]"]),
        true,
    )?;
    require(
        field(&result, "execution", "value")? == "42"
            && field(&result, "execution", "differential")? == "equal",
        "final source run full value or differential substituted",
    )?;
    require(
        field(&result, "authority", "package")?
            == consumer_binding["package"]
                .as_str()
                .ok_or_else(|| DevError::corrupt("consumer package binding missing"))?
            && field(&result, "authority", "revision")?
                == consumer_binding["revision"]
                    .as_str()
                    .ok_or_else(|| DevError::corrupt("consumer revision binding missing"))?
            && field(&result, "artifact", "bundle")?
                == consumer_binding["bundle"]
                    .as_str()
                    .ok_or_else(|| DevError::corrupt("consumer artifact binding missing"))?
            && field(&result, "artifact", "packages")? == "2"
            && field(&result, "artifact", "bytes")?
                == fs::metadata(root.join("build-cold.lkja"))?
                    .len()
                    .to_string(),
        "final source run differs from the independently admitted exact cold artifact/source closure",
    )?;
    let binding = super::super::source_batch::Binding::new(&result)?;
    let input = root.join("batch.input.jsonl");
    let output = root.join("batch.result.jsonl");
    require(
        process::read_bounded(&input, MAXIMUM_OUTPUT_BYTES)? == batch_input(),
        "final bounded batch row inventory substituted",
    )?;
    let summary = records(
        receipt,
        root,
        "source-batch",
        Some("consumer"),
        &strings(&[
            "run",
            "main",
            "--batch-input-file",
            &input.display().to_string(),
            "--batch-result-file",
            &output.display().to_string(),
        ]),
        true,
    )?;
    binding.require(&summary)?;
    require(
        field(&summary, "batch", "package-revision")?
            == consumer_binding["package_revision"]
                .as_str()
                .ok_or_else(|| DevError::corrupt("final consumer package revision missing"))?
            && field(&summary, "batch", "schema")?
                == lkjscript::platform::canonical_schema::CANONICAL_SCHEMA_HASH_HEX
            && field(&summary, "batch", "validator")?
                == lkjscript::platform::canonical_schema::VALIDATOR_IMPLEMENTATION_HASH
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>(),
        "final source batch current schema/validator/accepted package binding changed",
    )?;
    require(
        field(&summary, "batch", "rows")? == "3"
            && field(&summary, "batch", "passed")? == "2"
            && field(&summary, "batch", "failed")? == "1"
            && field(&summary, "batch", "source-preparations")? == "1"
            && field(&summary, "output", "visibility")? == "created"
            && field(&summary, "output", "durability")? == "synchronized"
            && field(&summary, "output", "stage-cleanup")? == "removed",
        "final batch publication boundary differs",
    )?;
    let row_bytes = process::read_bounded(&output, MAXIMUM_OUTPUT_BYTES)?;
    require(
        field(&summary, "output", "path")? == output.display().to_string()
            && field(&summary, "output", "bytes")? == row_bytes.len().to_string(),
        "final batch output path or byte count changed",
    )?;
    let framed = row_bytes
        .strip_suffix(b"\n")
        .ok_or_else(|| DevError::corrupt("final batch complete row framing missing"))?;
    let rows = framed
        .split(|byte| *byte == b'\n')
        .map(strict_from_slice::<Value>)
        .collect::<Result<Vec<_>, _>>()?;
    require(
        rows.len() == 3,
        "final batch omitted a recovery or refusal row",
    )?;
    for (row, (id, passed)) in rows.iter().zip([
        ("success", true),
        ("arity-refusal", false),
        ("post-refusal-recovery", true),
    ]) {
        require(
            row.as_object().is_some_and(|object| {
                object.len() == 4
                    && ["id", "exit_code", "stdout", "result_json"]
                        .into_iter()
                        .all(|key| object.contains_key(key))
            }),
            "final batch row fields omitted or added",
        )?;
        require(
            row["id"] == id && row["exit_code"] == if passed { 0 } else { 2 },
            "final batch row identity or outcome changed",
        )?;
        let row_records = parse_records(
            "final-batch-row",
            row["stdout"]
                .as_str()
                .ok_or_else(|| DevError::corrupt("batch row stdout omitted"))?
                .as_bytes(),
        )
        .map_err(|_| DevError::corrupt("batch row response invalid"))?;
        if passed {
            binding.require(&row_records)?;
            require(
                row["result_json"] == "42"
                    && field(&row_records, "execution", "value-location")? == "batch-row"
                    && !row_records
                        .iter()
                        .filter(|record| record.operation == "execution")
                        .flat_map(|record| &record.fields)
                        .any(|field| matches!(field.name.as_str(), "value" | "result-file"))
                    && field(&row_records, "execution", "target")? == "main"
                    && field(&row_records, "execution", "differential")? == "equal"
                    && ["production-instructions", "reference-expressions"]
                        .into_iter()
                        .all(|name| {
                            field(&row_records, "execution", name).is_ok_and(|value| {
                                value.parse::<u64>().is_ok_and(|count| count > 0)
                            })
                        }),
                "final batch did not independently execute full literal result",
            )?;
            super::super::source_batch::observations(&row_records)?;
        } else {
            require(
                row["result_json"].is_null()
                    && field(&row_records, "result", "status")? == "failure"
                    && field(&row_records, "result", "command")? == "run"
                    && field(&row_records, "diagnostic", "class")? == "source"
                    && !row_records
                        .iter()
                        .any(|record| record.operation == "execution")
                    && field(&row_records, "diagnostic", "code")?
                        == "normalized_runner_argument_count",
                "final batch refusal changed",
            )?;
        }
    }
    let refusal = &receipt.assertions["source-refusals"];
    let bad = records(
        receipt,
        root,
        "bad-transport",
        Some("consumer"),
        &strings(&[
            "package",
            "dependency",
            "stage",
            "--transport",
            &transport,
            "--input-file",
            &root.join("bad-transport.lkjp").display().to_string(),
        ]),
        false,
    )?;
    require(
        refusal["transport"] == "package_container_contract"
            && field(&bad, "diagnostic", "code")? == "package_container_contract"
            && refusal["authority_before"] == refusal["authority_after"]
            && !refusal["authority_before"].is_null(),
        "final transport refusal changed accepted inventory",
    )?;
    let rejected = records(
        receipt,
        root,
        "canonical-source-refusal",
        Some("consumer"),
        &strings(&["check"]),
        false,
    )?;
    require(
        field(&rejected, "diagnostic", "code")? == "pack_entry_checksum"
            && refusal["canonical"] == "pack_entry_checksum",
        "final canonical fault became a cache miss",
    )?;
    let descriptor = project("detached").join("command.deployment.json");
    require(
        strict_from_slice::<Value>(&process::read_bounded(
            &root.join("detached.deployment.json"),
            MAXIMUM_OUTPUT_BYTES,
        )?)? == self::descriptor("application.lkja"),
        "final detached descriptor substituted",
    )?;
    let rejected = records(
        receipt,
        root,
        "bad-artifact",
        None,
        &strings(&[
            "run",
            "--deployment",
            &project("detached")
                .join("bad.deployment.json")
                .display()
                .to_string(),
            "--arguments",
            "[]",
        ]),
        false,
    )?;
    require(
        receipt.assertions["bad-artifact-code"] == "artifact_bundle_length"
            && field(&rejected, "diagnostic", "code")? == "artifact_bundle_length",
        "final invalid artifact diagnostic substituted",
    )?;
    let args = strings(&[
        "run",
        "--deployment",
        &descriptor.display().to_string(),
        "--arguments",
        "[]",
        "--result-file",
        &root.join("detached.result.json").display().to_string(),
    ]);
    let result = records(receipt, root, "detached-result", None, &args, true)?;
    let artifact = lkjscript::platform::contributor::strict_artifact_identity_probe(
        &process::read_bounded(&root.join("build-cold.lkja"), MAXIMUM_CONTAINER_BYTES)?,
    )
    .map_err(|error| DevError::corrupt(error.to_string()))?;
    closed_execution(&result, &artifact, "main")?;
    require(
        field(&result, "execution", "result-file")?
            == root.join("detached.result.json").display().to_string()
            && field(&result, "output", "path")?
                == root.join("detached.result.json").display().to_string()
            && field(&result, "output", "bytes")? == "2"
            && field(&result, "output", "visibility")? == "created"
            && field(&result, "output", "durability")? == "synchronized"
            && field(&result, "output", "stage-cleanup")? == "removed"
            && process::read_bounded(&root.join("detached.result.json"), 1024)? == b"42",
        "final detached publication differs",
    )?;
    let rejected = records(receipt, root, "existing-result", None, &args, false)?;
    require(
        field(&rejected, "diagnostic", "code")? == "output_conflict"
            && receipt.assertions["detached"]
                == json!({"checkout_unavailable":true,"result":42,"existing_bytes_unchanged":true}),
        "final existing result conflict changed bytes",
    )?;
    for name in ["bad-artifact", "detached-result", "existing-result"] {
        require(
            receipt
                .processes
                .iter()
                .find(|call| call.name == name)
                .is_some_and(|call| call.cwd == project("unrelated").display().to_string()),
            "detached final run lost its unrelated working directory",
        )?;
    }
    Ok(())
}
