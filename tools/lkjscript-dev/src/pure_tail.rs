//! Bounded copied-public-executable acceptance for pure tail execution.

use crate::{authority, error::DevError, evidence, process, pure_tail_program};
use lkjscript::platform::contributor::offline_producer_inventory;
use lkjscript::platform::control::{CompactRecord, decode_logical_change_plan, parse_records};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};

const MAXIMUM_FILES_BYTES: u64 = 1_073_741_824;
const MAXIMUM_OUTPUT: u64 = 4 * 1024 * 1024;
const MAXIMUM_SECONDS: u64 = 900;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Receipt {
    schema: String,
    status: String,
    pub(crate) candidate_sha256: String,
    copied_candidate_sha256: String,
    pub(crate) verifier_sha256: String,
    isolated_root: String,
    evidence_root: String,
    pub(crate) elapsed_nanoseconds: u64,
    peak_owned_file_bytes: u64,
    commands: Vec<CommandEvidence>,
    outcomes: BTreeMap<String, serde_json::Value>,
    files: Vec<evidence::FileProof>,
    pub(crate) cleanup_complete: bool,
    failure: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CommandEvidence {
    command: Vec<String>,
    observation: process::ProcessObservation,
    expected_success: bool,
}

struct Context {
    root: PathBuf,
    output: PathBuf,
    binary: PathBuf,
    started: Instant,
    receipt: Receipt,
}

struct Package {
    path: PathBuf,
    id: String,
    revision: String,
    logical: String,
    transport: String,
    container: PathBuf,
    symbols: BTreeMap<String, String>,
}

pub(crate) fn command(mut arguments: impl Iterator<Item = OsString>) -> Result<u8, DevError> {
    let mut binary = None;
    let mut output = None;
    let mut machine = false;
    while let Some(option) = crate::next_utf8(&mut arguments, "option")? {
        match option.as_str() {
            "--binary" if binary.is_none() => {
                binary = Some(PathBuf::from(
                    crate::next_utf8(&mut arguments, "binary")?
                        .ok_or_else(|| DevError::usage("missing binary"))?,
                ))
            }
            "--evidence-root" if output.is_none() => {
                output = Some(PathBuf::from(
                    crate::next_utf8(&mut arguments, "evidence root")?
                        .ok_or_else(|| DevError::usage("missing evidence root"))?,
                ))
            }
            "--machine" if !machine => machine = true,
            _ => {
                return Err(DevError::usage(
                    "pure-tail requires --binary PATH --evidence-root ABSENT_ABSOLUTE_PATH [--machine]",
                ));
            }
        }
    }
    let binary = binary.ok_or_else(|| DevError::usage("pure-tail requires --binary"))?;
    let candidate_sha256 = digest(&binary)?;
    let output = output.ok_or_else(|| DevError::usage("pure-tail requires --evidence-root"))?;
    require(
        output.is_absolute()
            && !output
                .components()
                .any(|part| matches!(part, Component::CurDir | Component::ParentDir)),
        "evidence path must be absolute and canonical",
    )?;
    let parent = output
        .parent()
        .ok_or_else(|| DevError::usage("evidence parent missing"))?;
    require(
        parent.canonicalize()? == parent,
        "evidence parent must be a real canonical directory",
    )?;
    fs::create_dir(&output)?;
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&output, fs::Permissions::from_mode(0o700))?;
    let root = tempfile::Builder::new()
        .prefix("lkjscript-pure-tail-")
        .tempdir()?;
    let copied = root.path().join("lkjscript");
    fs::copy(&binary, &copied)?;
    let copied_candidate_sha256 = digest(&copied)?;
    require(
        candidate_sha256 == copied_candidate_sha256,
        "candidate copy changed",
    )?;
    let mut context = Context {
        root: root.path().to_path_buf(),
        output: output.clone(),
        binary: copied,
        started: Instant::now(),
        receipt: Receipt {
            schema: "lkjscript-pure-tail-acceptance-1".to_owned(),
            status: "failed".to_owned(),
            candidate_sha256,
            copied_candidate_sha256,
            verifier_sha256: digest(&std::env::current_exe()?)?,
            isolated_root: root.path().display().to_string(),
            evidence_root: output.display().to_string(),
            elapsed_nanoseconds: 0,
            peak_owned_file_bytes: 0,
            commands: Vec::new(),
            outcomes: BTreeMap::new(),
            files: Vec::new(),
            cleanup_complete: false,
            failure: None,
        },
    };
    let result = workflow(&mut context);
    context.receipt.elapsed_nanoseconds = u64::try_from(context.started.elapsed().as_nanos())
        .map_err(|_| DevError::corrupt("elapsed overflow"))?;
    root.close()?;
    context.receipt.cleanup_complete = !context.root.exists();
    match result {
        Ok(()) => context.receipt.status = "fresh passed".to_owned(),
        Err(error) => context.receipt.failure = Some(error.to_string()),
    }
    let mut files = fs::read_dir(&output)?.collect::<Result<Vec<_>, _>>()?;
    files.sort_by_key(fs::DirEntry::file_name);
    for file in files {
        context.receipt.files.push(evidence::proof(
            &file.path(),
            file.file_name().to_string_lossy().to_string(),
        )?);
    }
    let proof = evidence::publish_json(&output.join("receipt.json"), &context.receipt)?;
    let success = context.receipt.status == "fresh passed" && context.receipt.cleanup_complete;
    if success {
        read_transferred_receipt(
            &output.join("receipt.json"),
            &binary,
            &std::env::current_exe()?,
        )?;
    }
    println!(
        "{}",
        serde_json::json!({"status":if success {"passed"}else{"failed"},"classification":context.receipt.status,"receipt":proof.path,"digest":proof.digest,"failure":context.receipt.failure})
    );
    Ok(if success { 0 } else { 1 })
}

