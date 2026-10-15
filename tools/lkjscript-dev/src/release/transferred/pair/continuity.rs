//! One current-format selector switch while the accepted installed resident stays alive.
use super::*;
use std::io::Write;
use std::net::{SocketAddr, TcpStream};
use std::sync::mpsc;
use std::thread;

const COMMANDS: [&str; 10] = [
    "tar-version",
    "gzip-version",
    "tar",
    "gzip",
    "install",
    "new",
    "check",
    "build",
    "select-alternate",
    "select-current",
];
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
pub(super) struct Continuity {
    fixture: String,
    archive: FileBinding,
    admission: archive::VerifiedArchive,
    commands: Vec<Command>,
    resident: Command,
    original_executable: FileBinding,
    responses: [FileBinding; 3],
    retained: Vec<FileBinding>,
    cleanup_complete: bool,
}
fn root(options: &PairOptions) -> PathBuf {
    options.evidence_root.join("current-continuity")
}
fn alternate_tag(options: &PairOptions) -> &'static str {
    if options.tag == "v999.0.0" {
        "v999.0.1"
    } else {
        "v999.0.0"
    }
}
fn default(options: &PairOptions) -> PathBuf {
    installation::prefix(options, Route::Exact).join("bin/lkjscript")
}
fn manager(options: &PairOptions) -> PathBuf {
    installation::candidate(options, Route::Exact)
}
fn pointer(tag: &str) -> PathBuf {
    PathBuf::from(format!(
        "../lib/lkjscript/versions/{tag}/{}/lkjscript",
        target::TARGET_TRIPLE
    ))
}
fn selected(options: &PairOptions, tag: &str) -> Result<(), DevError> {
    require(
        fs::read_link(default(options))? == pointer(tag),
        "continuity selected the wrong slot",
    )
}
fn environment(options: &PairOptions) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("LANG".to_owned(), "C".to_owned()),
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
fn arguments(
    options: &PairOptions,
    current: &archive::VerifiedArchive,
    name: &str,
) -> Result<Vec<String>, DevError> {
    let root = root(options);
    let prefix = installation::prefix(options, Route::Exact);
    let project = root.join("runtime/project");
    let archive_path = root.join("alternate.tar.gz");
    Ok(match name {
        "tar-version" => vec!["/usr/bin/tar".to_owned(), "--version".to_owned()],
        "gzip-version" => vec!["/usr/bin/gzip".to_owned(), "--version".to_owned()],
        "tar" => archive::normalized_tar_invocation(current.source_timestamp_unix_seconds)
            .into_iter()
            .map(|argument| match argument.as_str() {
                "tar" => "/usr/bin/tar".to_owned(),
                "--file=$TAR" => format!("--file={}", root.join("alternate.tar").display()),
                "--directory=$STAGE" => format!("--directory={}", root.join("stage").display()),
                _ => argument,
            })
            .collect(),
        "gzip" => vec![
            "/usr/bin/gzip".to_owned(),
            "--no-name".to_owned(),
            "--best".to_owned(),
            root.join("alternate.tar").display().to_string(),
        ],
        "install" => invocation(
            &manager(options),
            &[
                "runtime",
                "install",
                "--archive",
                &archive_path.display().to_string(),
                "--sha256",
                archive::sha256_file(&archive_path)?.0.as_str(),
                "--prefix",
                &prefix.display().to_string(),
            ],
        ),
        "new" => invocation(
            &manager(options),
            &[
                "new",
                &project.display().to_string(),
                "--template",
                "http",
                "--name",
                "continuity",
            ],
        ),
        "check" => invocation(
            &manager(options),
            &["--project", &project.display().to_string(), "check"],
        ),
        "build" => invocation(
            &manager(options),
            &[
                "--project",
                &project.display().to_string(),
                "build",
                "--output",
                &root
                    .join("application/generated/application.lkja")
                    .display()
                    .to_string(),
            ],
        ),
        "select-alternate" | "select-current" => invocation(
            &manager(options),
            &[
                "runtime",
                "select",
                if name == "select-alternate" {
                    alternate_tag(options)
                } else {
                    &options.tag
                },
                "--prefix",
                &prefix.display().to_string(),
            ],
        ),
        "resident" => invocation(
            &default(options),
            &[
                "serve",
                "--deployment",
                &root
                    .join("application/service.deployment.json")
                    .display()
                    .to_string(),
            ],
        ),
        _ => return Err(DevError::corrupt("foreign continuity command")),
    })
}
fn spec(options: &PairOptions, name: &str, arguments: Vec<String>) -> process::ProcessSpec {
    process::ProcessSpec {
        command: arguments,
        cwd: root(options).join("runtime"),
        environment: environment(options),
        timeout: Duration::from_secs(120),
        maximum_stdout_bytes: MAXIMUM_OUTPUT_BYTES,
        maximum_stderr_bytes: MAXIMUM_OUTPUT_BYTES,
        stdout_path: root(options).join(format!("{name}.stdout.log")),
        stderr_path: root(options).join(format!("{name}.stderr.log")),
        unavailable_exit_code: None,
    }
}
fn observe(
    options: &PairOptions,
    current: &archive::VerifiedArchive,
    name: &str,
    control: &process::ProcessControl,
) -> Result<Command, DevError> {
    require(!control.cancelled(), "continuity cancelled")?;
    let arguments = arguments(options, current, name)?;
    let process = process::run_supervised(
        &spec(options, name, arguments.clone()),
        &root(options),
        Some(control),
    );
    let command = Command {
        name: name.to_owned(),
        arguments,
        environment: environment(options),
        process,
    };
    evidence::publish_json(
        &root(options).join(format!("{name}.process.json")),
        &command,
    )?;
    validate_command(options, current, name, &command)?;
    Ok(command)
}
fn validate_command(
    options: &PairOptions,
    current: &archive::VerifiedArchive,
    name: &str,
    command: &Command,
) -> Result<(), DevError> {
    let p = &command.process;
    require(
        command.name == name
            && command.arguments == arguments(options, current, name)?
            && command.environment == environment(options)
            && p.status == process::ProcessStatus::Passed
            && p.exit_code == Some(0)
            && p.signal.is_none()
            && p.reason.is_none()
            && !p.stdout_limit_exhausted
            && !p.stderr_limit_exhausted
            && p.stdout_limit_bytes == MAXIMUM_OUTPUT_BYTES
            && p.stderr_limit_bytes == MAXIMUM_OUTPUT_BYTES
            && process::read_bounded(
                &root(options).join(format!("{name}.process.json")),
                MAXIMUM_RECEIPT_BYTES,
            )? == evidence::encode_json(command)?,
        "continuity command was redirected, failed, exhausted or interrupted",
    )?;
    for (proof, suffix) in [(&p.stdout, "stdout"), (&p.stderr, "stderr")] {
        let path = format!("{name}.{suffix}.log");
        require(
            proof.path == path && *proof == evidence::proof(&root(options).join(&path), path)?,
            "continuity original command output changed",
        )?;
    }
    Ok(())
}
fn version(options: &PairOptions, name: &str) -> Result<String, DevError> {
    let bytes = process::read_bounded(
        &root(options).join(format!("{name}-version.stdout.log")),
        MAXIMUM_OUTPUT_BYTES,
    )?;
    String::from_utf8(bytes)
        .map_err(|_| DevError::corrupt("continuity tool version is not UTF-8"))?
        .lines()
        .next()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| DevError::corrupt("continuity tool version missing"))
}
fn derived_manifest(
    options: &PairOptions,
    current: &archive::VerifiedArchive,
) -> Result<ReleaseManifest, DevError> {
    let mut manifest = current.manifest.clone();
    manifest.product.version = alternate_tag(options).trim_start_matches('v').to_owned();
    manifest.source.expected_release_tag = alternate_tag(options).to_owned();
    manifest.packaging.tar_version = version(options, "tar")?;
    manifest.packaging.gzip_version = version(options, "gzip")?;
    Ok(manifest)
}
fn retained(options: &PairOptions) -> Result<Vec<FileBinding>, DevError> {
    let mut files = Vec::new();
    for tag in [options.tag.as_str(), alternate_tag(options)] {
        let slot = installation::prefix(options, Route::Exact)
            .join("lib/lkjscript/versions")
            .join(tag)
            .join(target::TARGET_TRIPLE);
        for name in [
            "lkjscript",
            "LICENSE",
            "THIRD-PARTY-LICENSES.html",
            "RELEASE-MANIFEST.json",
            "INSTALL-RECEIPT.json",
        ] {
            files.push(binding(&slot.join(name))?);
        }
    }
    for name in ["service.deployment.json", "generated/application.lkja"] {
        files.push(binding(&root(options).join("application").join(name))?);
    }
    Ok(files)
}
fn ready_address(line: &[u8]) -> Result<SocketAddr, DevError> {
    let event: serde_json::Value = serde_json::from_slice(line)?;
    require(
        event["event"] == "ready"
            && event["ok"] == true
            && event["deployment"]["runner"] == "http"
            && event["deployment"]["listen"] == "127.0.0.1:0",
        "continuity resident readiness changed",
    )?;
    let address: SocketAddr = event["local_address"]
        .as_str()
        .ok_or_else(|| DevError::corrupt("continuity address absent"))?
        .parse()
        .map_err(|_| DevError::corrupt("continuity address malformed"))?;
    require(
        address.ip().is_loopback() && address.port() != 0,
        "continuity listener is not private",
    )?;
    Ok(address)
}
fn validate_response(bytes: &[u8]) -> Result<(), DevError> {
    require(
        bytes.len() <= 65536
            && bytes.starts_with(b"HTTP/1.1 200 OK\r\n")
            && bytes.ends_with(b"\r\n\r\nhello from lkjscript"),
        "continuity starter HTTP result differs",
    )
}
fn request(
    options: &PairOptions,
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
    let path = root(options).join(format!("response-{ordinal}.http"));
    archive::write_new(&path, &bytes, 0o644)?;
    binding(&path)
}
struct Active {
    control: process::ProcessControl,
    receiver: mpsc::Receiver<Result<process::ProcessObservation, DevError>>,
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
    fn start(options: &PairOptions, current: &archive::VerifiedArchive) -> Result<Self, DevError> {
        let spec = spec(
            options,
            "resident",
            arguments(options, current, "resident")?,
        );
        let control = process::ProcessControl::default();
        let child_control = control.clone();
        let root = root(options);
        let (sender, receiver) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("current-selector-resident".to_owned())
            .spawn(move || {
                let result = process::run_controlled(&spec, &root, &child_control);
                let recorded = evidence::publish_json(
                    &root.join("resident.process.json"),
                    &Command {
                        name: "resident".to_owned(),
                        arguments: spec.command,
                        environment: spec.environment,
                        process: result.clone(),
                    },
                );
                let _ = sender.send(recorded.map(|_| result));
            })?;
        Ok(Self {
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
        let path = root(options).join("resident.stdout.log");
        loop {
            require(
                !control.cancelled() && started.elapsed() < Duration::from_secs(30),
                "continuity readiness cancelled or timed out",
            )?;
            if path.is_file() {
                let bytes = process::read_bounded(&path, MAXIMUM_OUTPUT_BYTES)?;
                if let Some(line) = bytes
                    .split_inclusive(|byte| *byte == b'\n')
                    .find(|line| line.ends_with(b"\n"))
                {
                    return ready_address(line);
                }
            }
            require(
                matches!(self.receiver.try_recv(), Err(mpsc::TryRecvError::Empty)),
                "continuity resident exited before readiness",
            )?;
            thread::sleep(Duration::from_millis(20));
        }
    }
    fn finish(
        mut self,
        options: &PairOptions,
        current: &archive::VerifiedArchive,
    ) -> Result<Command, DevError> {
        self.control.interrupt();
        let process = self
            .receiver
            .recv_timeout(Duration::from_secs(30))
            .map_err(|error| DevError::infrastructure(format!("continuity shutdown: {error}")))??;
        if let Some(thread) = self.thread.take() {
            thread
                .join()
                .map_err(|_| DevError::infrastructure("continuity supervisor failed"))?;
        }
        let result = Command {
            name: "resident".to_owned(),
            arguments: arguments(options, current, "resident")?,
            environment: environment(options),
            process,
        };
        validate_command(options, current, "resident", &result)?;
        Ok(result)
    }
}
pub(super) fn run(
    options: &PairOptions,
    current: &archive::VerifiedArchive,
    control: &process::ProcessControl,
) -> Result<Continuity, DevError> {
    let root = root(options);
    fs::create_dir(&root)?;
    fs::create_dir(root.join("runtime"))?;
    fs::create_dir(root.join("stage"))?;
    fs::create_dir_all(root.join("application/generated"))?;
    let outcome: Result<Continuity, DevError> = (|| {
        selected(options, &options.tag)?;
        let original_executable = binding(&manager(options))?;
        let mut commands = Vec::new();
        for name in &COMMANDS[..2] {
            commands.push(observe(options, current, name, control)?);
        }
        let extraction = Route::Exact.extraction(options);
        archive::stage_payload(
            &root.join("stage/lkjscript"),
            &extraction.join("lkjscript"),
            &extraction.join("LICENSE"),
            &extraction.join("THIRD-PARTY-LICENSES.html"),
            &archive::canonical_json(&derived_manifest(options, current)?)?,
        )?;
        for name in &COMMANDS[2..5] {
            commands.push(observe(options, current, name, control)?);
        }
        selected(options, &options.tag)?;
        let admission = archive::verify_archive(
            &root.join("alternate.tar.gz"),
            &root,
            Some(&manager(options)),
        )?;
        require(
            admission.manifest == derived_manifest(options, current)?,
            "continuity derived container changed",
        )?;
        for name in &COMMANDS[5..8] {
            commands.push(observe(options, current, name, control)?);
        }
        archive::copy_new(
            &root.join("runtime/project/service.deployment.json"),
            &root.join("application/service.deployment.json"),
            0o644,
        )?;
        fs::remove_dir_all(root.join("runtime/project"))?;
        let unchanged = retained(options)?;
        let resident = Active::start(options, current)?;
        let address = resident.ready(options, control)?;
        let before = request(options, 0, address)?;
        commands.push(observe(options, current, "select-alternate", control)?);
        selected(options, alternate_tag(options))?;
        let switched = request(options, 1, address)?;
        require(
            retained(options)? == unchanged,
            "selector switch changed application or installed bytes",
        )?;
        commands.push(observe(options, current, "select-current", control)?);
        selected(options, &options.tag)?;
        let restored = request(options, 2, address)?;
        let resident = resident.finish(options, current)?;
        require(
            !control.cancelled()
                && retained(options)? == unchanged
                && binding(&manager(options))? == original_executable,
            "continuity changed retained state or was cancelled",
        )?;
        Ok(Continuity {
            fixture: "same-binary-current-format-two-tags".to_owned(),
            archive: binding(&root.join("alternate.tar.gz"))?,
            admission,
            commands,
            resident,
            original_executable,
            responses: [before, switched, restored],
            retained: unchanged,
            cleanup_complete: false,
        })
    })();
    let cleanup = fs::remove_dir_all(root.join("runtime"));
    let mut receipt = match (outcome, cleanup) {
        (Ok(receipt), Ok(())) => receipt,
        (Err(error), Ok(())) => return Err(error),
        (Ok(_), Err(error)) => return Err(error.into()),
        (Err(error), Err(cleanup)) => {
            return Err(DevError::infrastructure(format!(
                "{error}; continuity cleanup also failed: {cleanup}"
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
    receipt: &Continuity,
) -> Result<(), DevError> {
    let root = root(options);
    selected(options, &options.tag)?;
    for (tag, admission) in [
        (options.tag.as_str(), current),
        (alternate_tag(options), &receipt.admission),
    ] {
        let mut slot = options.clone();
        slot.tag = tag.to_owned();
        installation::payloads(&slot, Route::Exact, admission)?;
    }
    require(
        receipt.fixture == "same-binary-current-format-two-tags"
            && receipt.cleanup_complete
            && !root.join("runtime").try_exists()?
            && receipt.commands.len() == COMMANDS.len()
            && receipt.archive == binding(&root.join("alternate.tar.gz"))?
            && receipt.admission.manifest == derived_manifest(options, current)?
            && receipt.admission
                == archive::verify_archive(
                    &root.join("alternate.tar.gz"),
                    &root,
                    Some(&manager(options)),
                )?
            && receipt.original_executable == binding(&manager(options))?
            && receipt.retained == retained(options)?
            && process::read_bounded(&root.join("receipt.json"), MAXIMUM_RECEIPT_BYTES)?
                == evidence::encode_json(receipt)?,
        "continuity evidence changed or is incomplete",
    )?;
    for (command, name) in receipt.commands.iter().zip(COMMANDS) {
        validate_command(options, current, name, command)?;
    }
    validate_command(options, current, "resident", &receipt.resident)?;
    let stdout = process::read_bounded(&root.join("resident.stdout.log"), MAXIMUM_OUTPUT_BYTES)?;
    let mut lines = stdout
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty());
    ready_address(
        lines
            .next()
            .ok_or_else(|| DevError::corrupt("continuity ready event missing"))?,
    )?;
    let stopped: serde_json::Value = serde_json::from_slice(
        lines
            .next()
            .ok_or_else(|| DevError::corrupt("continuity stopped event missing"))?,
    )?;
    require(
        lines.next().is_none()
            && stopped["event"] == "stopped"
            && stopped["ok"] == true
            && stopped["receipt"]["shutdown"]["admission_stopped"] == true
            && stopped["receipt"]["shutdown"]["remaining_tasks"] == 0
            && stopped["receipt"]["shutdown"]["cleanup_failures"] == serde_json::json!([]),
        "continuity resident cleanup failed",
    )?;
    for (ordinal, proof) in receipt.responses.iter().enumerate() {
        let path = root.join(format!("response-{ordinal}.http"));
        require(*proof == binding(&path)?, "continuity response changed")?;
        validate_response(&process::read_bounded(&path, 65536)?)?;
    }
    for (name, tag) in [
        ("install", options.tag.as_str()),
        ("select-alternate", alternate_tag(options)),
        ("select-current", options.tag.as_str()),
    ] {
        let output = process::read_bounded(
            &root.join(format!("{name}.stdout.log")),
            MAXIMUM_OUTPUT_BYTES,
        )?;
        let records = super::super::super::admission::compact(name, &output)?;
        require(
            lifecycle::value(&records, "selection", "tag")? == tag,
            "continuity command did not report its selected tag",
        )?;
    }
    let output = process::read_bounded(&root.join("check.stdout.log"), MAXIMUM_OUTPUT_BYTES)?;
    let records = super::super::super::admission::compact("continuity-check", &output)?;
    require(
        lifecycle::value(&records, "tests", "failed")? == "0"
            && lifecycle::value(&records, "tests", "differential")? == "equal",
        "continuity application check failed",
    )
}
