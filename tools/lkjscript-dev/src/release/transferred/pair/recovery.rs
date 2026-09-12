//! One small two-version recovery/continuity witness per pair, alongside the existing five owners.
use super::*;
use lkjscript::platform::control::CompactRecord;
use std::io::Write;
use std::net::{SocketAddr, TcpStream};
use std::sync::mpsc;
use std::thread;

const OLD_TAG: &str = "v0.1.32";
const OLD_SOURCE: &str = "67baaf0b081842e0e2e3745e8d5503e22cc791e4";
const OLD_SHA: &str = "3beed341cbb315999531746d0f2ed197df4871ba9e6e0393666b1feaf11efa66";
const OLD_BYTES: u64 = 9_877_724;
const OLD_URL: &str = "https://github.com/lkjsxc/lkjscript/releases/download/v0.1.32/lkjscript-x86_64-unknown-linux-musl.tar.gz";
const REQUEST: &[u8] = b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Command {
    name: String,
    arguments: Vec<String>,
    environment: BTreeMap<String, String>,
    process: process::ProcessObservation,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Resident {
    command: Command,
    pinned_executable: FileBinding,
    responses: [FileBinding; 2],
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Recovery {
    predecessor: FileBinding,
    admission: archive::VerifiedArchive,
    commands: Vec<Command>,
    residents: Vec<Resident>,
    retained: Vec<FileBinding>,
    bundles: Vec<FileBinding>,
    started_unix_nanoseconds: u128,
    completed_unix_nanoseconds: u128,
    cleanup_complete: bool,
}
fn root(options: &PairOptions) -> PathBuf {
    options.evidence_root.join("recovery")
}
fn old(options: &PairOptions) -> PathBuf {
    installation::prefix(options, Route::Exact)
        .join("lib/lkjscript/versions")
        .join(OLD_TAG)
        .join(target::TARGET_TRIPLE)
        .join("lkjscript")
}
fn manager(options: &PairOptions) -> PathBuf {
    installation::candidate(options, Route::Exact)
}
fn default(options: &PairOptions) -> PathBuf {
    installation::prefix(options, Route::Exact).join("bin/lkjscript")
}
fn environment(options: &PairOptions) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("LANG".to_owned(), "C".to_owned()),
        // Every native operation works with acquisition tools entirely absent from PATH.
        ("PATH".to_owned(), String::new()),
        (
            "HOME".to_owned(),
            root(options).join("runtime").display().to_string(),
        ),
        (
            "TMPDIR".to_owned(),
            root(options).join("runtime").display().to_string(),
        ),
    ])
}
fn invocation(executable: &Path, args: &[&str]) -> Vec<String> {
    std::iter::once(executable.display().to_string())
        .chain(args.iter().map(|s| (*s).to_owned()))
        .collect()
}
fn install(options: &PairOptions) -> Vec<String> {
    invocation(
        &manager(options),
        &[
            "runtime",
            "install",
            "--archive",
            &root(options)
                .join(archive::ARCHIVE_NAME)
                .display()
                .to_string(),
            "--sha256",
            OLD_SHA,
            "--prefix",
            &installation::prefix(options, Route::Exact)
                .display()
                .to_string(),
        ],
    )
}
fn select(options: &PairOptions, tag: &str) -> Vec<String> {
    invocation(
        &manager(options),
        &[
            "runtime",
            "select",
            tag,
            "--prefix",
            &installation::prefix(options, Route::Exact)
                .display()
                .to_string(),
        ],
    )
}
fn plan(options: &PairOptions) -> Vec<(String, Vec<String>, bool)> {
    let root = root(options);
    let mut plan = vec![
        (
            "acquire-predecessor".to_owned(),
            vec![
                "/usr/bin/curl",
                "-q",
                "--fail",
                "--location",
                "--silent",
                "--show-error",
                "--proto",
                "=https",
                "--proto-redir",
                "=https",
                "--connect-timeout",
                "15",
                "--max-time",
                "180",
                "--retry",
                "2",
                "--retry-max-time",
                "240",
                "--max-filesize",
                "9877724",
                "--output",
                &root.join(archive::ARCHIVE_NAME).display().to_string(),
                OLD_URL,
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
            true,
        ),
        ("install-predecessor".to_owned(), install(options), true),
        ("install-identical".to_owned(), install(options), true),
        (
            "unknown-selection".to_owned(),
            select(options, "v999.0.0"),
            false,
        ),
    ];
    for (name, executable) in [("new", manager(options)), ("old", old(options))] {
        let project = root.join("runtime").join(name).display().to_string();
        let artifact = root
            .join(name)
            .join("generated/application.lkja")
            .display()
            .to_string();
        for (suffix, args) in [
            (
                "new",
                vec!["new", &project, "--template", "http", "--name", name],
            ),
            ("check", vec!["--project", &project, "check"]),
            (
                "build",
                vec!["--project", &project, "build", "--output", &artifact],
            ),
        ] {
            plan.push((
                format!("{name}-{suffix}"),
                invocation(&executable, &args),
                true,
            ));
        }
    }
    plan.extend([
        ("select-old".to_owned(), select(options, OLD_TAG), true),
        (
            "old-default-version".to_owned(),
            invocation(&default(options), &["--version"]),
            true,
        ),
        (
            "old-default-has-no-manager".to_owned(),
            invocation(
                &default(options),
                &[
                    "runtime",
                    "list",
                    "--prefix",
                    &installation::prefix(options, Route::Exact)
                        .display()
                        .to_string(),
                ],
            ),
            false,
        ),
        (
            "select-old-identical".to_owned(),
            select(options, OLD_TAG),
            true,
        ),
        (
            "incompatible-bundle".to_owned(),
            invocation(
                &old(options),
                &[
                    "serve",
                    "--deployment",
                    &root
                        .join("invalid/service.deployment.json")
                        .display()
                        .to_string(),
                ],
            ),
            false,
        ),
        (
            "recover-new-manager".to_owned(),
            select(options, &options.tag),
            true,
        ),
        (
            "select-new-identical".to_owned(),
            select(options, &options.tag),
            true,
        ),
        (
            "inventory".to_owned(),
            invocation(
                &manager(options),
                &[
                    "runtime",
                    "list",
                    "--prefix",
                    &installation::prefix(options, Route::Exact)
                        .display()
                        .to_string(),
                ],
            ),
            true,
        ),
    ]);
    plan
}
fn spec(options: &PairOptions, name: &str, arguments: Vec<String>) -> process::ProcessSpec {
    process::ProcessSpec {
        command: arguments,
        cwd: root(options).join("runtime"),
        environment: environment(options),
        timeout: Duration::from_secs(600),
        maximum_stdout_bytes: MAXIMUM_OUTPUT_BYTES,
        maximum_stderr_bytes: MAXIMUM_OUTPUT_BYTES,
        stdout_path: root(options).join(format!("{name}.stdout.log")),
        stderr_path: root(options).join(format!("{name}.stderr.log")),
        unavailable_exit_code: None,
    }
}
fn records(options: &PairOptions, command: &Command) -> Result<Vec<CompactRecord>, DevError> {
    super::super::super::admission::compact(
        &command.name,
        &process::read_bounded(
            &root(options).join(&command.process.stdout.path),
            MAXIMUM_OUTPUT_BYTES,
        )?,
    )
}
fn observe(
    options: &PairOptions,
    name: &str,
    arguments: Vec<String>,
    success: bool,
    control: &process::ProcessControl,
) -> Result<Command, DevError> {
    require(!control.cancelled(), "recovery witness cancelled")?;
    let spec = spec(options, name, arguments.clone());
    let p = process::run_supervised(&spec, &root(options), Some(control));
    let result = Command {
        name: name.to_owned(),
        arguments,
        environment: spec.environment,
        process: p,
    };
    validate_command(options, &result, success)?;
    Ok(result)
}
fn validate_command(
    options: &PairOptions,
    command: &Command,
    success: bool,
) -> Result<(), DevError> {
    let p = &command.process;
    require(
        command.environment == environment(options)
            && p.signal.is_none()
            && if success {
                p.reason.is_none()
            } else {
                p.reason.as_deref() == Some("nonzero_exit")
            }
            && !p.stdout_limit_exhausted
            && !p.stderr_limit_exhausted
            && p.stdout_limit_bytes == MAXIMUM_OUTPUT_BYTES
            && p.stderr_limit_bytes == MAXIMUM_OUTPUT_BYTES
            && if success {
                p.status == process::ProcessStatus::Passed && p.exit_code == Some(0)
            } else {
                p.status == process::ProcessStatus::Failed && p.exit_code.is_some_and(|n| n != 0)
            },
        "recovery process failed its expected disposition",
    )?;
    for (proof, suffix) in [(&p.stdout, "stdout.log"), (&p.stderr, "stderr.log")] {
        let name = format!("{}.{suffix}", command.name);
        require(
            proof.path == name
                && *proof == evidence::proof(&root(options).join(&name), name.clone())?,
            "recovery process log binding changed",
        )?;
    }
    if command.name.starts_with("select-") || command.name == "recover-new-manager" {
        let records = records(options, command)?;
        let expected = if command.name.starts_with("select-old") {
            OLD_TAG
        } else {
            &options.tag
        };
        require(
            lifecycle::value(&records, "selection", "tag")? == expected
                && lifecycle::value(&records, "recovery", "retained-manager")? == "true"
                && lifecycle::value(&records, "recovery", "manager-path")?
                    == manager(options).display().to_string(),
            "selection lost the retained new manager",
        )?;
    }
    if command.name == "old-default-version" {
        require(
            process::read_bounded(&root(options).join(&p.stdout.path), 4096)?
                == b"lkjscript 0.1.32\n",
            "default did not invoke the exact predecessor",
        )?;
    }
    if command.name == "incompatible-bundle" {
        let failure: serde_json::Value = serde_json::from_slice(&process::read_bounded(
            &root(options).join(&p.stdout.path),
            MAXIMUM_OUTPUT_BYTES,
        )?)?;
        require(
            failure["ok"] == false && failure["error"]["code"] == "artifact_bundle_contract",
            "chosen predecessor did not reject the foreign artifact contract before readiness",
        )?;
    }
    if command.name.ends_with("-check") {
        let records = records(options, command)?;
        require(
            lifecycle::value(&records, "tests", "failed")? == "0"
                && lifecycle::value(&records, "tests", "differential")? == "equal",
            "installed application check failed",
        )?;
    }
    if command.name == "install-predecessor" || command.name == "install-identical" {
        let records = records(options, command)?;
        require(
            lifecycle::value(&records, "selection", "tag")? == options.tag,
            "nonactivating installation changed the default",
        )?;
    }
    Ok(())
}
fn admit_predecessor(options: &PairOptions) -> Result<archive::VerifiedArchive, DevError> {
    let path = root(options).join(archive::ARCHIVE_NAME);
    let bytes = process::read_bounded(&path, OLD_BYTES)?;
    require(
        bytes.len() as u64 == OLD_BYTES && archive::sha256_bytes(&bytes)?.as_str() == OLD_SHA,
        "immutable predecessor acquisition identity differs",
    )?;
    let admitted = lkjscript::release_container::admit_gzip(&bytes)?.verified;
    require(
        admitted.manifest.source.expected_release_tag == OLD_TAG
            && admitted.manifest.source.tagged_commit_sha == OLD_SOURCE
            && admitted.manifest.publication_mode == PublicationMode::Release,
        "predecessor historical identity differs",
    )?;
    Ok(admitted)
}
fn retained(
    options: &PairOptions,
    current: &archive::VerifiedArchive,
    previous: &archive::VerifiedArchive,
) -> Result<Vec<FileBinding>, DevError> {
    let mut files = Vec::new();
    for (executable, admission) in [(manager(options), current), (old(options), previous)] {
        for member in admission.members.iter().skip(1) {
            let file =
                binding(&executable.with_file_name(member.name.trim_start_matches("lkjscript/")))?;
            require(
                file.mode == member.mode
                    && file.file.byte_length == member.byte_length
                    && Some(&file.file.sha256) == member.sha256.as_ref(),
                "selection changed immutable payload bytes",
            )?;
            files.push(file);
        }
        require(
            target::inspect_static_elf(&executable)? == admission.manifest.executable.elf,
            "retained static target differs",
        )?;
        files.push(binding(&executable.with_file_name("INSTALL-RECEIPT.json"))?);
    }
    Ok(files)
}
fn bundle_files(options: &PairOptions) -> Result<Vec<FileBinding>, DevError> {
    let mut files = Vec::new();
    for name in ["new", "old", "invalid"] {
        for payload in ["service.deployment.json", "generated/application.lkja"] {
            files.push(binding(&root(options).join(name).join(payload))?);
        }
    }
    Ok(files)
}
fn prepare_bundles(options: &PairOptions) -> Result<(), DevError> {
    for name in ["new", "old"] {
        let project = root(options).join("runtime").join(name);
        fs::copy(
            project.join("service.deployment.json"),
            root(options).join(name).join("service.deployment.json"),
        )?;
        fs::remove_dir_all(project)?;
    }
    // An explicitly incompatible artifact header is rejected by the selected old runtime before readiness/effects.
    fs::copy(
        root(options).join("old/service.deployment.json"),
        root(options).join("invalid/service.deployment.json"),
    )?;
    let mut incompatible = process::read_bounded(
        &root(options).join("old/generated/application.lkja"),
        128 * 1024 * 1024,
    )?;
    let first = incompatible
        .first_mut()
        .ok_or_else(|| DevError::corrupt("authored old artifact is empty"))?;
    *first ^= 0xff;
    archive::write_new(
        &root(options).join("invalid/generated/application.lkja"),
        &incompatible,
        0o644,
    )?;
    Ok(())
}
struct Active {
    name: String,
    arguments: Vec<String>,
    control: process::ProcessControl,
    receiver: mpsc::Receiver<process::ProcessObservation>,
    thread: Option<thread::JoinHandle<()>>,
}
impl Drop for Active {
    fn drop(&mut self) {
        self.control.kill();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
impl Active {
    fn start(options: &PairOptions, name: &str) -> Result<Self, DevError> {
        let executable = if name == "new" {
            default(options)
        } else {
            old(options)
        };
        let arguments = invocation(
            &executable,
            &[
                "serve",
                "--deployment",
                &root(options)
                    .join(name)
                    .join("service.deployment.json")
                    .display()
                    .to_string(),
            ],
        );
        let name = format!("resident-{name}");
        let spec = spec(options, &name, arguments.clone());
        let control = process::ProcessControl::default();
        let child_control = control.clone();
        let root = root(options);
        let (sender, receiver) = mpsc::channel();
        let thread = thread::Builder::new().name(name.clone()).spawn(move || {
            let observation = process::run_controlled(&spec, &root, &child_control);
            let _ = sender.send(observation);
        })?;
        Ok(Self {
            name,
            arguments,
            control,
            receiver,
            thread: Some(thread),
        })
    }
    fn ready(
        &self,
        options: &PairOptions,
        control: &process::ProcessControl,
    ) -> Result<SocketAddr, DevError> {
        let started = Instant::now();
        let log = root(options).join(format!("{}.stdout.log", self.name));
        loop {
            require(
                !control.cancelled() && started.elapsed() < Duration::from_secs(30),
                "resident readiness cancelled or timed out",
            )?;
            if log.is_file() {
                let bytes = process::read_bounded(&log, MAXIMUM_OUTPUT_BYTES)?;
                if let Some(line) = bytes
                    .split_inclusive(|b| *b == b'\n')
                    .find(|line| line.ends_with(b"\n"))
                {
                    return ready_address(line);
                }
            }
            require(
                matches!(self.receiver.try_recv(), Err(mpsc::TryRecvError::Empty)),
                "resident exited before readiness",
            )?;
            thread::sleep(Duration::from_millis(20));
        }
    }
    fn finish(
        mut self,
        options: &PairOptions,
        executable: PathBuf,
        responses: [FileBinding; 2],
    ) -> Result<Resident, DevError> {
        self.control.interrupt();
        let observation = self
            .receiver
            .recv_timeout(Duration::from_secs(30))
            .map_err(|e| DevError::infrastructure(format!("resident shutdown: {e}")))?;
        if let Some(thread) = self.thread.take() {
            thread
                .join()
                .map_err(|_| DevError::infrastructure("resident supervisor panicked"))?;
        }
        let proof = Resident {
            command: Command {
                name: self.name.clone(),
                arguments: self.arguments.clone(),
                environment: environment(options),
                process: observation,
            },
            pinned_executable: binding(&executable)?,
            responses,
        };
        validate_resident(options, &proof)?;
        Ok(proof)
    }
}
fn ready_address(line: &[u8]) -> Result<SocketAddr, DevError> {
    let value: serde_json::Value = serde_json::from_slice(line)?;
    require(
        value["event"] == "ready"
            && value["ok"] == true
            && value["deployment"]["runner"] == "http"
            && value["deployment"]["listen"] == "127.0.0.1:0",
        "resident readiness differs from the authored deployment",
    )?;
    let address: SocketAddr = value["local_address"]
        .as_str()
        .ok_or_else(|| DevError::corrupt("resident address missing"))?
        .parse()
        .map_err(|_| DevError::corrupt("resident address malformed"))?;
    require(
        address.ip().is_loopback() && address.port() != 0,
        "resident listener is outside the owned fixture",
    )?;
    Ok(address)
}
fn request(
    options: &PairOptions,
    name: &str,
    ordinal: usize,
    address: SocketAddr,
) -> Result<FileBinding, DevError> {
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(5))?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    stream.write_all(REQUEST)?;
    let mut bytes = Vec::new();
    stream.take(65537).read_to_end(&mut bytes)?;
    validate_response(&bytes)?;
    let path = root(options).join(format!("{name}-response-{ordinal}.http"));
    archive::write_new(&path, &bytes, 0o644)?;
    binding(&path)
}
fn validate_response(bytes: &[u8]) -> Result<(), DevError> {
    require(
        bytes.len() <= 65536
            && bytes.starts_with(b"HTTP/1.1 200 OK\r\n")
            && bytes.ends_with(b"\r\n\r\nhello from lkjscript"),
        "independent starter HTTP response differs",
    )
}
fn validate_resident(options: &PairOptions, resident: &Resident) -> Result<(), DevError> {
    validate_command(options, &resident.command, true)?;
    let name = resident
        .command
        .name
        .strip_prefix("resident-")
        .ok_or_else(|| DevError::corrupt("foreign resident name"))?;
    require(["new", "old"].contains(&name), "foreign resident")?;
    let executable = if name == "new" {
        default(options)
    } else {
        old(options)
    };
    require(
        resident.command.arguments
            == invocation(
                &executable,
                &[
                    "serve",
                    "--deployment",
                    &root(options)
                        .join(name)
                        .join("service.deployment.json")
                        .display()
                        .to_string(),
                ],
            )
            && resident.pinned_executable
                == binding(&if name == "new" {
                    manager(options)
                } else {
                    old(options)
                })?,
        "resident executable identity changed",
    )?;
    let stdout = process::read_bounded(
        &root(options).join(&resident.command.process.stdout.path),
        MAXIMUM_OUTPUT_BYTES,
    )?;
    let mut lines = stdout.split(|b| *b == b'\n').filter(|s| !s.is_empty());
    ready_address(
        lines
            .next()
            .ok_or_else(|| DevError::corrupt("resident ready event missing"))?,
    )?;
    let stopped: serde_json::Value = serde_json::from_slice(
        lines
            .next()
            .ok_or_else(|| DevError::corrupt("resident stopped event missing"))?,
    )?;
    require(
        lines.next().is_none()
            && stopped["event"] == "stopped"
            && stopped["ok"] == true
            && stopped["receipt"]["shutdown"]["admission_stopped"] == true
            && stopped["receipt"]["shutdown"]["remaining_tasks"] == 0
            && stopped["receipt"]["shutdown"]["cleanup_failures"] == serde_json::json!([]),
        "resident cleanup is incomplete",
    )?;
    for (ordinal, proof) in resident.responses.iter().enumerate() {
        let path = root(options).join(format!("{name}-response-{ordinal}.http"));
        require(
            *proof == binding(&path)?,
            "resident response binding differs",
        )?;
        validate_response(&process::read_bounded(&path, 65536)?)?;
    }
    Ok(())
}
pub(super) fn run(
    options: &PairOptions,
    current: &archive::VerifiedArchive,
    control: &process::ProcessControl,
) -> Result<Recovery, DevError> {
    let started = super::super::super::unix_nanoseconds()?;
    let root = root(options);
    fs::create_dir(&root)?;
    fs::create_dir(root.join("runtime"))?;
    for name in ["new", "old", "invalid"] {
        fs::create_dir_all(root.join(name).join("generated"))?;
    }
    let result = (|| {
        let mut commands = Vec::new();
        let plan = plan(options);
        commands.push(observe(
            options,
            &plan[0].0,
            plan[0].1.clone(),
            true,
            control,
        )?);
        let admission = admit_predecessor(options)?;
        for (name, command, success) in &plan[1..10] {
            commands.push(observe(options, name, command.clone(), *success, control)?);
        }
        prepare_bundles(options)?;
        let unchanged = retained(options, current, &admission)?;
        let bundles = bundle_files(options)?;
        let new = Active::start(options, "new")?;
        let new_address = new.ready(options, control)?;
        let old_runner = Active::start(options, "old")?;
        let old_address = old_runner.ready(options, control)?;
        let new_before = request(options, "new", 0, new_address)?;
        let old_before = request(options, "old", 0, old_address)?;
        for (name, command, success) in &plan[10..14] {
            commands.push(observe(options, name, command.clone(), *success, control)?);
        }
        require(
            fs::read_link(default(options))?
                == Path::new(&format!(
                    "../lib/lkjscript/versions/{OLD_TAG}/{}/lkjscript",
                    target::TARGET_TRIPLE
                )),
            "default did not select the complete predecessor",
        )?;
        let new_after = request(options, "new", 1, new_address)?;
        let old_after = request(options, "old", 1, old_address)?;
        for (name, command, success) in &plan[14..] {
            commands.push(observe(options, name, command.clone(), *success, control)?);
        }
        let residents = vec![
            new.finish(options, manager(options), [new_before, new_after])?,
            old_runner.finish(options, old(options), [old_before, old_after])?,
        ];
        require(
            retained(options, current, &admission)? == unchanged
                && bundle_files(options)? == bundles,
            "selection migrated application or installed bytes",
        )?;
        Ok(Recovery {
            predecessor: binding(&root.join(archive::ARCHIVE_NAME))?,
            admission,
            commands,
            residents,
            retained: unchanged,
            bundles,
            started_unix_nanoseconds: started,
            completed_unix_nanoseconds: super::super::super::unix_nanoseconds()?,
            cleanup_complete: false,
        })
    })();
    let cleanup = fs::remove_dir_all(root.join("runtime"));
    let mut receipt = match (result, cleanup) {
        (Ok(proof), Ok(())) => proof,
        (Err(error), Ok(())) => return Err(error),
        (Ok(_), Err(cleanup)) => return Err(cleanup.into()),
        (Err(error), Err(cleanup)) => {
            return Err(DevError::infrastructure(format!(
                "{error}; owned recovery cleanup also failed: {cleanup}"
            )));
        }
    };
    receipt.cleanup_complete = true;
    evidence::publish_json(&root.join("receipt.json"), &receipt)?;
    validate(options, current, &receipt)?;
    Ok(receipt)
}
pub(super) fn validate(
    options: &PairOptions,
    current: &archive::VerifiedArchive,
    receipt: &Recovery,
) -> Result<(), DevError> {
    require(
        process::read_bounded(&root(options).join("receipt.json"), MAXIMUM_RECEIPT_BYTES)?
            == evidence::encode_json(receipt)?,
        "recovery source receipt changed",
    )?;
    require(
        receipt.cleanup_complete
            && !root(options).join("runtime").exists()
            && receipt.started_unix_nanoseconds > 0
            && receipt.completed_unix_nanoseconds >= receipt.started_unix_nanoseconds
            && receipt.predecessor == binding(&root(options).join(archive::ARCHIVE_NAME))?
            && receipt.admission == admit_predecessor(options)?
            && receipt.retained == retained(options, current, &receipt.admission)?
            && receipt.bundles == bundle_files(options)?,
        "recovery evidence is foreign, changed or incomplete",
    )?;
    require(
        fs::read_link(default(options))?
            == Path::new(&format!(
                "../lib/lkjscript/versions/{}/{}/lkjscript",
                options.tag,
                target::TARGET_TRIPLE
            )),
        "new manager did not restore the exact current selection",
    )?;
    let expected = plan(options);
    require(
        receipt.commands.len() == expected.len() && receipt.residents.len() == 2,
        "recovery command/resident omitted or added",
    )?;
    for (observed, (name, args, success)) in receipt.commands.iter().zip(expected) {
        require(
            observed.name == name && observed.arguments == args,
            "recovery command was redirected, omitted or duplicated",
        )?;
        validate_command(options, observed, success)?;
    }
    for (resident, name) in receipt
        .residents
        .iter()
        .zip(["resident-new", "resident-old"])
    {
        require(resident.command.name == name, "resident order differs")?;
        validate_resident(options, resident)?;
    }
    Ok(())
}
