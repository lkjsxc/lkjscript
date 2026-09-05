//! Transferable, public-authored offline composition oracle. No producer API writes meaning.

use crate::{error::DevError, evidence, process};
use lkjscript::platform::contributor::{
    OfflinePackageInventory, OfflineProducerInventory, offline_package_inventory,
    offline_package_owner_bytes, offline_producer_inventory,
};
use lkjscript::platform::control::{CompactRecord, decode_logical_change_plan, parse_records};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};

const MAXIMUM_CONTAINER_BYTES: u64 = 268_435_456;
const MAXIMUM_EXECUTABLE_BYTES: u64 = 384 * 1024 * 1024;
const MAXIMUM_OUTPUT_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Receipt {
    pub schema: String,
    pub status: String,
    pub candidate_sha256: String,
    pub verifier_sha256: String,
    pub copied_candidate_sha256: String,
    pub isolated_root: String,
    pub evidence_root: String,
    pub environment_names: Vec<String>,
    pub elapsed_nanoseconds: u64,
    pub commands: Vec<CommandEvidence>,
    pub runners: Vec<process::ProcessObservation>,
    pub inventories: Vec<OfflinePackageInventory>,
    pub producer_inventories: Vec<OfflineProducerInventory>,
    pub transport_digests: Vec<String>,
    pub observations: BTreeMap<String, String>,
    pub files: Vec<evidence::FileProof>,
    pub cleanup_complete: bool,
    pub failure: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CommandEvidence {
    command: Vec<String>,
    cwd: String,
    expects_success: bool,
    observation: process::ProcessObservation,
}

struct Context {
    root: PathBuf,
    evidence: PathBuf,
    binary: PathBuf,
    receipt: Receipt,
}

#[derive(Clone)]
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
                        .ok_or_else(|| DevError::usage("missing --binary path"))?,
                ))
            }
            "--evidence-root" if output.is_none() => {
                output = Some(PathBuf::from(
                    crate::next_utf8(&mut arguments, "evidence root")?
                        .ok_or_else(|| DevError::usage("missing evidence path"))?,
                ))
            }
            "--machine" if !machine => machine = true,
            _ => {
                return Err(DevError::usage(format!(
                    "unknown or duplicate offline-packages option {option}"
                )));
            }
        }
    }
    let requested_binary = binary.unwrap_or_else(|| PathBuf::from("target/release/lkjscript"));
    let candidate_sha256 = digest_file(&requested_binary, MAXIMUM_EXECUTABLE_BYTES)?;
    let binary = requested_binary.canonicalize()?;
    let verifier_sha256 = digest_file(&std::env::current_exe()?, MAXIMUM_EXECUTABLE_BYTES)?;
    let output = match output {
        Some(path) => create_evidence_root(&path)?,
        None => {
            let parent = std::env::current_dir()?.join(".artifacts/offline-packages");
            fs::create_dir_all(&parent)?;
            tempfile::Builder::new()
                .prefix("run-")
                .tempdir_in(parent.canonicalize()?)?
                .keep()
        }
    };
    let isolated = tempfile::Builder::new()
        .prefix("lkjscript-offline-packages-")
        .tempdir()?;
    let copied = isolated.path().join("lkjscript");
    fs::copy(&binary, &copied)?;
    require(
        digest_file(&copied, MAXIMUM_EXECUTABLE_BYTES)? == candidate_sha256,
        "copied candidate bytes changed",
    )?;
    let started = Instant::now();
    let mut context = Context {
        root: isolated.path().to_path_buf(),
        evidence: output.clone(),
        binary: copied,
        receipt: Receipt {
            schema: "lkjscript-offline-packages-acceptance-1".to_owned(),
            status: "failed".to_owned(),
            copied_candidate_sha256: candidate_sha256.clone(),
            candidate_sha256,
            verifier_sha256,
            isolated_root: isolated.path().display().to_string(),
            evidence_root: output.display().to_string(),
            environment_names: vec!["LANG".to_owned()],
            elapsed_nanoseconds: 0,
            commands: Vec::new(),
            runners: Vec::new(),
            inventories: Vec::new(),
            producer_inventories: Vec::new(),
            transport_digests: Vec::new(),
            observations: BTreeMap::new(),
            files: Vec::new(),
            cleanup_complete: false,
            failure: None,
        },
    };
    let outcome = workflow(&mut context);
    context.receipt.elapsed_nanoseconds = u64::try_from(started.elapsed().as_nanos())
        .map_err(|_| DevError::infrastructure("elapsed time overflow"))?;
    isolated.close()?;
    context.receipt.cleanup_complete = !context.root.exists();
    match outcome {
        Ok(()) => context.receipt.status = "fresh passed".to_owned(),
        Err(error) => context.receipt.failure = Some(error.to_string()),
    }
    let mut files = fs::read_dir(&output)?.collect::<Result<Vec<_>, _>>()?;
    files.sort_by_key(fs::DirEntry::file_name);
    for entry in files {
        let label = entry
            .file_name()
            .into_string()
            .map_err(|_| DevError::corrupt("non-UTF-8 evidence filename"))?;
        context
            .receipt
            .files
            .push(evidence::proof(&entry.path(), label)?);
    }
    let receipt = evidence::publish_json(&output.join("receipt.json"), &context.receipt)?;
    let success = context.receipt.status == "fresh passed" && context.receipt.cleanup_complete;
    if success {
        read_transferred_receipt(&receipt.path, &binary, &std::env::current_exe()?)?;
    }
    if machine {
        println!(
            "{}",
            serde_json::json!({"status": if success { "passed" } else { "failed" }, "classification": context.receipt.status, "receipt": receipt.path, "digest": receipt.digest, "failure": context.receipt.failure})
        );
    } else {
        println!(
            "offline packages: {} ({})",
            context.receipt.status,
            receipt.path.display()
        );
    }
    Ok(if success { 0 } else { 1 })
}

impl Context {
    fn cli(
        &mut self,
        project: Option<&Path>,
        arguments: &[&str],
        passes: bool,
    ) -> Result<Vec<CompactRecord>, DevError> {
        let index = self.receipt.commands.len();
        let mut command = vec![self.binary.display().to_string()];
        if let Some(project) = project {
            command.extend(["--project".to_owned(), project.display().to_string()]);
        }
        command.extend(arguments.iter().map(|value| (*value).to_owned()));
        let spec = process::ProcessSpec {
            command,
            cwd: self.root.clone(),
            environment: BTreeMap::from([("LANG".to_owned(), "C.UTF-8".to_owned())]),
            timeout: Duration::from_secs(90),
            maximum_stdout_bytes: MAXIMUM_OUTPUT_BYTES,
            maximum_stderr_bytes: MAXIMUM_OUTPUT_BYTES,
            stdout_path: self.evidence.join(format!("command-{index:04}.stdout")),
            stderr_path: self.evidence.join(format!("command-{index:04}.stderr")),
            unavailable_exit_code: None,
        };
        let observation = process::run(&spec, &self.evidence);
        let success = observation.status == process::ProcessStatus::Passed;
        let expected_failure = observation.status == process::ProcessStatus::Failed
            && observation.exit_code.is_some_and(|code| code != 0);
        self.receipt.commands.push(CommandEvidence {
            command: spec.command.clone(),
            cwd: self.root.display().to_string(),
            expects_success: passes,
            observation,
        });
        let stdout = process::read_bounded(&spec.stdout_path, MAXIMUM_OUTPUT_BYTES)?;
        if success != passes || (!passes && !expected_failure) {
            let stderr = process::read_bounded(&spec.stderr_path, MAXIMUM_OUTPUT_BYTES)?;
            return Err(DevError::corrupt(format!(
                "command {index} {}: expected success={passes}; stdout={} stderr={}: {}",
                arguments.join(" "),
                spec.stdout_path.display(),
                spec.stderr_path.display(),
                String::from_utf8_lossy(&stderr)
            )));
        }
        let records = parse_records("copied-output", &stdout)
            .map_err(|errors| DevError::corrupt(format!("public compact output: {errors:?}")))?;
        if !passes {
            require(
                records
                    .iter()
                    .any(|record| record.operation == "diagnostic"),
                "expected rejection omitted its diagnostic",
            )?;
        }
        Ok(records)
    }