pub(crate) fn probe_command(mut arguments: impl Iterator<Item = OsString>) -> Result<u8, DevError> {
    let project = crate::next_utf8(&mut arguments, "project")?
        .ok_or_else(|| DevError::usage("pure-tail-probe requires a project path"))?;
    require(
        arguments.next().is_none(),
        "pure-tail-probe takes exactly one project path",
    )?;
    let observed = lkjscript::platform::contributor::pure_tail_execution_probe(Path::new(&project))
        .map_err(|error| DevError::corrupt(error.to_string()))?;
    println!("{}", serde_json::to_string(&observed)?);
    Ok(0)
}

pub(crate) fn transaction_probe_command(
    mut arguments: impl Iterator<Item = OsString>,
) -> Result<u8, DevError> {
    let path = crate::next_utf8(&mut arguments, "deployment")?
        .ok_or_else(|| DevError::usage("transaction probe requires deployment and exact helper"))?;
    let function = crate::next_utf8(&mut arguments, "function")?
        .ok_or_else(|| DevError::usage("transaction probe requires exact helper"))?;
    require(
        arguments.next().is_none(),
        "transaction probe takes two arguments",
    )?;
    let observed =
        lkjscript::platform::contributor::pure_tail_transaction_probe(Path::new(&path), &function)
            .map_err(|error| DevError::corrupt(error.to_string()))?;
    println!("{}", serde_json::to_string(&observed)?);
    Ok(0)
}

impl Context {
    fn bound(&mut self) -> Result<Duration, DevError> {
        let owned = directory_bytes(&self.root)?
            .checked_add(directory_bytes(&self.output)?)
            .ok_or_else(|| DevError::corrupt("file count overflow"))?;
        self.receipt.peak_owned_file_bytes = self.receipt.peak_owned_file_bytes.max(owned);
        require(
            owned <= MAXIMUM_FILES_BYTES,
            "pure-tail owned experiment files exceed 1073741824 bytes",
        )?;
        Duration::from_secs(MAXIMUM_SECONDS)
            .checked_sub(self.started.elapsed())
            .filter(|remaining| !remaining.is_zero())
            .ok_or_else(|| DevError::corrupt("pure-tail oracle exhausted 900 wall seconds"))
    }

    fn cli(
        &mut self,
        project: Option<&Path>,
        args: &[&str],
        expected_success: bool,
    ) -> Result<Vec<CompactRecord>, DevError> {
        let timeout = self.bound()?;
        let mut command = vec![self.binary.display().to_string()];
        if let Some(project) = project {
            command.extend(["--project".to_owned(), project.display().to_string()]);
        }
        command.extend(args.iter().map(|value| (*value).to_owned()));
        let index = self.receipt.commands.len();
        let spec = process::ProcessSpec {
            command,
            cwd: self.root.clone(),
            environment: BTreeMap::from([("LANG".to_owned(), "C.UTF-8".to_owned())]),
            timeout,
            maximum_stdout_bytes: MAXIMUM_OUTPUT,
            maximum_stderr_bytes: MAXIMUM_OUTPUT,
            stdout_path: self.output.join(format!("command-{index:04}.stdout")),
            stderr_path: self.output.join(format!("command-{index:04}.stderr")),
            unavailable_exit_code: None,
        };
        let observation = process::run(&spec, &self.output);
        let passed = observation.status == process::ProcessStatus::Passed;
        let failed = observation.status == process::ProcessStatus::Failed
            && observation.exit_code.is_some_and(|code| code != 0);
        self.receipt.commands.push(CommandEvidence {
            command: spec.command,
            observation,
            expected_success,
        });
        require(
            if expected_success { passed } else { failed },
            &format!(
                "public command {index} did not match expected success={expected_success}; see {}",
                spec.stdout_path.display()
            ),
        )?;
        self.bound()?;
        parse_records(
            "pure-tail copied response",
            &process::read_bounded(&spec.stdout_path, MAXIMUM_OUTPUT)?,
        )
        .map_err(|error| DevError::corrupt(format!("compact response: {error:?}")))
    }

    fn new_package(&mut self, name: &str) -> Result<Package, DevError> {
        let path = self.root.join(name);
        let records = self.cli(
            None,
            &[
                "new",
                &path.display().to_string(),
                "--template",
                "minimal",
                "--name",
                name,
            ],
            true,
        )?;
        Ok(Package {
            path,
            id: field(&records, "package", "id")?,
            revision: field(&records, "revision", "id")?,
            logical: String::new(),
            transport: String::new(),
            container: PathBuf::new(),
            symbols: BTreeMap::new(),
        })
    }

    fn stage(&mut self, target: &Package, dependency: &Package) -> Result<(), DevError> {
        let before = digest(&target.path.join("HEAD"))?;
        self.cli(
            Some(&target.path),
            &[
                "package",
                "dependency",
                "stage",
                "--transport",
                &dependency.transport,
                "--input-file",
                &dependency.container.display().to_string(),
            ],
            true,
        )?;
        require(
            before == digest(&target.path.join("HEAD"))?,
            "stage advanced meaning",
        )
    }

    fn apply(&mut self, package: &mut Package, changes: &str) -> Result<(), DevError> {
        let id = self.receipt.commands.len();
        let request = self.output.join(format!("request-{id}.lkjc"));
        fs::write(
            &request,
            format!(
                "request base={} idempotency=ptx-{id}\n{changes}",
                package.revision
            ),
        )?;
        let before = authority::observe_graph_authority(&package.path)?;
        let plan = self.output.join(format!("review-{id}.lkjplan"));
        let records = self.cli(
            Some(&package.path),
            &[
                "change",
                "plan",
                "--input-file",
                &request.display().to_string(),
                "--output",
                &plan.display().to_string(),
            ],
            true,
        )?;
        require(
            before == authority::observe_graph_authority(&package.path)?,
            "planning changed complete authority inventory",
        )?;
        let token = field(&records, "plan", "token")?;
        let decoded = decode_logical_change_plan(std::io::BufReader::new(fs::File::open(plan)?))
            .map_err(|error| DevError::corrupt(error.to_string()))?;
        require(decoded.token == token, "review did not bind apply token")?;
        let records = self.cli(
            Some(&package.path),
            &[
                "change",
                "apply",
                "--input-file",
                &request.display().to_string(),
                "--plan",
                &token,
            ],
            true,
        )?;
        package.revision = field(&records, "revision", "result")?;
        for record in records
            .iter()
            .filter(|record| record.operation == "identity")
        {
            package
                .symbols
                .insert(record_field(record, "symbol")?, record_field(record, "id")?);
        }
        Ok(())
    }

