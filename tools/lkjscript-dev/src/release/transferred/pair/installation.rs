//! Installed-route proof within this pair. Original script bytes are always executed unchanged.
use super::*;
use crate::release::admission;
use lkjscript::platform::control::CompactRecord;
use std::ffi::OsStr;
use std::os::unix::fs::MetadataExt;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum Acquisition {
    Simulated,
    Anonymous,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Installation {
    acquisition: Acquisition,
    script: FileBinding,
    archive_url: String,
    command: Vec<String>,
    environment: BTreeMap<String, String>,
    process: process::ProcessObservation,
    cleanup: Vec<CleanupCommand>,
    container_id: Option<FileBinding>,
    installed: Vec<FileBinding>,
    receipt: FileBinding,
    pub(super) candidate: FileBinding,
    pointer: String,
    fixture_requests: Option<FileBinding>,
    pub(super) cleanup_complete: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CleanupCommand {
    name: String,
    command: Vec<String>,
    process: process::ProcessObservation,
}
pub(super) fn prefix(options: &PairOptions, route: Route) -> PathBuf {
    route.root(options).join("installation")
}
pub(super) fn candidate(options: &PairOptions, route: Route) -> PathBuf {
    prefix(options, route)
        .join("lib/lkjscript/versions")
        .join(&options.tag)
        .join(target::TARGET_TRIPLE)
        .join("lkjscript")
}
fn archive_url(options: &PairOptions) -> String {
    format!(
        "https://github.com/lkjsxc/lkjscript/releases/download/{}/{}",
        options.tag,
        archive::ARCHIVE_NAME
    )
}
fn role(route: Route) -> &'static str {
    match route {
        Route::Exact => "musl",
        Route::Latest => "older-glibc",
    }
}
fn image(route: Route) -> &'static str {
    match route {
        Route::Exact => target::MUSL_USERLAND_IMAGE,
        Route::Latest => target::OLDER_GLIBC_USERLAND_IMAGE,
    }
}
fn pinned_userland(options: &PairOptions) -> bool {
    options.tier == Tier::CandidateInstallation
}
fn command(options: &PairOptions, route: Route) -> Result<Vec<String>, DevError> {
    let script = route
        .assets(options)
        .join(super::super::super::bootstrap::NAME);
    let mut args = if !pinned_userland(options) {
        vec!["/bin/sh".to_owned()]
    } else {
        let uid = fs::metadata(route.root(options))?.uid();
        let gid = fs::metadata(route.root(options))?.gid();
        let owned = admission::read_owned_container(&route.root(options), role(route))?
            .ok_or_else(|| DevError::corrupt("bootstrap container ownership intent missing"))?;
        let mut command = vec!["docker".to_owned(), "run".to_owned()];
        command.extend(admission::owned_container_options(
            &route.root(options),
            &owned,
        ));
        command.extend([
            "--platform".to_owned(),
            "linux/amd64".to_owned(),
            "--network".to_owned(),
            "none".to_owned(),
            "--user".to_owned(),
            format!("{uid}:{gid}"),
            "--volume".to_owned(),
            format!(
                "{}:{}",
                options.evidence_root.display(),
                options.evidence_root.display()
            ),
            "--volume".to_owned(),
            format!(
                "{}:{}:ro",
                route.assets(options).display(),
                route.assets(options).display()
            ),
            "--env".to_owned(),
            format!(
                "PATH={}/acquisition-fixture:/usr/bin:/bin",
                route.root(options).display()
            ),
            "--env".to_owned(),
            format!("TMPDIR={}/runtime", route.root(options).display()),
            "--env".to_owned(),
            format!(
                "LKJSCRIPT_FIXTURE_ARCHIVE={}",
                route.assets(options).join(archive::ARCHIVE_NAME).display()
            ),
            "--env".to_owned(),
            format!("LKJSCRIPT_FIXTURE_URL={}", archive_url(options)),
            "--env".to_owned(),
            format!(
                "LKJSCRIPT_FIXTURE_LOG={}/acquisition-requests.txt",
                route.root(options).display()
            ),
            "--entrypoint".to_owned(),
            "/bin/sh".to_owned(),
            image(route).to_owned(),
        ]);
        command
    };
    args.extend([
        script.display().to_string(),
        "--prefix".to_owned(),
        prefix(options, route).display().to_string(),
    ]);
    Ok(args)
}
fn environment(options: &PairOptions, route: Route) -> BTreeMap<String, String> {
    let mut values = BTreeMap::from([
        ("LANG".to_owned(), "C".to_owned()),
        ("PATH".to_owned(), "/usr/bin:/bin".to_owned()),
        (
            "HOME".to_owned(),
            route.root(options).join("runtime").display().to_string(),
        ),
        (
            "TMPDIR".to_owned(),
            route.root(options).join("runtime").display().to_string(),
        ),
    ]);
    if options.acquisition == Acquisition::Simulated && !pinned_userland(options) {
        let root = route.root(options);
        values.insert(
            "PATH".to_owned(),
            format!(
                "{}:/usr/bin:/bin",
                root.join("acquisition-fixture").display()
            ),
        );
        values.insert(
            "LKJSCRIPT_FIXTURE_ARCHIVE".to_owned(),
            route
                .assets(options)
                .join(archive::ARCHIVE_NAME)
                .display()
                .to_string(),
        );
        values.insert("LKJSCRIPT_FIXTURE_URL".to_owned(), archive_url(options));
        values.insert(
            "LKJSCRIPT_FIXTURE_LOG".to_owned(),
            root.join("acquisition-requests.txt").display().to_string(),
        );
    }
    values
}
const CURL_FIXTURE: &str = "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$@\" > \"$LKJSCRIPT_FIXTURE_LOG\"\noutput=''\nurl=''\nwhile [ \"$#\" -gt 0 ]; do\n case \"$1\" in --output) output=$2; shift 2 ;; *) url=$1; shift ;; esac\ndone\n[ \"$url\" = \"$LKJSCRIPT_FIXTURE_URL\" ]\n[ -n \"$output\" ]\ncp \"$LKJSCRIPT_FIXTURE_ARCHIVE\" \"$output\"\n";