    fn new_package(&mut self, directory: &str) -> Result<Package, DevError> {
        let path = self.root.join(directory);
        // Equal display names deliberately do not select identity.
        let result = self.cli(
            None,
            &[
                "new",
                &path.display().to_string(),
                "--template",
                "minimal",
                "--name",
                "same-name",
            ],
            true,
        )?;
        Ok(Package {
            path,
            id: field(&result, "package", "id")?,
            revision: field(&result, "revision", "id")?,
            logical: String::new(),
            transport: String::new(),
            container: PathBuf::new(),
            symbols: BTreeMap::new(),
        })
    }

    fn apply(
        &mut self,
        package: &mut Package,
        changes: &str,
    ) -> Result<Vec<CompactRecord>, DevError> {
        let request = self
            .evidence
            .join(format!("request-{}.lkjc", self.receipt.commands.len()));
        fs::write(
            &request,
            format!(
                "request base={} idempotency=offline-{}\n{changes}",
                package.revision,
                self.receipt.commands.len()
            ),
        )?;
        let path = request.display().to_string();
        let review = self
            .evidence
            .join(format!("review-{}.lkjplan", self.receipt.commands.len()));
        let plan = self.cli(
            Some(&package.path),
            &[
                "change",
                "plan",
                "--input-file",
                &path,
                "--output",
                &review.display().to_string(),
            ],
            true,
        )?;
        let token = field(&plan, "plan", "token")?;
        let decoded = decode_logical_change_plan(std::io::BufReader::new(fs::File::open(&review)?))
            .map_err(|error| DevError::corrupt(error.to_string()))?;
        require(
            decoded.token == token,
            "review file is not bound by the apply token",
        )?;
        if plan
            .iter()
            .any(|record| record.operation == "package-closure")
        {
            require(
                field(&plan, "package-closure", "before")?
                    == decoded.counts.package_befores.to_string()
                    && field(&plan, "package-closure", "after")?
                        == decoded.counts.package_afters.to_string(),
                "bounded response and complete reviewed closure disagree",
            )?;
        }
        let applied = self.cli(
            Some(&package.path),
            &["change", "apply", "--input-file", &path, "--plan", &token],
            true,
        )?;
        package.revision = field(&applied, "revision", "result")?;
        for record in applied
            .iter()
            .filter(|record| record.operation == "identity")
        {
            package
                .symbols
                .insert(record_field(record, "symbol")?, record_field(record, "id")?);
        }
        Ok(applied)
    }

    fn export(&mut self, package: &mut Package) -> Result<OfflinePackageInventory, DevError> {
        let producer = offline_producer_inventory(&package.path)
            .map_err(|error| DevError::corrupt(error.to_string()))?;
        require(
            producer.package == package.id && producer.semantic_revision == package.revision,
            "producer inventory disagrees with public accepted identity",
        )?;
        package.container = self
            .root
            .join(format!("transport-{}.lkjp", self.receipt.commands.len()));
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
        let bytes = process::read_bounded(&package.container, MAXIMUM_CONTAINER_BYTES)?;
        let inventory = offline_package_inventory(&bytes, &package.transport)
            .map_err(|error| DevError::corrupt(error.to_string()))?;
        verify_producer_inventory(&producer, &inventory)?;
        self.receipt.producer_inventories.push(producer);
        self.receipt.inventories.push(inventory.clone());
        self.receipt
            .transport_digests
            .push(package.transport.clone());
        fs::copy(
            &package.container,
            self.evidence
                .join(format!("transport-{}.lkjp", self.receipt.inventories.len())),
        )?;
        Ok(inventory)
    }

    fn stage(&mut self, target: &Package, source: &Package) -> Result<(), DevError> {
        let before = digest_file(&target.path.join("HEAD"), 1024 * 1024)?;
        self.cli(
            Some(&target.path),
            &[
                "package",
                "dependency",
                "stage",
                "--transport",
                &source.transport,
                "--input-file",
                &source.container.display().to_string(),
            ],
            true,
        )?;
        require(
            digest_file(&target.path.join("HEAD"), 1024 * 1024)? == before,
            "staging advanced semantic HEAD",
        )
    }

    fn run(&mut self, target: &Package, expected: i64) -> Result<(), DevError> {
        let records = self.cli(
            Some(&target.path),
            &["run", "main", "--arguments", "[10]"],
            true,
        )?;
        require(
            field(&records, "execution", "value")? == expected.to_string(),
            "fixed arithmetic result disagrees",
        )?;
        require(
            field(&records, "execution", "differential")? == "equal",
            "production/reference disagreement",
        )
    }

    fn check(&mut self, target: &Package) -> Result<(), DevError> {
        let records = self.cli(Some(&target.path), &["check"], true)?;
        require(
            field(&records, "tests", "passed")? == "24"
                && field(&records, "tests", "failed")? == "0"
                && field(&records, "tests", "differential")? == "equal",
            "each of five selected packages must be tested exactly once (20+1+1+1+1)",
        )?;
        require(
            field(&records, "artifact", "packages")? == "5",
            "compiled diamond lost or duplicated a selected package",
        )
    }

    fn reject(&mut self, target: &Package, arguments: &[&str], code: &str) -> Result<(), DevError> {
        let before = crate::authority::observe_graph_authority(&target.path)?;
        let records = self.cli(Some(&target.path), arguments, false)?;
        require(
            records
                .iter()
                .filter(|record| record.operation == "diagnostic")
                .any(|record| record_field(record, "code").is_ok_and(|observed| observed == code)),
            &format!("expected exact rejection {code}"),
        )?;
        require(
            crate::authority::observe_graph_authority(&target.path)? == before,
            "rejected work changed complete accepted or ready inventory",
        )?;
        self.receipt
            .observations
            .insert(format!("rejection-{code}"), before.inventory_sha256);
        Ok(())
    }