    fn run(
        &mut self,
        package: &Package,
        target: &str,
        arguments: &str,
        expected: &str,
    ) -> Result<(u64, u64), DevError> {
        let before = authority::observe_graph_authority(&package.path)?;
        let records = self.cli(
            Some(&package.path),
            &["run", target, "--arguments", arguments],
            true,
        )?;
        require(
            field(&records, "execution", "value")? == expected
                && field(&records, "execution", "differential")? == "equal",
            "fixed independent result mismatch",
        )?;
        require(
            before == authority::observe_graph_authority(&package.path)?,
            "pure run changed complete accepted inventory",
        )?;
        let production = integer_field(&records, "production-peak-call-frames")?;
        let reference = integer_field(&records, "reference-peak-call-frames")?;
        require(
            production <= 8 && reference <= 8,
            "tail chain retained more than eight live call frames",
        )?;
        let transfers = (
            integer_field(&records, "production-tail-transfers")?,
            integer_field(&records, "reference-tail-transfers")?,
        );
        self.receipt.outcomes.insert(format!("run-{}-{target}",self.receipt.commands.len()), serde_json::json!({"expected":expected,"production_peak_call_frames":production,"reference_peak_call_frames":reference,"production_tail_transfers":transfers.0,"reference_tail_transfers":transfers.1,"production_instructions":integer_field(&records,"production-instructions")?,"reference_expressions":integer_field(&records,"reference-expressions")?,"authority":before}));
        Ok((production, reference))
    }

    fn build(&mut self, package: &Package, name: &str) -> Result<String, DevError> {
        let artifact = self.output.join(format!("{name}.lkja"));
        self.cli(
            Some(&package.path),
            &["build", "--output", &artifact.display().to_string()],
            true,
        )?;
        digest(&artifact)
    }

    fn export(&mut self, package: &mut Package) -> Result<(), DevError> {
        package.container = self.root.join(format!("{}.lkjp", package.id));
        let records = self.cli(
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
        )?;
        package.logical = field(&records, "package", "package-revision")?;
        package.transport = field(&records, "package", "transport")?;
        Ok(())
    }

    fn probe(&mut self, package: &Package) -> Result<(), DevError> {
        let before = authority::observe_graph_authority(&package.path)?;
        let spec = process::ProcessSpec {
            command: vec![
                std::env::current_exe()?.display().to_string(),
                "pure-tail-probe".to_owned(),
                package.path.display().to_string(),
            ],
            cwd: self.root.clone(),
            environment: BTreeMap::from([("LANG".to_owned(), "C.UTF-8".to_owned())]),
            timeout: self.bound()?,
            maximum_stdout_bytes: MAXIMUM_OUTPUT,
            maximum_stderr_bytes: MAXIMUM_OUTPUT,
            stdout_path: self.output.join("bounded-stack.stdout"),
            stderr_path: self.output.join("bounded-stack.stderr"),
            unavailable_exit_code: None,
        };
        let observation = process::run(&spec, &self.output);
        let passed = observation.status == process::ProcessStatus::Passed;
        self.receipt.commands.push(CommandEvidence {
            command: spec.command,
            observation,
            expected_success: true,
        });
        require(
            passed,
            "bounded-stack subprocess failed; see bounded-stack.stderr",
        )?;
        let receipt: serde_json::Value =
            serde_json::from_slice(&process::read_bounded(&spec.stdout_path, MAXIMUM_OUTPUT)?)?;
        require(
            receipt["classification"] == "fresh passed"
                && receipt["stack_bytes"] == 2_097_152
                && receipt["cleanup_complete"] == true,
            "bounded-stack receipt incomplete",
        )?;
        require(
            before == authority::observe_graph_authority(&package.path)?,
            "bounded-stack probe changed authority",
        )?;
        self.receipt
            .outcomes
            .insert("bounded_stack".to_owned(), receipt);
        Ok(())
    }
}