pub(super) fn run(
    options: &PairOptions,
    route: Route,
    admitted: &archive::VerifiedArchive,
    control: &process::ProcessControl,
) -> Result<Installation, DevError> {
    let root = route.root(options);
    let script = route
        .assets(options)
        .join(super::super::super::bootstrap::NAME);
    super::super::super::bootstrap::verify(&script, admitted)?;
    if options.acquisition == Acquisition::Simulated {
        let fixture = root.join("acquisition-fixture");
        fs::create_dir(&fixture)?;
        archive::write_new(&fixture.join("curl"), CURL_FIXTURE.as_bytes(), 0o755)?;
        // Latest has moved in the simulated origin after these script bytes were acquired.
        archive::write_new(&fixture.join("latest-tag"), b"v999.0.0\n", 0o644)?;
    }
    let owned = if pinned_userland(options) {
        Some(admission::prepare_owned_container(&root, role(route))?)
    } else {
        None
    };
    let command = command(options, route)?;
    let environment = environment(options, route);
    let spec = process::ProcessSpec {
        command: command.clone(),
        cwd: root.join("runtime"),
        environment: environment.clone(),
        timeout: Duration::from_secs(600),
        maximum_stdout_bytes: MAXIMUM_OUTPUT_BYTES,
        maximum_stderr_bytes: MAXIMUM_OUTPUT_BYTES,
        stdout_path: root.join("bootstrap.stdout.log"),
        stderr_path: root.join("bootstrap.stderr.log"),
        unavailable_exit_code: None,
    };
    let observation = process::run_supervised(&spec, &root, Some(control));
    let mut cleanup = Vec::new();
    let cleanup_result = owned.as_ref().map_or(Ok(()), |owned| {
        admission::cleanup_owned_container(&root, owned, |name, command| {
            let spec = process::ProcessSpec {
                command: command.clone(),
                cwd: root.clone(),
                environment: environment.clone(),
                timeout: Duration::from_secs(30),
                maximum_stdout_bytes: MAXIMUM_OUTPUT_BYTES,
                maximum_stderr_bytes: MAXIMUM_OUTPUT_BYTES,
                stdout_path: root.join(format!("{name}.stdout.log")),
                stderr_path: root.join(format!("{name}.stderr.log")),
                unavailable_exit_code: None,
            };
            let observation = process::run(&spec, &root);
            let observed = CleanupCommand {
                name: name.to_owned(),
                command,
                process: observation,
            };
            let output = cleanup_output(&root, &observed);
            cleanup.push(observed);
            output
        })
    });
    // Publish original cleanup observations even when bootstrap failed or was cancelled.
    evidence::publish_json(&root.join("bootstrap-cleanup.json"), &cleanup)?;
    let cleanup_complete = cleanup_result.is_ok();
    cleanup_result?;
    let container_id = if let Some(owned) = &owned {
        require(
            admission::owned_container_cid(&root, owned)?.is_some(),
            "successful bootstrap omitted its complete container identity",
        )?;
        Some(binding(
            &root.join(format!("{}-container.cid", role(route))),
        )?)
    } else {
        None
    };
    require(
        observation.status == process::ProcessStatus::Passed && cleanup_complete,
        "bootstrap execution or owned container cleanup failed; retain logs and owned prefix",
    )?;
    let installed = payloads(options, route, admitted)?;
    let candidate = binding(&candidate(options, route))?;
    let receipt = binding(&Path::new(&candidate.file.path).with_file_name("INSTALL-RECEIPT.json"))?;
    let pointer = fs::read_link(prefix(options, route).join("bin/lkjscript"))?
        .display()
        .to_string();
    let proof = Installation {
        acquisition: options.acquisition,
        script: binding(&script)?,
        archive_url: archive_url(options),
        command,
        environment,
        process: observation,
        cleanup,
        container_id,
        installed,
        receipt,
        candidate,
        pointer,
        fixture_requests: if options.acquisition == Acquisition::Simulated {
            Some(binding(&root.join("acquisition-requests.txt"))?)
        } else {
            None
        },
        cleanup_complete,
    };
    validate(options, route, admitted, &proof)?;
    Ok(proof)
}
fn payloads(
    options: &PairOptions,
    route: Route,
    admitted: &archive::VerifiedArchive,
) -> Result<Vec<FileBinding>, DevError> {
    let executable = candidate(options, route);
    let parent = executable
        .parent()
        .ok_or_else(|| DevError::corrupt("installed executable has no parent"))?;
    let mut payloads = Vec::new();
    for member in admitted.members.iter().skip(1) {
        let name = member
            .name
            .strip_prefix("lkjscript/")
            .ok_or_else(|| DevError::corrupt("foreign installed payload"))?;
        let observed = binding(&parent.join(name))?;
        require(
            observed.mode == member.mode
                && observed.file.byte_length == member.byte_length
                && Some(&observed.file.sha256) == member.sha256.as_ref(),
            "installed payload differs from independently admitted archive bytes",
        )?;
        payloads.push(observed);
    }
    require(
        target::inspect_static_elf(&executable)? == admitted.manifest.executable.elf,
        "installed static target differs",
    )?;
    Ok(payloads)
}
fn cleanup_output(root: &Path, observed: &CleanupCommand) -> Result<Vec<u8>, DevError> {
    let p = &observed.process;
    require(
        p.status == process::ProcessStatus::Passed
            && p.exit_code == Some(0)
            && p.signal.is_none()
            && p.reason.is_none()
            && !p.stdout_limit_exhausted
            && !p.stderr_limit_exhausted
            && p.stdout_limit_bytes == MAXIMUM_OUTPUT_BYTES
            && p.stderr_limit_bytes == MAXIMUM_OUTPUT_BYTES,
        "owned bootstrap cleanup failed, was interrupted or exhausted",
    )?;
    for (proof, suffix) in [(&p.stdout, "stdout"), (&p.stderr, "stderr")] {
        let name = format!("{}.{}.log", observed.name, suffix);
        regular(&root.join(&name))?;
        require(
            proof.path == name && *proof == evidence::proof(&root.join(&name), name.clone())?,
            "owned bootstrap cleanup original log changed",
        )?;
    }
    process::read_bounded(
        &root.join(format!("{}.stdout.log", observed.name)),
        MAXIMUM_OUTPUT_BYTES,
    )
}
pub(super) fn validate(
    options: &PairOptions,
    route: Route,
    admitted: &archive::VerifiedArchive,
    receipt: &Installation,
) -> Result<(), DevError> {
    let root = route.root(options);
    super::super::super::bootstrap::verify(
        &route
            .assets(options)
            .join(super::super::super::bootstrap::NAME),
        admitted,
    )?;
    let p = &receipt.process;
    require(
        receipt.acquisition == options.acquisition
            && receipt.archive_url == archive_url(options)
            && receipt.command == command(options, route)?
            && receipt.environment == environment(options, route)
            && receipt.script
                == binding(
                    &route
                        .assets(options)
                        .join(super::super::super::bootstrap::NAME),
                )?
            && p.status == process::ProcessStatus::Passed
            && p.exit_code == Some(0)
            && p.signal.is_none()
            && p.reason.is_none()
            && !p.stdout_limit_exhausted
            && !p.stderr_limit_exhausted
            && receipt.cleanup_complete
            && receipt.installed == payloads(options, route, admitted)?
            && receipt.candidate == binding(&candidate(options, route))?
            && receipt.receipt
                == binding(&candidate(options, route).with_file_name("INSTALL-RECEIPT.json"))?,
        "installed bootstrap receipt binding mismatch",
    )?;
    let pointer = format!(
        "../lib/lkjscript/versions/{}/{}/lkjscript",
        options.tag,
        target::TARGET_TRIPLE
    );
    require(
        receipt.pointer == pointer
            && fs::read_link(prefix(options, route).join("bin/lkjscript"))? == Path::new(&pointer),
        "installed selection pointer mismatch",
    )?;
    for (proof, name) in [
        (&p.stdout, "bootstrap.stdout.log"),
        (&p.stderr, "bootstrap.stderr.log"),
    ] {
        regular(&root.join(name))?;
        require(
            proof.path == name && *proof == evidence::proof(&root.join(name), name.to_owned())?,
            "bootstrap output log binding mismatch",
        )?;
    }
    let stdout = process::read_bounded(&root.join("bootstrap.stdout.log"), MAXIMUM_OUTPUT_BYTES)?;
    let records = super::super::super::admission::compact("bootstrap", &stdout)?;
    require(
        field(&records, "result", "status")? == "success"
            && field(&records, "selection", "tag")? == options.tag
            && field(&records, "runtime", "sha256")?
                == admitted.manifest.executable.sha256.as_str()
            && field(&records, "runtime", "path")?
                == candidate(options, route).display().to_string()
            && field(&records, "recovery", "retained-manager")? == "true"
            && field(&records, "recovery", "manager-path")?
                == candidate(options, route).display().to_string(),
        "bootstrap did not report its retained manager slot",
    )?;
    match options.acquisition {
        Acquisition::Anonymous => require(
            receipt.fixture_requests.is_none()
                && receipt.cleanup.is_empty()
                && receipt.container_id.is_none()
                && !root.join("acquisition-fixture").exists(),
            "public acquisition contains simulated inputs",
        )?,
        Acquisition::Simulated => {
            require(
                process::read_bounded(&root.join("acquisition-fixture/curl"), 16384)?
                    == CURL_FIXTURE.as_bytes(),
                "controlled acquisition fixture changed",
            )?;
            require(
                receipt.fixture_requests == Some(binding(&root.join("acquisition-requests.txt"))?)
                    && if pinned_userland(options) {
                        receipt.container_id
                            == Some(binding(
                                &root.join(format!("{}-container.cid", role(route))),
                            )?)
                            && !receipt.cleanup.is_empty()
                    } else {
                        receipt.container_id.is_none() && receipt.cleanup.is_empty()
                    },
                "simulated acquisition or cleanup observation missing",
            )?;
            let requested = String::from_utf8(process::read_bounded(
                &root.join("acquisition-requests.txt"),
                16384,
            )?)
            .map_err(|_| DevError::corrupt("invalid request log"))?;
            let arguments = requested.lines().collect::<Vec<_>>();
            let expected = [
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
            ];
            require(
                arguments.len() == 22
                    && arguments[..18] == expected
                    && arguments[18] == admitted.archive_byte_length.to_string()
                    && arguments[19] == "--output"
                    && arguments[21] == archive_url(options),
                "bootstrap acquisition command, bounds or immutable URL changed",
            )?;
            let output = Path::new(arguments[20]);
            let stage = output
                .parent()
                .ok_or_else(|| DevError::corrupt("fixture output parent missing"))?;
            require(
                stage.parent() == Some(root.join("runtime").as_path())
                    && stage.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                        n.strip_prefix("lkjscript-install.").is_some_and(|id| {
                            id.len() == 10 && id.bytes().all(|b| b.is_ascii_alphanumeric())
                        })
                    })
                    && output.file_name() == Some(OsStr::new("archive.tar.gz")),
                "bootstrap acquisition escaped its private temporary storage",
            )?;
            if !pinned_userland(options) {
                return Ok(());
            }
            let owned = admission::read_owned_container(&root, role(route))?
                .ok_or_else(|| DevError::corrupt("bootstrap ownership intent missing"))?;
            let mut observations = receipt.cleanup.iter();
            admission::cleanup_owned_container(&root, &owned, |name, command| {
                let observed = observations
                    .next()
                    .ok_or_else(|| DevError::corrupt("owned cleanup observation missing"))?;
                require(
                    observed.name == name && observed.command == command,
                    "owned cleanup command was redirected or skipped",
                )?;
                cleanup_output(&root, observed)
            })?;
            require(
                observations.next().is_none(),
                "unexpected extra owned cleanup command",
            )?;
            require(
                process::read_bounded(&root.join("bootstrap-cleanup.json"), MAXIMUM_RECEIPT_BYTES)?
                    == evidence::encode_json(&receipt.cleanup)?,
                "bootstrap cleanup original receipt changed",
            )?;
        }
    }
    Ok(())
}
fn field<'a>(
    records: &'a [CompactRecord],
    operation: &str,
    key: &str,
) -> Result<&'a str, DevError> {
    lifecycle::value(records, operation, key)
}