    fn forbidden_reference(
        &mut self,
        target: &Package,
        foreign: &str,
        code: &str,
    ) -> Result<(), DevError> {
        let request = self
            .evidence
            .join(format!("forbidden-{}.lkjc", self.receipt.commands.len()));
        fs::write(
            &request,
            format!(
                "request base={}\nexpression.local as=$bad_arg value={}\n{}replace.body function={} body=$bad_body\n",
                target.revision,
                target.symbols["$x"],
                call("$bad_body", foreign, &["$bad_arg"]),
                target.symbols["$entry"]
            ),
        )?;
        self.reject(
            target,
            &[
                "change",
                "plan",
                "--input-file",
                &request.display().to_string(),
            ],
            code,
        )
    }

    fn build(&mut self, target: &Package, label: &str) -> Result<String, DevError> {
        let output = self.evidence.join(format!("{label}.lkja"));
        self.cli(
            Some(&target.path),
            &["build", "--output", &output.display().to_string()],
            true,
        )?;
        let digest = digest_file(&output, MAXIMUM_CONTAINER_BYTES)?;
        self.receipt
            .observations
            .insert(format!("artifact-{label}"), digest.clone());
        Ok(digest)
    }

    fn cache_recovery(&mut self, target: &Package, label: &str) -> Result<(), DevError> {
        let head = digest_file(&target.path.join("HEAD"), 1024 * 1024)?;
        let exact = self.build(target, &format!("{label}-exact"))?;
        let current = target.path.join("derived/compiler/CURRENT");
        let saved = process::read_bounded(&current, 1024 * 1024)?;
        fs::remove_file(&current)?;
        let clean = self.build(target, &format!("{label}-missing-cache"))?;
        require(
            clean == exact,
            "missing-cache clean compilation changed artifact bytes",
        )?;
        fs::write(&current, b"offline package invalid disposable cache")?;
        let recovery = self.build(target, &format!("{label}-corrupt-cache"));
        fs::write(&current, saved)?;
        require(
            recovery? == exact,
            "corrupt-cache recovery changed artifact bytes",
        )?;
        require(
            digest_file(&target.path.join("HEAD"), 1024 * 1024)? == head,
            "cache recovery advanced semantic HEAD",
        )
    }
}