fn workflow(context: &mut Context) -> Result<(), DevError> {
    let discovery = context.cli(None, &["capabilities", "--section", "runners"], true)?;
    require(
        discovery
            .iter()
            .any(|record| record.operation == "execution.tail"),
        "discovery omitted pure-tail guarantee",
    )?;
    let records = context.cli(None, &["package", "builtin", "inspect"], true)?;
    let mut standard = Package {
        path: PathBuf::new(),
        id: field(&records, "package", "id")?,
        revision: field(&records, "package", "revision")?,
        logical: field(&records, "package", "package-revision")?,
        transport: field(&records, "package", "transport")?,
        container: context.root.join("standard.lkjp"),
        symbols: BTreeMap::new(),
    };
    context.cli(
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
    )?;
    for name in ["add", "subtract", "divide", "i64-equal", "list-fold-left"] {
        let records = context.cli(
            None,
            &["package", "builtin", "query", "owners", "--name", name],
            true,
        )?;
        standard
            .symbols
            .insert(name.to_owned(), field(&records, "owner", "reference")?);
    }
    let mut library = context.new_package("producer")?;
    context.stage(&library, &standard)?;
    context.apply(
        &mut library,
        &format!(
            "{}{}",
            dependency(&standard),
            pure_tail_program::library(&standard.symbols)
        ),
    )?;
    let inventory = offline_producer_inventory(&library.path)
        .map_err(|error| DevError::corrupt(error.to_string()))?;
    evidence::publish_json(&context.output.join("producer-inventory.json"), &inventory)?;
    library.container = context.root.join("library.lkjp");
    let records = context.cli(
        Some(&library.path),
        &[
            "package",
            "current",
            "export",
            "--kind",
            "transport",
            "--output",
            &library.container.display().to_string(),
        ],
        true,
    )?;
    library.logical = field(&records, "package", "package-revision")?;
    library.transport = field(&records, "package", "transport")?;
    let keep = format!("{}/{}", library.id, library.symbols["$keep"]);
    let mut consumer = context.new_package("consumer")?;
    context.stage(&consumer, &library)?;
    context.stage(&consumer, &standard)?;
    context.apply(
        &mut consumer,
        &format!(
            "{}{}{}",
            dependency(&library),
            dependency(&standard),
            pure_tail_program::consumer(&standard.symbols, &keep)
        ),
    )?;
    context.cli(
        Some(&consumer.path),
        &[
            "package",
            "dependency",
            "inspect",
            "--package-revision",
            &library.logical,
        ],
        true,
    )?;
    let sum = consumer.symbols["$sum"].clone();
    context.cli(
        Some(&consumer.path),
        &[
            "inspect",
            "owner",
            "pure_function",
            &sum,
            "--detail",
            "definition",
            "--limit",
            "1000",
            "--bytes",
            "1048576",
        ],
        true,
    )?;
    context.cli(Some(&consumer.path), &["check"], true)?;
    let clean = context.build(&consumer, "clean")?;
    let exact = context.build(&consumer, "exact")?;
    require(clean == exact, "equal bound artifact inputs changed bytes")?;
    fs::remove_dir_all(&library.path)?;
    fs::remove_file(&library.container)?;
    fs::remove_file(&standard.container)?;
    let mut peak = None;
    for count in [0_i64, 1, 256, 4096, 8192] {
        let arguments = serde_json::to_string(&vec![(1..=count).collect::<Vec<_>>()])?;
        let observed = context.run(
            &consumer,
            "sum",
            &arguments,
            &(count * (count + 1) / 2).to_string(),
        )?;
        if count >= 256 {
            require(
                peak.is_none_or(|previous| previous == observed),
                "live frame peak grows with tail-chain length",
            )?;
            peak = Some(observed);
        }
    }
    for (target, args, expected) in [
        ("count", "[8192]", "0"),
        ("even", "[8192]", "true"),
        ("even", "[8191]", "false"),
        ("generic-i64", "[8192,-17,true]", "-17"),
        ("generic-bool", "[8192,true,-17]", "true"),
        ("ordered", "[[1,2,4,8]]", "5"),
        ("ordered", "[[8,4,2,1]]", "-5"),
    ] {
        context.run(&consumer, target, args, expected)?;
    }
    context.probe(&consumer)?;
    let current = consumer.path.join("derived/compiler/CURRENT");
    fs::remove_file(&current)?;
    require(
        context.build(&consumer, "missing-cache")? == clean,
        "missing cache changed artifact",
    )?;
    fs::write(&current, b"pure-tail corrupt disposable cache")?;
    require(
        context.build(&consumer, "corrupt-cache")? == clean,
        "corrupt cache changed artifact",
    )?;
    context.run(&consumer, "count", "[8192]", "0")?;
    let renamed = consumer.symbols["$ordered-step"].clone();
    context.apply(
        &mut consumer,
        &format!("rename.owner owner={renamed} name=ordered-step-reviewed\n"),
    )?;
    let incremental = context.build(&consumer, "reviewed-incremental")?;
    fs::remove_dir_all(consumer.path.join("derived/compiler"))?;
    require(
        context.build(&consumer, "reviewed-clean")? == incremental,
        "reviewed incremental and clean bytes disagree",
    )?;
    context.run(
        &consumer,
        "sum",
        &serde_json::to_string(&vec![(1_i64..=8192).collect::<Vec<_>>()])?,
        "33558528",
    )?;
    context.receipt.outcomes.insert(
        "reviewed_artifact_sha256".to_owned(),
        serde_json::json!(incremental),
    );
    for target in ["argument-order", "callee-order"] {
        let before = authority::observe_graph_authority(&consumer.path)?;
        let records = context.cli(
            Some(&consumer.path),
            &["run", target, "--arguments", "[]"],
            false,
        )?;
        require(
            before == authority::observe_graph_authority(&consumer.path)?
                && records.iter().all(|record| record.operation != "execution")
                && field(&records, "diagnostic", "code")? == "normalized_integer_division",
            "public early failure changed authority, evaluated later work, or emitted a success receipt",
        )?;
        context.receipt.outcomes.insert(
            format!("public-failure-{target}"),
            serde_json::json!({"code":field(&records,"diagnostic","code")?,"authority":before}),
        );
    }
    context
        .receipt
        .outcomes
        .insert("artifact_sha256".to_owned(), serde_json::json!(clean));
    context.receipt.outcomes.insert(
        "producer_removed".to_owned(),
        serde_json::json!(!library.path.exists()),
    );
    standalone_http(context, &mut consumer, &mut standard)?;
    context.bound()?;
    Ok(())
}

fn standalone_http(
    context: &mut Context,
    consumer: &mut Package,
    standard: &mut Package,
) -> Result<(), DevError> {
    use lkjscript::platform::data::{DataLimits, DataScanDirection, DataStore};
    context.export(consumer)?;
    canonical_corruption(context, consumer)?;
    for name in [
        "bytes-from-text",
        "json-decode-or",
        "json-encode",
        "DataKeyPart",
        "DataExpectation",
        "DataStore",
        "ByteStream",
    ] {
        let records = context.cli(
            None,
            &["package", "builtin", "query", "owners", "--name", name],
            true,
        )?;
        standard
            .symbols
            .insert(name.to_owned(), field(&records, "owner", "reference")?);
    }
    for (name, kind, member) in [
        ("DataKeyPart", "variant", "case"),
        ("DataExpectation", "variant", "case"),
        ("DataStore", "interface", "operation"),
        ("ByteStream", "interface", "operation"),
    ] {
        let owner = standard.symbols[name]
            .rsplit('/')
            .next()
            .ok_or_else(|| DevError::corrupt("builtin reference missing identity"))?
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
    let path = context.root.join("http");
    let created = context.cli(
        None,
        &[
            "new",
            &path.display().to_string(),
            "--template",
            "http",
            "--name",
            "pure-tail-http",
        ],
        true,
    )?;
    let mut http = Package {
        path,
        id: field(&created, "package", "id")?,
        revision: field(&created, "revision", "id")?,
        logical: String::new(),
        transport: String::new(),
        container: PathBuf::new(),
        symbols: BTreeMap::new(),
    };
    context.stage(&http, consumer)?;
    let module = context.cli(
        Some(&http.path),
        &["query", "find", "module", "application"],
        true,
    )?;
    let module = field(&module, "owner", "id")?;
    let function = context.cli(
        Some(&http.path),
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
        Some(&http.path),
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
        Some(&http.path),
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
    let bindings = BTreeMap::from([
        ("module".to_owned(), module),
        ("component".to_owned(), component),
        ("function".to_owned(), function),
        (
            "parameter".to_owned(),
            field(&definition, "definition.parameter", "id")?,
        ),
        (
            "result".to_owned(),
            field(&definition, "definition.function", "result")?,
        ),
        (
            "streams".to_owned(),
            record_field(
                definition
                    .iter()
                    .find(|record| {
                        record.operation == "definition.reference"
                            && record_field(record, "role")
                                .is_ok_and(|role| role == "function_requirement")
                    })
                    .ok_or_else(|| DevError::corrupt("handler requirement reference missing"))?,
                "target",
            )?,
        ),
    ]);
    let sum = format!("{}/{}", consumer.id, consumer.symbols["$sum"]);
    context.apply(
        &mut http,
        &format!(
            "{}{}",
            dependency(consumer),
            pure_tail_program::http(&standard.symbols, &bindings, &sum)
        ),
    )?;
    let inventory = offline_producer_inventory(&http.path)
        .map_err(|error| DevError::corrupt(error.to_string()))?;
    evidence::publish_json(&context.output.join("http-inventory.json"), &inventory)?;
    let before = authority::observe_graph_authority(&http.path)?;
    let before_files = authority_files(&http.path)?;
    evidence::publish_json(
        &context.output.join("http-build-before.json"),
        &authority_files(&http.path)?,
    )?;
    let standalone = context.root.join("standalone");
    fs::create_dir(&standalone)?;
    let artifact = standalone.join("application.lkja");
    context.cli(Some(&http.path), &["check"], true)?;
    let built = context.cli(
        Some(&http.path),
        &["build", "--output", &artifact.display().to_string()],
        true,
    )?;
    let after_files = authority_files(&http.path)?;
    evidence::publish_json(&context.output.join("http-build-after.json"), &after_files)?;
    require(
        before.head_sha256 == authority::observe_graph_authority(&http.path)?.head_sha256
            && inventory
                == offline_producer_inventory(&http.path)
                    .map_err(|error| DevError::corrupt(error.to_string()))?
            && before_files
                .iter()
                .all(|(path, digest)| after_files.get(path) == Some(digest)),
        "HTTP check/build changed accepted meaning or existing immutable inputs",
    )?;
    let bundle = field(&built, "artifact", "bundle")?;
    let artifact_sha256 = digest(&artifact)?;
    let descriptor_path = standalone.join("service.deployment.json");
    let mut descriptor: serde_json::Value = serde_json::from_slice(&process::read_bounded(
        &http.path.join("service.deployment.json"),
        MAXIMUM_OUTPUT,
    )?)?;
    descriptor["artifact"] = "application.lkja".into();
    descriptor["listen"] = "127.0.0.1:0".into();
    let limits = DataLimits {
        maximum_live_transactions: 1,
        ..Default::default()
    };
    descriptor["grants"].as_array_mut().ok_or_else(||DevError::corrupt("starter grants absent"))?.push(serde_json::json!({"requirement":"data","sharing_domain":"pure-tail-data","authority_revision":"7777777777777777777777777777777777777777777777777777777777777777","adapter":{"kind":"data","root":"data","namespace":"tail","limits":limits}}));
    fs::write(&descriptor_path, evidence::encode_json(&descriptor)?)?;
    fs::copy(&artifact, context.output.join("standalone.lkja"))?;
    fs::copy(
        &descriptor_path,
        context.output.join("standalone.deployment.json"),
    )?;
    let data_root = standalone.join("data");
    DataStore::initialize(&data_root).map_err(|error| DevError::corrupt(error.to_string()))?;
    fs::remove_dir_all(&http.path)?;
    fs::remove_dir_all(&consumer.path)?;
    fs::remove_file(&consumer.container)?;
    for entry in fs::read_dir(&context.root)? {
        let entry = entry?;
        require(
            !entry.file_type()?.is_dir() || entry.path() == standalone,
            "project directory remains before standalone HTTP",
        )?;
    }
    let spec = process::ProcessSpec {
        command: vec![
            context.binary.display().to_string(),
            "serve".to_owned(),
            "--deployment".to_owned(),
            descriptor_path.display().to_string(),
        ],
        cwd: standalone,
        environment: BTreeMap::from([("LANG".to_owned(), "C.UTF-8".to_owned())]),
        timeout: context.bound()?,
        maximum_stdout_bytes: MAXIMUM_OUTPUT,
        maximum_stderr_bytes: MAXIMUM_OUTPUT,
        stdout_path: context.output.join("standalone.stdout"),
        stderr_path: context.output.join("standalone.stderr"),
        unavailable_exit_code: None,
    };
    let command = spec.command.clone();
    let stdout = spec.stdout_path.clone();
    let control = process::ProcessControl::default();
    let child_control = control.clone();
    let output = context.output.clone();
    let (sender, receiver) = std::sync::mpsc::channel();
    let thread = std::thread::Builder::new()
        .name("pure-tail-http-observer".to_owned())
        .spawn(move || {
            let _ = sender.send(process::run_controlled(&spec, &output, &child_control));
        })?;
    let result: Result<(), DevError> = (|| {
        let started = Instant::now();
        let ready: serde_json::Value = loop {
            require(
                started.elapsed() < Duration::from_secs(30) && !thread.is_finished(),
                "standalone HTTP readiness failed",
            )?;
            if stdout.exists() {
                let bytes = process::read_bounded(&stdout, MAXIMUM_OUTPUT)?;
                if let Some(end) = bytes.iter().position(|byte| *byte == b'\n') {
                    break serde_json::from_slice(&bytes[..end])?;
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        require(
            ready["event"] == "ready"
                && ready["ok"] == true
                && ready["deployment"]["artifact_digest"] == bundle,
            "standalone readiness has a foreign artifact",
        )?;
        let address = ready["local_address"]
            .as_str()
            .ok_or_else(|| DevError::corrupt("listener missing"))?
            .parse()
            .map_err(|_| DevError::corrupt("listener invalid"))?;
        let store = DataStore::open(&data_root, "tail", DataLimits::default())
            .map_err(|error| DevError::corrupt(error.to_string()))?;
        let data_scan = || -> Result<_, DevError> {
            store
                .begin()
                .map_err(|error| DevError::corrupt(error.to_string()))?
                .scan(
                    "tail",
                    &[],
                    DataScanDirection::Forward,
                    10,
                    4096,
                    1000,
                    None,
                )
                .map_err(|error| DevError::corrupt(error.to_string()))
        };
        require(
            data_scan()?.items.is_empty(),
            "isolated data was not initially empty",
        )?;
        let values = (1_i64..=8192).collect::<Vec<_>>();
        let response = crate::http_probe::request(
            address,
            "GET",
            "/?success",
            &serde_json::to_vec(&values)?,
            &[],
        )?;
        require(
            response.status == 200 && response.body == b"33558528",
            "HTTP transaction did not return the fixed long-fold result",
        )?;
        let committed = data_scan()?;
        require(
            committed.items.len() == 1
                && committed.items[0].value == b"written"
                && committed.continuation.is_none(),
            "independent data read did not observe exactly one committed write",
        )?;
        let committed_head = digest(&data_root.join("HEAD"))?;
        let mut trapping = values.clone();
        if let Some(last) = trapping.last_mut() {
            *last = i64::MAX;
        }
        let response = crate::http_probe::request(
            address,
            "GET",
            "/?trapped",
            &serde_json::to_vec(&trapping)?,
            &[],
        )?;
        require(
            response.status == 500,
            "injected helper overflow did not fail HTTP execution",
        )?;
        require(
            data_scan()? == committed && digest(&data_root.join("HEAD"))? == committed_head,
            "helper trap exposed the staged write",
        )?;
        let response = crate::http_probe::request(
            address,
            "GET",
            "/?recovery",
            &serde_json::to_vec(&values)?,
            &[],
        )?;
        require(
            response.status == 200 && response.body == b"33558528" && data_scan()?.items.len() == 2,
            "transaction state leaked after helper failure",
        )?;
        context.receipt.outcomes.insert("standalone_http".to_owned(),serde_json::json!({"fixed_response":"33558528","initial_committed_changes":1,"after_trap_changes":1,"after_recovery_changes":2,"maximum_live_transactions":1,"project_directories_absent":true,"committed":committed,"artifact_sha256":artifact_sha256,"pure_effects_replayed":false,"trap":"integer overflow inside the final fold callback after staging"}));
        Ok(())
    })();
    control.interrupt();
    let terminal = receiver.recv_timeout(Duration::from_secs(35)).or_else(|_| {
        control.kill();
        receiver.recv_timeout(Duration::from_secs(5))
    });
    thread
        .join()
        .map_err(|_| DevError::corrupt("HTTP observer thread failed"))?;
    let terminal = terminal.map_err(|_| DevError::corrupt("HTTP runner did not terminate"))?;
    let stopped = terminal.status == process::ProcessStatus::Passed;
    context.receipt.commands.push(CommandEvidence {
        command,
        observation: terminal,
        expected_success: true,
    });
    result?;
    require(stopped, "HTTP runner did not shut down cleanly")?;
    let spec = process::ProcessSpec {
        command: vec![
            std::env::current_exe()?.display().to_string(),
            "pure-tail-transaction-probe".to_owned(),
            descriptor_path.display().to_string(),
            http.symbols["$write-fold"].clone(),
        ],
        cwd: context.root.clone(),
        environment: BTreeMap::from([("LANG".to_owned(), "C.UTF-8".to_owned())]),
        timeout: context.bound()?,
        maximum_stdout_bytes: MAXIMUM_OUTPUT,
        maximum_stderr_bytes: MAXIMUM_OUTPUT,
        stdout_path: context.output.join("transaction-cancellation.stdout"),
        stderr_path: context.output.join("transaction-cancellation.stderr"),
        unavailable_exit_code: None,
    };
    let observation = process::run(&spec, &context.output);
    let passed = observation.status == process::ProcessStatus::Passed;
    context.receipt.commands.push(CommandEvidence {
        command: spec.command,
        observation,
        expected_success: true,
    });
    require(
        passed,
        "transaction cancellation probe failed; see transaction-cancellation.stderr",
    )?;
    let cancellation: serde_json::Value =
        serde_json::from_slice(&process::read_bounded(&spec.stdout_path, MAXIMUM_OUTPUT)?)?;
    require(
        cancellation["classification"] == "fresh passed"
            && cancellation["failure"]["code"] == "execution_cancelled"
            && cancellation["cleanup_complete"] == true
            && cancellation["recovery_value"] == 33_558_528,
        "transaction cancellation or healthy recovery evidence missing",
    )?;
    let store = DataStore::open(&data_root, "tail", DataLimits::default())
        .map_err(|error| DevError::corrupt(error.to_string()))?;
    let after = store
        .begin()
        .map_err(|error| DevError::corrupt(error.to_string()))?
        .scan(
            "tail",
            &[],
            DataScanDirection::Forward,
            10,
            4096,
            1000,
            None,
        )
        .map_err(|error| DevError::corrupt(error.to_string()))?;
    require(
        after.items.len() == 3
            && after.items.iter().all(|item| {
                item.key.parts()
                    != [lkjscript::platform::data::DataKeyPart::Text(
                        "cancelled".to_owned(),
                    )]
            }),
        "cancelled staged write became durable or healthy recovery did not commit once",
    )?;
    context.receipt.outcomes.insert("transaction_cancellation".to_owned(), serde_json::json!({"execution":cancellation,"after":after,"cancelled_key_absent":true,"committed_changes":3}));
    require(
        digest(&artifact)? == artifact_sha256,
        "HTTP execution changed artifact",
    )
}

fn canonical_corruption(context: &mut Context, package: &Package) -> Result<(), DevError> {
    let inventory = offline_producer_inventory(&package.path)
        .map_err(|error| DevError::corrupt(error.to_string()))?;
    evidence::publish_json(&context.output.join("consumer-inventory.json"), &inventory)?;
    let owner = inventory
        .owners
        .iter()
        .find(|(owner, _, _)| owner == &package.symbols["$ordered-step"])
        .ok_or_else(|| DevError::corrupt("canonical corruption owner absent"))?;
    let body = lkjscript::platform::contributor::offline_package_owner_bytes(
        &process::read_bounded(&package.container, 268_435_456)?,
        &package.transport,
        &owner.2,
    )
    .map_err(|error| DevError::corrupt(error.to_string()))?;
    require(!body.is_empty(), "canonical corruption body is empty")?;
    let mut matches = Vec::new();
    for file in fs::read_dir(package.path.join("packs"))? {
        let file = file?;
        let bytes = process::read_bounded(&file.path(), 268_435_456)?;
        for (offset, window) in bytes.windows(body.len()).enumerate() {
            if window == body {
                matches.push((file.path(), offset));
            }
        }
    }
    require(
        matches.len() == 1,
        "canonical corruption did not resolve one exact immutable owner",
    )?;
    let (path, offset) = &matches[0];
    let original = process::read_bounded(path, 268_435_456)?;
    let mut corrupt = original.clone();
    corrupt[offset + body.len() - 1] ^= 1;
    let before = authority::observe_graph_authority(&package.path)?;
    fs::write(path, corrupt)?;
    let damaged = authority::observe_graph_authority(&package.path)?;
    let rejected = context.cli(Some(&package.path), &["check"], false);
    let after = authority::observe_graph_authority(&package.path);
    fs::write(path, original)?;
    require(
        after? == damaged && before == authority::observe_graph_authority(&package.path)?,
        "corrupt canonical source was repaired or authority changed on failure",
    )?;
    let rejected = rejected?;
    require(
        field(&rejected, "diagnostic", "code")? == "pack_entry_checksum",
        "canonical corruption was treated as cache recovery",
    )?;
    context.receipt.outcomes.insert("canonical_corruption".to_owned(),serde_json::json!({"code":"pack_entry_checksum","unchanged_on_failure":true,"restored_exact_owned_fixture":true,"authority":before}));
    Ok(())
}

fn dependency(package: &Package) -> String {
    format!(
        "add.dependency package={} semantic-revision={} package-revision={}\n",
        package.id, package.revision, package.logical
    )
}

fn authority_files(root: &Path) -> Result<BTreeMap<String, String>, DevError> {
    let mut pending = vec![root.join("packs"), root.join("PACKAGE-TRANSPORTS")];
    let mut files = BTreeMap::from([("HEAD".to_owned(), digest(&root.join("HEAD"))?)]);
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                pending.push(entry.path());
            } else {
                files.insert(
                    entry
                        .path()
                        .strip_prefix(root)
                        .map_err(|_| DevError::corrupt("authority path escaped root"))?
                        .display()
                        .to_string(),
                    digest(&entry.path())?,
                );
            }
        }
    }
    Ok(files)
}

fn record_field(record: &CompactRecord, name: &str) -> Result<String, DevError> {
    record
        .fields
        .iter()
        .find(|field| field.name == name)
        .map(|field| field.value.clone())
        .ok_or_else(|| DevError::corrupt(format!("missing {}.{name}", record.operation)))
}

fn field(records: &[CompactRecord], operation: &str, name: &str) -> Result<String, DevError> {
    let record = records
        .iter()
        .find(|record| record.operation == operation)
        .ok_or_else(|| DevError::corrupt(format!("missing {operation}")))?;
    record_field(record, name)
}

fn integer_field(records: &[CompactRecord], name: &str) -> Result<u64, DevError> {
    field(records, "execution", name)?
        .parse()
        .map_err(|_| DevError::corrupt("execution observation is not an unsigned integer"))
}

fn require(condition: bool, message: &str) -> Result<(), DevError> {
    if condition {
        Ok(())
    } else {
        Err(DevError::corrupt(message))
    }
}

fn digest(path: &Path) -> Result<String, DevError> {
    let metadata = fs::symlink_metadata(path)?;
    require(
        metadata.is_file() && !metadata.file_type().is_symlink(),
        "oracle input is not a regular file",
    )?;
    Ok(
        Sha256::digest(process::read_bounded(path, 384 * 1024 * 1024)?)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    )
}

fn directory_bytes(root: &Path) -> Result<u64, DevError> {
    let mut pending = vec![root.to_path_buf()];
    let mut bytes = 0_u64;
    let mut count = 0_u64;
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let metadata = fs::symlink_metadata(entry.path())?;
            count += 1;
            require(
                count <= 100_000 && !metadata.file_type().is_symlink(),
                "oracle inventory has excessive files or a symlink",
            )?;
            if metadata.is_dir() {
                pending.push(entry.path());
            } else {
                require(
                    metadata.is_file(),
                    "oracle inventory contains a nonregular file",
                )?;
                bytes = bytes
                    .checked_add(metadata.len())
                    .ok_or_else(|| DevError::corrupt("oracle byte count overflow"))?;
                require(
                    bytes <= MAXIMUM_FILES_BYTES,
                    "oracle files exhausted byte budget",
                )?;
            }
        }
    }
    Ok(bytes)
}