fn workflow(context: &mut Context) -> Result<(), DevError> {
    context.cli(None, &["capabilities"], true)?;
    let builtin = context.cli(None, &["package", "builtin", "inspect"], true)?;
    let mut standard = Package {
        path: PathBuf::new(),
        id: field(&builtin, "package", "id")?,
        revision: field(&builtin, "package", "revision")?,
        logical: field(&builtin, "package", "package-revision")?,
        transport: field(&builtin, "package", "transport")?,
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
    for name in ["add", "multiply", "subtract"] {
        let owners = context.cli(
            None,
            &[
                "package", "builtin", "query", "owners", "--kind", "external", "--name", name,
            ],
            true,
        )?;
        standard
            .symbols
            .insert(name.to_owned(), field(&owners, "owner", "reference")?);
    }
    let mut d = context.new_package("producer-d")?;
    context.stage(&d, &standard)?;
    context.apply(
        &mut d,
        &format!(
            "{}{}{}{}{}{}",
            binding("add", &standard),
            module(),
            unary("$helper", "helper", "private", "$sum", "$hx"),
            "expression.local as=$harg value=$hx\nexpression.i64 as=$one value=1\n",
            call("$sum", &standard.symbols["add"], &["$harg", "$one"]),
            format_args!(
                "{}{}expression.local as=$arg value=$x\n{}",
                unary("$entry", "offset", "public", "$body", "$x"),
                call("$body", "$helper", &["$arg"]),
                graph_test()
            )
        ),
    )?;
    let nominal_generic = "create.record as=$box module=$module name=Box visibility=public\nadd.field as=$field record=$box name=value type=i64\ntype.parameter as=@item parameter=$item_type\ncreate.function as=$identity module=$module name=identity visibility=public result=@item effect=pure body=$identity_body\nadd.type-parameter as=$item_type function=$identity name=Item\nadd.parameter as=$identity_arg function=$identity name=item type=@item\nexpression.local as=$identity_body value=$identity_arg\n".replace("module=$module", &format!("module={}", d.symbols["$module"]));
    context.apply(&mut d, &nominal_generic)?;
    let http_body = format!(
        "create.function as=$http_body module={} name=wire-body visibility=public result=static-text effect=pure body=$wire\nexpression.static-text as=$wire value=offline-package-closure\n",
        d.symbols["$module"]
    );
    context.apply(&mut d, &http_body)?;
    let d1_inventory = context.export(&mut d)?;
    let d1 = d.clone();
    let mut b = context.new_package("producer-b")?;
    let mut c = context.new_package("producer-c")?;
    for (package, operator, constant, name) in [
        (&mut b, "multiply", 2, "twice"),
        (&mut c, "subtract", 0, "negative"),
    ] {
        context.stage(package, &d)?;
        let (crossing, argument) = if operator == "multiply" {
            (
                format!(
                    "type.named as=@box declaration={}\nexpression.record as=$boxed type={}\nexpression.record-field parent=$boxed index=0 field={} value=$arg\n{}type.argument parent=$identity_call index=0 type=@box\nexpression.field as=$unboxed value=$identity_call field={}\n",
                    reference(&d, "$box")?,
                    reference(&d, "$box")?,
                    reference(&d, "$field")?,
                    call("$identity_call", &reference(&d, "$identity")?, &["$boxed"]),
                    reference(&d, "$field")?
                ),
                "$unboxed",
            )
        } else {
            (String::new(), "$arg")
        };
        context.apply(package, &format!("{}{}{}{}expression.local as=$arg value=$x\nexpression.i64 as=$factor value={constant}\n{crossing}{}{}{}",
            binding("add", &standard), binding("add", &d), module(), unary("$entry", name, "public", "$body", "$x"),
            call("$offset", &reference(&d, "$entry")?, &[argument]), call("$body", &standard.symbols[operator], &["$factor", "$offset"]), graph_test()))?;
        context.export(package)?;
    }
    let b1 = b.clone();
    let c1 = c.clone();
    context.forbidden_reference(
        &b,
        &reference(&d, "$helper")?,
        "kernel_type_dependency_owner_missing",
    )?;
    let mut a = context.new_package("consumer-a")?;
    context.stage(&a, &b)?;
    context.stage(&a, &c)?;
    let symlink_input = context.root.join("symlink-source.lkjp");
    std::os::unix::fs::symlink(&b.container, &symlink_input)?;
    context.reject(
        &a,
        &[
            "package",
            "dependency",
            "stage",
            "--transport",
            &b.transport,
            "--input-file",
            &symlink_input.display().to_string(),
        ],
        "read_type",
    )?;
    fs::remove_file(&symlink_input)?;
    let imported = context.cli(
        Some(&a.path),
        &[
            "package",
            "dependency",
            "query",
            "owners",
            "--package-revision",
            &d.logical,
            "--kind",
            "pure_function",
            "--name",
            "offset",
        ],
        true,
    )?;
    require(
        field(&imported, "owner", "reference")? == reference(&d, "$entry")?,
        "imported signature discovery selected a foreign declaration",
    )?;
    let inspected = context.cli(
        Some(&a.path),
        &[
            "package",
            "dependency",
            "inspect",
            "--package-revision",
            &d.logical,
        ],
        true,
    )?;
    require(
        field(&inspected, "closure", "packages")? == "2"
            && field(&inspected, "closure", "edges")? == "1"
            && field(&inspected, "dependency", "package")? == standard.id,
        "staged root inspection did not expose the exact bounded transitive selection",
    )?;
    context.cli(
        Some(&a.path),
        &[
            "package",
            "dependency",
            "inspect",
            "owner",
            "pure_function",
            &d.symbols["$entry"],
            "--package-revision",
            &d.logical,
        ],
        true,
    )?;
    let first_page = context.cli(
        Some(&a.path),
        &[
            "package",
            "dependency",
            "query",
            "owners",
            "--package-revision",
            &d.logical,
            "--limit",
            "1",
            "--bytes",
            "4096",
        ],
        true,
    )?;
    let d1_continuation = field(&first_page, "continuation", "token")?;
    let second_page = context.cli(
        Some(&a.path),
        &[
            "package",
            "dependency",
            "query",
            "owners",
            "--package-revision",
            &d.logical,
            "--limit",
            "1",
            "--bytes",
            "4096",
            "--continuation",
            &d1_continuation,
        ],
        true,
    )?;
    require(
        field(&first_page, "owner", "reference")? != field(&second_page, "owner", "reference")?,
        "staged interface continuation repeated the same owner",
    )?;
    context.reject(
        &a,
        &[
            "package",
            "dependency",
            "query",
            "owners",
            "--package-revision",
            &b.logical,
            "--continuation",
            &d1_continuation,
        ],
        "builtin_continuation_foreign",
    )?;
    context.reject(
        &a,
        &[
            "package",
            "dependency",
            "query",
            "owners",
            "--package-revision",
            &d.logical,
            "--name",
            "offset",
            "--continuation",
            &d1_continuation,
        ],
        "builtin_continuation_selector",
    )?;
    context.reject(
        &a,
        &[
            "package",
            "dependency",
            "query",
            "owners",
            "--package-revision",
            &d.logical,
            "--bytes",
            "1",
        ],
        "builtin_query_bytes",
    )?;
    context.apply(&mut a, &format!("{}{}{}{}{}expression.local as=$arg value=$x\nexpression.local as=$arg2 value=$x\n{}{}{}{}{}",
        binding("add", &standard), binding("add", &b), binding("add", &c), module(), unary("$entry", "sum", "private", "$body", "$x"),
        call("$left", &reference(&b, "$entry")?, &["$arg"]), call("$right", &reference(&c, "$entry")?, &["$arg2"]), call("$body", &standard.symbols["add"], &["$left", "$right"]), target(), graph_test()))?;
    let first = context.export(&mut a)?;
    require(
        first.packages.len() == 5 && first.edges == 8,
        "independent diamond inventory must contain five packages and eight exact edges",
    )?;
    let imported_d = first
        .packages
        .iter()
        .find(|member| member.package == d.id)
        .ok_or_else(|| DevError::corrupt("diamond omitted D"))?;
    require(
        d1_inventory.packages.contains(imported_d),
        "producer D inventory differs after offline transfer",
    )?;
    for producer in [&b, &c] {
        let source = context
            .receipt
            .inventories
            .iter()
            .find(|inventory| {
                inventory
                    .packages
                    .iter()
                    .any(|member| member.package_revision == producer.logical)
            })
            .ok_or_else(|| DevError::corrupt("producer inventory missing"))?;
        for member in &source.packages {
            require(
                first.packages.contains(member),
                "producer inventory differs after offline transfer",
            )?;
        }
    }
    context.forbidden_reference(&a, &reference(&d, "$entry")?, "witness_dependency_target")?;
    context.receipt.observations.insert(
        "diamond_package_ids_distinct".to_owned(),
        (b.id != c.id && c.id != d.id).to_string(),
    );
    // Prepare successor transports before deleting every producer. No producer build is needed.
    let helper = d.symbols["$helper"].clone();
    let parameter = d.symbols["$hx"].clone();
    context.apply(&mut d, &format!("expression.local as=$new_arg value={parameter}\nexpression.i64 as=$two value=2\n{}replace.body function={helper} body=$new_sum\n", call("$new_sum", &standard.symbols["add"], &["$new_arg", "$two"])))?;
    context.export(&mut d)?;
    for package in [&mut b, &mut c] {
        context.stage(package, &d)?;
        context.apply(package, &binding("replace", &d))?;
        context.export(package)?;
    }
    for producer in [&d, &b, &c] {
        fs::remove_dir_all(&producer.path)?;
        require(!producer.path.exists(), "owned producer cleanup failed")?;
    }
    context.receipt.observations.insert(
        "producers_absent_before_execution".to_owned(),
        "true".to_owned(),
    );
    context.check(&a)?;
    context.run(&a, 11)?;
    context.cache_recovery(&a, "a1")?;
    let executable_input = context.evidence.join("a1-exact.lkja");
    context.reject(
        &a,
        &[
            "package",
            "dependency",
            "stage",
            "--transport",
            &d1.transport,
            "--input-file",
            &executable_input.display().to_string(),
        ],
        "package_container_contract",
    )?;
    let bare_pack = pack_files(&a.path)?
        .into_iter()
        .next()
        .ok_or_else(|| DevError::corrupt("negative bare-pack fixture missing"))?;
    context.reject(
        &a,
        &[
            "package",
            "dependency",
            "stage",
            "--transport",
            &d1.transport,
            "--input-file",
            &bare_pack.display().to_string(),
        ],
        "package_container_contract",
    )?;
    context.reject(
        &a,
        &[
            "package",
            "dependency",
            "stage",
            "--transport",
            &b1.transport,
            "--input-file",
            &d1.container.display().to_string(),
        ],
        "package_container_root",
    )?;
    let before_d2 = pack_files(&a.path)?;
    context.stage(&a, &d)?;
    context.reject(
        &a,
        &[
            "package",
            "dependency",
            "query",
            "owners",
            "--package-revision",
            &d.logical,
            "--continuation",
            &d1_continuation,
        ],
        "builtin_continuation_stale",
    )?;
    context.run(&a, 11)?;
    let after_d2 = pack_files(&a.path)?;
    let installed_d2 = after_d2.difference(&before_d2).cloned().collect::<Vec<_>>();
    require(
        installed_d2.len() == 1,
        "small D2 fixture did not install one independently identifiable source pack",
    )?;
    context.stage(&a, &c)?;
    let head = digest_file(&a.path.join("HEAD"), 1024 * 1024)?;
    let conflict = context.evidence.join("conflict.lkjc");
    fs::write(
        &conflict,
        format!("request base={}\n{}", a.revision, binding("replace", &c)),
    )?;
    context.reject(
        &a,
        &[
            "change",
            "plan",
            "--input-file",
            &conflict.display().to_string(),
        ],
        "package_revision_closure_package_conflict",
    )?;
    context.stage(&a, &b)?;
    let pending = context.evidence.join("stale-replacement.lkjc");
    fs::write(
        &pending,
        format!(
            "request base={}\n{}{}",
            a.revision,
            binding("replace", &b),
            binding("replace", &c)
        ),
    )?;
    let planned = context.cli(
        Some(&a.path),
        &[
            "change",
            "plan",
            "--input-file",
            &pending.display().to_string(),
        ],
        true,
    )?;
    let pending_token = field(&planned, "plan", "token")?;
    let unavailable_pack = &installed_d2[0];
    let held_pack = context.root.join("held-transitive-source.lkjp");
    let source_before = crate::authority::observe_graph_authority(&a.path)?;
    fs::rename(unavailable_pack, &held_pack)?;
    let missing = context.cli(
        Some(&a.path),
        &[
            "change",
            "apply",
            "--input-file",
            &pending.display().to_string(),
            "--plan",
            &pending_token,
        ],
        false,
    );
    fs::rename(&held_pack, unavailable_pack)?;
    require(
        crate::authority::observe_graph_authority(&a.path)? == source_before,
        "unavailable source changed complete accepted inventory",
    )?;
    let missing = missing?;
    require(
        field(&missing, "result", "status")? == "failure"
            && digest_file(&a.path.join("HEAD"), 1024 * 1024)? == head,
        "missing transitive source published a partial replacement",
    )?;
    let missing_message = field(&missing, "diagnostic", "message")?;
    require(
        missing_message.contains(&d.logical)
            && missing_message.contains("restage")
            && missing_message.contains("replan"),
        "missing transitive source must identify its exact revision and restage/replan action",
    )?;
    context.receipt.observations.insert(
        "unavailable_transitive_source".to_owned(),
        field(&missing, "diagnostic", "code")?,
    );
    let altered = context.evidence.join("altered-replacement.lkjc");
    fs::write(
        &altered,
        format!(
            "request base={} idempotency=altered-review\n{}{}",
            a.revision,
            binding("replace", &b),
            binding("replace", &c)
        ),
    )?;
    context.reject(
        &a,
        &[
            "change",
            "apply",
            "--input-file",
            &altered.display().to_string(),
            "--plan",
            &pending_token,
        ],
        "change_request_commitment_mismatch",
    )?;
    let conflicting_output = context.evidence.join("existing-output.lkjplan");
    fs::write(&conflicting_output, b"preserved existing output")?;
    context.reject(
        &a,
        &[
            "package",
            "current",
            "export",
            "--kind",
            "transport",
            "--output",
            &conflicting_output.display().to_string(),
        ],
        "output_conflict",
    )?;
    require(
        fs::read(&conflicting_output)? == b"preserved existing output",
        "failed review output overwrote existing bytes",
    )?;
    let symlink_output = context.root.join("symlink-output.lkjp");
    std::os::unix::fs::symlink(&conflicting_output, &symlink_output)?;
    context.reject(
        &a,
        &[
            "change",
            "plan",
            "--input-file",
            &pending.display().to_string(),
            "--output",
            &symlink_output.display().to_string(),
        ],
        "change_plan_output_type",
    )?;
    context.reject(
        &a,
        &[
            "package",
            "current",
            "export",
            "--kind",
            "transport",
            "--output",
            &symlink_output.display().to_string(),
        ],
        "output_conflict",
    )?;
    fs::remove_file(&symlink_output)?;
    require(
        fs::read(&conflicting_output)? == b"preserved existing output",
        "failed transport export followed an output symlink",
    )?;
    let replacement = context.apply(
        &mut a,
        &format!("{}{}", binding("replace", &b), binding("replace", &c)),
    )?;
    require(
        field(&replacement, "derived-cache", "status")? == "updated",
        "paired replacement omitted incremental compilation",
    )?;
    for key in ["compiled", "reused", "removed", "manifest"] {
        context.receipt.observations.insert(
            format!("post-replacement-{key}"),
            field(&replacement, "derived-cache", key)?,
        );
    }
    context.reject(
        &a,
        &[
            "change",
            "apply",
            "--input-file",
            &pending.display().to_string(),
            "--plan",
            &pending_token,
        ],
        "change_authored_stale_base",
    )?;
    context.check(&a)?;
    context.run(&a, 12)?;
    context.cache_recovery(&a, "a2")?;
    let second = context.export(&mut a)?;
    require(
        second.packages.len() == 5 && second.edges == 8,
        "replacement changed the fixed edge inventory",
    )?;
    require(
        second.packages.iter().all(|member| {
            member.package_revision != b1.logical
                && member.package_revision != c1.logical
                && member.package_revision != d1.logical
        }),
        "predecessor revision remains in selected closure",
    )?;
    let d2_member = second
        .packages
        .iter()
        .find(|member| member.package == d.id)
        .ok_or_else(|| DevError::corrupt("replacement D missing"))?;
    let private = d2_member
        .owners
        .iter()
        .find(|(owner, _, _)| owner == &d.symbols["$two"])
        .ok_or_else(|| DevError::corrupt("fixed private offset literal missing"))?;
    let body = offline_package_owner_bytes(
        &process::read_bounded(&d.container, MAXIMUM_CONTAINER_BYTES)?,
        &d.transport,
        &private.2,
    )
    .map_err(|error| DevError::corrupt(error.to_string()))?;
    let original_pack = process::read_bounded(unavailable_pack, MAXIMUM_CONTAINER_BYTES)?;
    let offsets = original_pack
        .windows(body.len())
        .enumerate()
        .filter_map(|(index, bytes)| (bytes == body).then_some(index))
        .collect::<Vec<_>>();
    require(
        offsets.len() == 1 && !body.is_empty(),
        "hostile source mutation must locate one exact canonical private body",
    )?;
    let mut corrupt_pack = original_pack.clone();
    corrupt_pack[offsets[0] + body.len() - 1] ^= 1;
    let canonical_before = crate::authority::observe_graph_authority(&a.path)?;
    fs::write(unavailable_pack, corrupt_pack)?;
    let canonical_failure = context.cli(Some(&a.path), &["check"], false);
    fs::write(unavailable_pack, original_pack)?;
    require(
        crate::authority::observe_graph_authority(&a.path)? == canonical_before,
        "canonical corruption handling changed accepted inventory",
    )?;
    let canonical_failure = canonical_failure?;
    require(
        field(&canonical_failure, "result", "status")? == "failure",
        "corrupt canonical body became a cache miss",
    )?;
    context.receipt.observations.insert(
        "canonical_body_corruption".to_owned(),
        field(&canonical_failure, "diagnostic", "code")?,
    );
    context.run(&a, 12)?;
    context
        .receipt
        .observations
        .insert("fixed_results".to_owned(), "11,11,12".to_owned());
    context.receipt.observations.insert(
        "imported_nominal_and_rank_one_generic".to_owned(),
        "fresh passed".to_owned(),
    );
    fs::remove_dir_all(&a.path)?;
    standalone_http(context, &d)?;
    Ok(())
}

fn pack_files(project: &Path) -> Result<std::collections::BTreeSet<PathBuf>, DevError> {
    fs::read_dir(project.join("packs"))?
        .map(|entry| {
            let entry = entry?;
            require(
                entry.file_type()?.is_file()
                    && entry
                        .path()
                        .extension()
                        .is_some_and(|extension| extension == "lkjp"),
                "owned canonical pack directory contains a foreign entry",
            )?;
            Ok(entry.path())
        })
        .collect()
}

fn standalone_http(context: &mut Context, dependency: &Package) -> Result<(), DevError> {
    let path = context.root.join("http-consumer");
    let created = context.cli(
        None,
        &[
            "new",
            &path.display().to_string(),
            "--template",
            "http",
            "--name",
            "offline-http",
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
    context.stage(&package, dependency)?;
    context.apply(
        &mut package,
        &format!(
            "{}{}replace.body function=application/response-text body=$wire\n",
            binding("add", dependency),
            call("$wire", &reference(dependency, "$http_body")?, &[])
        ),
    )?;
    context.export(&mut package)?;
    let standalone = context.root.join("standalone");
    fs::create_dir(&standalone)?;
    let artifact = standalone.join("application.lkja");
    let built = context.cli(
        Some(&package.path),
        &["build", "--output", &artifact.display().to_string()],
        true,
    )?;
    let artifact_identity = field(&built, "artifact", "bundle")?;
    let mut descriptor: serde_json::Value = serde_json::from_slice(&process::read_bounded(
        &package.path.join("service.deployment.json"),
        1024 * 1024,
    )?)?;
    descriptor["artifact"] = "application.lkja".into();
    let descriptor_path = standalone.join("service.deployment.json");
    fs::write(&descriptor_path, evidence::encode_json(&descriptor)?)?;
    fs::copy(&artifact, context.evidence.join("standalone.lkja"))?;
    fs::copy(
        &descriptor_path,
        context.evidence.join("standalone.deployment.json"),
    )?;
    fs::remove_dir_all(&package.path)?;
    for entry in fs::read_dir(&context.root)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name
            .to_str()
            .ok_or_else(|| DevError::corrupt("owned temporary path is not UTF-8"))?;
        if name.starts_with("transport-") && name.ends_with(".lkjp") || name == "standard.lkjp" {
            require(
                entry.file_type()?.is_file(),
                "owned source container became nonregular",
            )?;
            fs::remove_file(entry.path())?;
        }
    }
    for entry in fs::read_dir(&context.root)? {
        let entry = entry?;
        require(
            !entry.file_type()?.is_dir() || entry.path() == standalone,
            "project or dependency directory remains before standalone service",
        )?;
    }
    let stdout = context.evidence.join("standalone.stdout");
    let spec = process::ProcessSpec {
        command: vec![
            context.binary.display().to_string(),
            "serve".to_owned(),
            "--deployment".to_owned(),
            descriptor_path.display().to_string(),
        ],
        cwd: standalone,
        environment: BTreeMap::from([("LANG".to_owned(), "C.UTF-8".to_owned())]),
        timeout: Duration::from_secs(120),
        maximum_stdout_bytes: MAXIMUM_OUTPUT_BYTES,
        maximum_stderr_bytes: MAXIMUM_OUTPUT_BYTES,
        stdout_path: stdout.clone(),
        stderr_path: context.evidence.join("standalone.stderr"),
        unavailable_exit_code: None,
    };
    let control = process::ProcessControl::default();
    let child_control = control.clone();
    let observed_root = context.evidence.clone();
    let (sender, receiver) = std::sync::mpsc::channel();
    let thread = std::thread::Builder::new()
        .name("offline-package-service".to_owned())
        .spawn(move || {
            let _ = sender.send(process::run_controlled(
                &spec,
                &observed_root,
                &child_control,
            ));
        })?;
    let outcome: Result<(), DevError> = (|| {
        let started = Instant::now();
        let ready: serde_json::Value = loop {
            require(
                started.elapsed() < Duration::from_secs(30) && !thread.is_finished(),
                "standalone service exited or timed out before readiness",
            )?;
            if stdout.exists() {
                let bytes = process::read_bounded(&stdout, MAXIMUM_OUTPUT_BYTES)?;
                if let Some(end) = bytes.iter().position(|byte| *byte == b'\n') {
                    break serde_json::from_slice(&bytes[..end])?;
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        require(
            ready["event"] == "ready"
                && ready["ok"] == true
                && ready["deployment"]["artifact_digest"] == artifact_identity,
            "standalone readiness did not bind the exact executable closure",
        )?;
        let address = ready["local_address"]
            .as_str()
            .ok_or_else(|| DevError::corrupt("ready listener missing"))?
            .parse()
            .map_err(|_| DevError::corrupt("ready listener invalid"))?;
        let response = crate::http_probe::request(address, "GET", "/", &[], &[])?;
        require(
            response.status == 200 && response.body == b"offline-package-closure",
            "raw HTTP observation differs from the independently fixed non-built-in pure result",
        )?;
        fs::write(
            context.evidence.join("standalone-response.body"),
            response.body,
        )?;
        context.receipt.observations.insert(
            "standalone_http_body".to_owned(),
            "offline-package-closure".to_owned(),
        );
        context
            .receipt
            .observations
            .insert("standalone_artifact".to_owned(), artifact_identity);
        Ok(())
    })();
    control.interrupt();
    let terminal = receiver.recv_timeout(Duration::from_secs(35)).or_else(|_| {
        control.kill();
        receiver.recv_timeout(Duration::from_secs(5))
    });
    thread
        .join()
        .map_err(|_| DevError::infrastructure("standalone process observer failed"))?;
    let terminal =
        terminal.map_err(|_| DevError::infrastructure("standalone process failed to terminate"))?;
    let stopped = terminal.status == process::ProcessStatus::Passed;
    context.receipt.runners.push(terminal);
    outcome?;
    require(stopped, "standalone service did not shut down successfully")
}

fn module() -> &'static str {
    "create.module as=$module name=library\n"
}
fn graph_test() -> &'static str {
    "expression.i64 as=$actual value=7\nexpression.i64 as=$expected value=7\ncreate.test as=$test module=$module name=stable visibility=private actual=$actual expected=$expected\n"
}
fn binding(operation: &str, package: &Package) -> String {
    format!(
        "{operation}.dependency package={} semantic-revision={} package-revision={}\n",
        package.id, package.revision, package.logical
    )
}
fn unary(symbol: &str, name: &str, visibility: &str, body: &str, parameter: &str) -> String {
    format!(
        "create.function as={symbol} module=$module name={name} visibility={visibility} result=i64 effect=pure body={body}\nadd.parameter as={parameter} function={symbol} name=x type=i64\n"
    )
}
fn call(symbol: &str, function: &str, arguments: &[&str]) -> String {
    let mut text = format!("expression.call as={symbol} function={function}\n");
    for (index, expression) in arguments.iter().enumerate() {
        text.push_str(&format!(
            "expression.argument parent={symbol} index={index} expression={expression}\n"
        ));
    }
    text
}
fn target() -> &'static str {
    "type.function as=@entry result=i64\ntype.argument parent=@entry index=0 type=i64\ncreate.component as=$component module=$module name=consumer visibility=package\nadd.port as=$port component=$component name=main type=@entry function=$entry\ncreate.target as=$target name=main component=$component port=$port runner=command\n"
}
fn reference(package: &Package, symbol: &str) -> Result<String, DevError> {
    Ok(format!(
        "{}/{}",
        package.id,
        package
            .symbols
            .get(symbol)
            .ok_or_else(|| DevError::corrupt(format!(
                "missing public authoring allocation {symbol}"
            )))?
    ))
}
fn record_field(record: &CompactRecord, name: &str) -> Result<String, DevError> {
    record
        .fields
        .iter()
        .find(|field| field.name == name)
        .map(|field| field.value.clone())
        .ok_or_else(|| DevError::corrupt(format!("missing field {}.{name}", record.operation)))
}
fn field(records: &[CompactRecord], operation: &str, name: &str) -> Result<String, DevError> {
    record_field(
        records
            .iter()
            .find(|record| record.operation == operation)
            .ok_or_else(|| DevError::corrupt(format!("missing record {operation}")))?,
        name,
    )
}
fn require(condition: bool, message: &str) -> Result<(), DevError> {
    if condition {
        Ok(())
    } else {
        Err(DevError::corrupt(message))
    }
}
fn digest_file(path: &Path, maximum: u64) -> Result<String, DevError> {
    let metadata = fs::symlink_metadata(path)?;
    require(
        metadata.is_file() && !metadata.file_type().is_symlink(),
        "oracle input is not a regular file",
    )?;
    Ok(Sha256::digest(process::read_bounded(path, maximum)?)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn create_evidence_root(path: &Path) -> Result<PathBuf, DevError> {
    require(
        path.is_absolute()
            && !path
                .components()
                .any(|item| matches!(item, Component::CurDir | Component::ParentDir))
            && fs::symlink_metadata(path)
                .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound),
        "offline evidence root must be absent, absolute, and lexically canonical",
    )?;
    let parent = path
        .parent()
        .ok_or_else(|| DevError::usage("offline evidence root has no parent"))?;
    require(
        fs::symlink_metadata(parent)?.is_dir() && parent.canonicalize()? == parent,
        "offline evidence-root parent must be a canonical real directory",
    )?;
    fs::create_dir(path)?;
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(path.to_path_buf())
}

/// Admission validates the transferred evidence and reconstructs every retained source container;
/// it does not accept the child process exit status as proof of the package workflow.
pub(crate) fn read_transferred_receipt(
    path: &Path,
    candidate: &Path,
    verifier: &Path,
) -> Result<Receipt, DevError> {
    let bytes = process::read_bounded(path, 64 * 1024 * 1024)?;
    let receipt: Receipt = serde_json::from_slice(&bytes)?;
    let root = path
        .parent()
        .ok_or_else(|| DevError::corrupt("offline receipt parent missing"))?
        .canonicalize()?;
    require(
        evidence::encode_json(&receipt)? == bytes
            && path.canonicalize()? == root.join("receipt.json"),
        "offline receipt encoding or path is noncanonical",
    )?;
    require(
        receipt.schema == "lkjscript-offline-packages-acceptance-1"
            && receipt.status == "fresh passed"
            && receipt.failure.is_none()
            && receipt.cleanup_complete
            && !Path::new(&receipt.isolated_root).exists()
            && receipt.evidence_root == root.display().to_string()
            && receipt.environment_names == ["LANG"]
            && receipt.candidate_sha256 == digest_file(candidate, MAXIMUM_EXECUTABLE_BYTES)?
            && receipt.verifier_sha256 == digest_file(verifier, MAXIMUM_EXECUTABLE_BYTES)?
            && receipt.copied_candidate_sha256 == receipt.candidate_sha256,
        "offline receipt does not bind the exact transferred candidate, verifier, and cleanup",
    )?;
    for (key, value) in [
        ("fixed_results", "11,11,12"),
        ("diamond_package_ids_distinct", "true"),
        ("producers_absent_before_execution", "true"),
        ("imported_nominal_and_rank_one_generic", "fresh passed"),
        ("standalone_http_body", "offline-package-closure"),
        ("canonical_body_corruption", "pack_entry_checksum"),
        ("unavailable_transitive_source", "pack_file_missing"),
        ("post-replacement-compiled", "1"),
        ("post-replacement-reused", "3"),
        ("post-replacement-removed", "0"),
    ] {
        require(
            receipt.observations.get(key).map(String::as_str) == Some(value),
            "offline receipt omitted an independently fixed outcome",
        )?;
    }
    for code in [
        "change_authored_stale_base",
        "change_plan_output_type",
        "change_request_commitment_mismatch",
        "kernel_type_dependency_owner_missing",
        "output_conflict",
        "read_type",
        "witness_dependency_target",
        "builtin_continuation_foreign",
        "builtin_continuation_selector",
        "builtin_continuation_stale",
        "builtin_query_bytes",
        "package_container_contract",
        "package_container_root",
        "package_revision_closure_package_conflict",
    ] {
        require(
            receipt
                .observations
                .get(&format!("rejection-{code}"))
                .is_some_and(|digest| {
                    digest.len() == 64
                        && digest
                            .bytes()
                            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                }),
            "offline rejection omitted its complete unchanged authority inventory",
        )?;
    }
    require(
        receipt.runners.len() == 1 && receipt.runners[0].status == process::ProcessStatus::Passed,
        "standalone service observation missing or failed",
    )?;
    for label in ["a1", "a2"] {
        let exact = receipt
            .observations
            .get(&format!("artifact-{label}-exact"))
            .ok_or_else(|| DevError::corrupt("exact artifact evidence missing"))?;
        for mode in ["missing-cache", "corrupt-cache"] {
            require(
                receipt
                    .observations
                    .get(&format!("artifact-{label}-{mode}"))
                    == Some(exact),
                "clean and recovered artifacts disagree",
            )?;
        }
    }
    let mut previous = None;
    for file in &receipt.files {
        require(
            file.kind == evidence::FileKind::File
                && Path::new(&file.path).components().count() == 1
                && !Path::new(&file.path).is_absolute()
                && file.path != "receipt.json"
                && previous.is_none_or(|previous: &str| previous < file.path.as_str()),
            "offline evidence inventory is noncanonical",
        )?;
        require(
            evidence::proof(&root.join(&file.path), file.path.clone())? == *file,
            "offline evidence file changed after observation",
        )?;
        previous = Some(file.path.as_str());
    }
    let mut observed_files = fs::read_dir(&root)?
        .map(|entry| entry.map(|entry| entry.file_name().to_string_lossy().into_owned()))
        .collect::<Result<Vec<_>, _>>()?;
    observed_files.retain(|name| name != "receipt.json");
    observed_files.sort();
    require(
        observed_files
            == receipt
                .files
                .iter()
                .map(|file| file.path.clone())
                .collect::<Vec<_>>(),
        "offline evidence inventory omitted or added a file",
    )?;
    require(
        receipt.inventories.len() == 9
            && receipt.transport_digests.len() == 9
            && receipt.producer_inventories.len() == 9,
        "complete producer, replacement, and HTTP source inventories missing",
    )?;
    for (index, inventory) in receipt.inventories.iter().enumerate() {
        verify_producer_inventory(&receipt.producer_inventories[index], inventory)?;
        let input = process::read_bounded(
            &root.join(format!("transport-{}.lkjp", index + 1)),
            MAXIMUM_CONTAINER_BYTES,
        )?;
        require(
            offline_package_inventory(&input, &receipt.transport_digests[index])
                .map_err(|error| DevError::corrupt(error.to_string()))?
                == *inventory,
            "transferred independent source inventory changed",
        )?;
    }
    require(
        !receipt.commands.is_empty(),
        "offline command evidence missing",
    )?;
    let d2_producer = receipt
        .producer_inventories
        .get(4)
        .ok_or_else(|| DevError::corrupt("D2 producer inventory missing"))?;
    let d2_revision = receipt
        .inventories
        .get(4)
        .and_then(|inventory| {
            inventory
                .packages
                .iter()
                .find(|package| package.package == d2_producer.package)
        })
        .map(|package| package.package_revision.as_str())
        .ok_or_else(|| DevError::corrupt("D2 source revision missing"))?;
    let mut missing_source_diagnostic = false;
    for (index, command) in receipt.commands.iter().enumerate() {
        require(
            command.cwd == receipt.isolated_root
                && command.command.first().is_some_and(|binary| {
                    Path::new(binary) == Path::new(&receipt.isolated_root).join("lkjscript")
                }),
            "offline command consulted a foreign executable or checkout",
        )?;
        let expected = if command.expects_success {
            process::ProcessStatus::Passed
        } else {
            process::ProcessStatus::Failed
        };
        require(
            command.observation.status == expected
                && command.observation.signal.is_none()
                && command
                    .observation
                    .exit_code
                    .is_some_and(|code| (code == 0) == command.expects_success),
            "offline command classification does not match its expected boundary",
        )?;
        verify_observation_files(
            &command.observation,
            &receipt.files,
            &format!("command-{index:04}"),
        )?;
        let stdout = process::read_bounded(
            &root.join(&command.observation.stdout.path),
            MAXIMUM_OUTPUT_BYTES,
        )?;
        let records = parse_records("transferred-output", &stdout).map_err(|errors| {
            DevError::corrupt(format!("transferred public output: {errors:?}"))
        })?;
        if !command.expects_success {
            require(
                records
                    .iter()
                    .any(|record| record.operation == "diagnostic"),
                "transferred rejection omitted its diagnostic",
            )?;
            if field(&records, "diagnostic", "code")? == "pack_file_missing" {
                let message = field(&records, "diagnostic", "message")?;
                require(
                    message.contains(d2_revision)
                        && message.contains("restage")
                        && message.contains("replan"),
                    "transferred missing-source diagnostic lost its exact revision or correction",
                )?;
                missing_source_diagnostic = true;
            }
        }
    }
    require(
        missing_source_diagnostic,
        "transferred missing-source failure was not observed",
    )?;
    verify_observation_files(&receipt.runners[0], &receipt.files, "standalone")?;
    require(
        process::read_bounded(&root.join("standalone-response.body"), MAXIMUM_OUTPUT_BYTES)?
            == b"offline-package-closure",
        "transferred raw HTTP body changed",
    )?;
    Ok(receipt)
}

fn verify_producer_inventory(
    producer: &OfflineProducerInventory,
    inventory: &OfflinePackageInventory,
) -> Result<(), DevError> {
    let imported = inventory
        .packages
        .iter()
        .find(|package| package.package == producer.package)
        .ok_or_else(|| {
            DevError::corrupt("transport omitted the independently reconstructed producer")
        })?;
    require(
        imported.semantic_revision == producer.semantic_revision
            && imported.owners == producer.owners
            && imported.types == producer.types
            && imported.retirements == producer.retirements
            && imported.dependencies == producer.dependencies,
        "complete transported canonical owner/type/retirement/edge inventory differs from its accepted producer",
    )
}

fn verify_observation_files(
    observation: &process::ProcessObservation,
    files: &[evidence::FileProof],
    label: &str,
) -> Result<(), DevError> {
    require(
        observation.stdout_limit_bytes == MAXIMUM_OUTPUT_BYTES
            && observation.stderr_limit_bytes == MAXIMUM_OUTPUT_BYTES
            && !observation.stdout_limit_exhausted
            && !observation.stderr_limit_exhausted,
        "offline process output exhausted or changed its bound",
    )?;
    for (proof, suffix) in [
        (&observation.stdout, "stdout"),
        (&observation.stderr, "stderr"),
    ] {
        require(
            proof.path == format!("{label}.{suffix}") && files.contains(proof),
            "offline process output does not bind its retained file",
        )?;
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "bounded hostile path fixtures")]
mod tests {
    use super::*;

    #[test]
    fn independent_producer_join_detects_changed_private_body_and_omitted_edge() {
        let producer = OfflineProducerInventory {
            package: "fixed-root".to_owned(),
            semantic_revision: "fixed-revision".to_owned(),
            owners: vec![(
                "private-helper".to_owned(),
                "expression".to_owned(),
                "fixed-body-object".to_owned(),
            )],
            types: vec!["i64-type".to_owned()],
            retirements: Vec::new(),
            dependencies: vec![("transitive-D".to_owned(), "exact-D1".to_owned())],
        };
        let mut inventory = OfflinePackageInventory {
            packages: vec![
                lkjscript::platform::contributor::OfflinePackageInventoryMember {
                    package: producer.package.clone(),
                    semantic_revision: producer.semantic_revision.clone(),
                    package_revision: "fixed-logical".to_owned(),
                    interface: "fixed-public-interface".to_owned(),
                    owners: producer.owners.clone(),
                    types: producer.types.clone(),
                    retirements: Vec::new(),
                    dependencies: producer.dependencies.clone(),
                    public_owners: Vec::new(),
                },
            ],
            objects: 1,
            edges: 1,
            bytes: 1,
            oracle_validation_visits: 1,
            oracle_validation_read_bytes: 1,
        };
        verify_producer_inventory(&producer, &inventory).unwrap();
        inventory.packages[0].owners[0].2 = "changed-private-body".to_owned();
        assert!(verify_producer_inventory(&producer, &inventory).is_err());
        inventory.packages[0].owners = producer.owners.clone();
        inventory.packages[0].dependencies.clear();
        assert!(verify_producer_inventory(&producer, &inventory).is_err());
    }

    #[test]
    fn offline_evidence_paths_reject_existing_symlink_and_parent_escape() {
        let temporary = tempfile::tempdir().unwrap();
        let parent = temporary.path().canonicalize().unwrap();
        let absent = parent.join("evidence");
        assert_eq!(create_evidence_root(&absent).unwrap(), absent);
        assert!(create_evidence_root(&absent).is_err());
        assert!(create_evidence_root(&parent.join("evidence/../escape")).is_err());
        let alias = parent.join("alias");
        std::os::unix::fs::symlink(&absent, &alias).unwrap();
        assert!(create_evidence_root(&alias).is_err());
        assert!(create_evidence_root(&alias.join("nested")).is_err());
        let executable = parent.join("candidate");
        fs::write(&executable, b"exact candidate").unwrap();
        let link = parent.join("candidate-link");
        std::os::unix::fs::symlink(&executable, &link).unwrap();
        assert!(digest_file(&link, 1024).is_err());
        assert!(digest_file(&executable, 1024).is_ok());
    }
}