impl Receipt {
    pub(crate) fn command_count(&self) -> u64 {
        self.commands.len() as u64
    }
}

pub(crate) fn read_transferred_receipt(
    path: &Path,
    candidate: &Path,
    verifier: &Path,
) -> Result<Receipt, DevError> {
    let bytes = process::read_bounded(path, 16 * 1024 * 1024)?;
    let receipt: Receipt = serde_json::from_slice(&bytes)?;
    require(
        evidence::encode_json(&receipt)? == bytes,
        "pure-tail receipt is noncanonical",
    )?;
    let root = path
        .parent()
        .ok_or_else(|| DevError::corrupt("receipt parent missing"))?
        .canonicalize()?;
    require(
        receipt.schema == "lkjscript-pure-tail-acceptance-1"
            && receipt.status == "fresh passed"
            && receipt.failure.is_none()
            && receipt.cleanup_complete
            && !Path::new(&receipt.isolated_root).exists(),
        "pure-tail receipt is incomplete",
    )?;
    require(
        receipt.candidate_sha256 == digest(candidate)?
            && receipt.copied_candidate_sha256 == receipt.candidate_sha256
            && receipt.verifier_sha256 == digest(verifier)?,
        "pure-tail receipt has foreign executable bindings",
    )?;
    require(
        receipt.elapsed_nanoseconds <= MAXIMUM_SECONDS * 1_000_000_000
            && receipt.peak_owned_file_bytes <= MAXIMUM_FILES_BYTES,
        "pure-tail receipt exceeded experiment bounds",
    )?;
    require(
        receipt.commands.len() >= 50 && receipt.commands.len() <= 200,
        "pure-tail command inventory is incomplete or excessive",
    )?;
    for command in &receipt.commands {
        require(
            if command.expected_success {
                command.observation.status == process::ProcessStatus::Passed
            } else {
                command.observation.status == process::ProcessStatus::Failed
            },
            "pure-tail command evidence did not pass its specified boundary",
        )?;
    }
    for expected in [
        "0", "1", "32896", "8390656", "33558528", "true", "false", "-17", "5", "-5",
    ] {
        require(
            receipt.outcomes.values().any(|observed| {
                observed["expected"] == expected
                    && observed["production_peak_call_frames"]
                        .as_u64()
                        .is_some_and(|peak| peak <= 8)
                    && observed["reference_peak_call_frames"]
                        .as_u64()
                        .is_some_and(|peak| peak <= 8)
            }),
            "pure-tail fixed public result or frame boundary missing",
        )?;
    }
    let stack = receipt
        .outcomes
        .get("bounded_stack")
        .ok_or_else(|| DevError::corrupt("bounded-stack evidence missing"))?;
    require(
        stack["classification"] == "fresh passed"
            && stack["stack_bytes"] == 2_097_152
            && stack["call_frame_limit"] == 8
            && stack["cleanup_complete"] == true,
        "bounded-stack admission missing",
    )?;
    let cases = stack["cases"]
        .as_array()
        .ok_or_else(|| DevError::corrupt("bounded-stack cases absent"))?;
    for case in cases {
        let observation = &case["observation"];
        require(
            observation["maximum_call_depth"]
                .as_u64()
                .is_some_and(|peak| peak <= 8)
                && observation["maximum_control_frames"]
                    .as_u64()
                    .is_some_and(|peak| peak > 0)
                && observation["maximum_live_locals"].as_u64().is_some()
                && observation["maximum_live_type_bindings"].as_u64().is_some()
                && observation["live_call_frames_after"] == 0
                && observation["live_transactions_after"] == 0,
            "actual control/local ownership or cleanup observation is missing",
        )?;
    }
    for code in [
        "normalized_call_depth",
        "normalized_reference_call_depth",
        "normalized_instruction_steps",
        "normalized_reference_expression_steps",
        "execution_cancelled",
        "normalized_allocation",
        "normalized_reference_allocation",
        "normalized_value_stack",
        "reference_integer_division",
        "normalized_integer_division",
    ] {
        require(
            cases.iter().any(|case| case["failure"]["code"] == code),
            "resource/order failure discrimination missing",
        )?;
    }
    require(
        cases
            .iter()
            .any(|case| case["fault"] == "forced-ordinary-frame-growth-detected")
            && cases
                .iter()
                .any(|case| case["fault"] == "canonical-reference-independent-of-dispatch"),
        "independent fault sensitivity missing",
    )?;
    let http = receipt
        .outcomes
        .get("standalone_http")
        .ok_or_else(|| DevError::corrupt("standalone HTTP evidence missing"))?;
    require(
        http["fixed_response"] == "33558528"
            && http["initial_committed_changes"] == 1
            && http["after_trap_changes"] == 1
            && http["after_recovery_changes"] == 2
            && http["project_directories_absent"] == true,
        "standalone transaction boundary incomplete",
    )?;
    let cancellation = receipt
        .outcomes
        .get("transaction_cancellation")
        .ok_or_else(|| DevError::corrupt("transaction cancellation evidence missing"))?;
    require(
        cancellation["execution"]["failure"]["code"] == "execution_cancelled"
            && cancellation["execution"]["cleanup_complete"] == true
            && cancellation["execution"]["observation"]["maximum_live_transactions"] == 1
            && cancellation["cancelled_key_absent"] == true
            && cancellation["committed_changes"] == 3,
        "transaction cancellation boundary incomplete",
    )?;
    require(
        receipt
            .outcomes
            .get("canonical_corruption")
            .is_some_and(|outcome| {
                outcome["code"] == "pack_entry_checksum" && outcome["unchanged_on_failure"] == true
            })
            && receipt
                .outcomes
                .get("reviewed_artifact_sha256")
                .is_some_and(|outcome| outcome.as_str().is_some_and(|digest| digest.len() == 64)),
        "canonical corruption or reviewed incremental proof missing",
    )?;
    for file in &receipt.files {
        let relative = Path::new(&file.path);
        require(
            relative.components().count() == 1
                && matches!(relative.components().next(), Some(Component::Normal(_))),
            "pure-tail evidence path escaped root",
        )?;
        require(
            evidence::proof(&root.join(relative), file.path.clone())? == *file,
            "pure-tail evidence file changed",
        )?;
    }
    Ok(receipt)
}
