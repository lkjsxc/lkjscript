//! Authentic legacy-manager rejection and recovery through the unchanged new bootstrap.
use super::*;

const TAG: &str = "v0.1.35";
const SOURCE: &str = "4306ef64783462a8dda48f2a855a88482d1faec8";
const SHA: &str = "e68cabfcb729b510cae4a599a342b7e0a5131224086ed4c8ea62bbb82f417f92";
const BYTES: u64 = 10_491_770;
const URL: &str = "https://github.com/lkjsxc/lkjscript/releases/download/v0.1.35/lkjscript-x86_64-unknown-linux-musl.tar.gz";
const ACQUIRE: &str = "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$@\" > \"$FIXTURE_LOG\"\noutput=''\nurl=''\nwhile [ \"$#\" -gt 0 ]; do\n case \"$1\" in --output) output=$2; shift 2 ;; *) url=$1; shift ;; esac\ndone\n[ \"$url\" = \"$FIXTURE_URL\" ]\ncp \"$FIXTURE_ARCHIVE\" \"$output\"\n";

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
pub(super) struct LegacyUpgrade {
    legacy_archive: FileBinding,
    current_archive: FileBinding,
    current_installer: FileBinding,
    commands: Vec<Command>,
    legacy_payloads_before: Vec<FileBinding>,
    legacy_payloads_after: Vec<FileBinding>,
    selected_after_rejection: String,
    selected_after_recovery: String,
    cleanup_complete: bool,
}

fn root(options: &PairOptions) -> PathBuf {
    options.evidence_root.join("legacy-manager")
}
fn prefix(options: &PairOptions) -> PathBuf {
    root(options).join("prefix")
}
fn slot(options: &PairOptions, tag: &str) -> PathBuf {
    prefix(options)
        .join("lib/lkjscript/versions")
        .join(tag)
        .join(target::TARGET_TRIPLE)
}
fn pointer(tag: &str) -> String {
    format!(
        "../lib/lkjscript/versions/{tag}/{}/lkjscript",
        target::TARGET_TRIPLE
    )
}
fn selected(options: &PairOptions) -> Result<String, DevError> {
    Ok(fs::read_link(prefix(options).join("bin/lkjscript"))?
        .display()
        .to_string())
}
fn archive_path(options: &PairOptions) -> PathBuf {
    root(options).join("legacy.tar.gz")
}
fn current(options: &PairOptions) -> PathBuf {
    options.exact_assets.join(archive::ARCHIVE_NAME)
}
fn installer(options: &PairOptions) -> PathBuf {
    options
        .exact_assets
        .join(super::super::super::bootstrap::NAME)
}

fn command(options: &PairOptions, index: usize) -> Result<(String, Vec<String>, bool), DevError> {
    let legacy = archive_path(options);
    let current = current(options);
    let (digest, _) = archive::sha256_file(&current)?;
    let prefix = prefix(options);
    let old = slot(options, TAG).join("lkjscript");
    let manager = installation::candidate(options, Route::Exact);
    let selected = prefix.join("bin/lkjscript");
    let script = installer(options);
    let (name, arguments, success) = match index {
        0 => (
            "acquire",
            vec![
                "/usr/bin/curl".to_owned(),
                "-q".to_owned(),
                "--fail".to_owned(),
                "--location".to_owned(),
                "--silent".to_owned(),
                "--show-error".to_owned(),
                "--proto".to_owned(),
                "=https".to_owned(),
                "--proto-redir".to_owned(),
                "=https".to_owned(),
                "--connect-timeout".to_owned(),
                "15".to_owned(),
                "--max-time".to_owned(),
                "180".to_owned(),
                "--max-filesize".to_owned(),
                BYTES.to_string(),
                "--output".to_owned(),
                legacy.display().to_string(),
                URL.to_owned(),
            ],
            true,
        ),
        1 | 2 => (
            if index == 1 {
                "install-legacy"
            } else {
                "reject-neutral"
            },
            vec![
                if index == 1 {
                    manager.display().to_string()
                } else {
                    old.display().to_string()
                },
                "runtime".to_owned(),
                "install".to_owned(),
                "--archive".to_owned(),
                if index == 1 {
                    legacy.display().to_string()
                } else {
                    current.display().to_string()
                },
                "--sha256".to_owned(),
                if index == 1 {
                    SHA.to_owned()
                } else {
                    digest.as_str().to_owned()
                },
                "--prefix".to_owned(),
                prefix.display().to_string(),
                "--activate".to_owned(),
            ],
            index == 1,
        ),
        3 => (
            "legacy-still-usable",
            vec![selected.display().to_string(), "--version".to_owned()],
            true,
        ),
        4 => (
            "bootstrap-recovery",
            vec![
                "/bin/sh".to_owned(),
                script.display().to_string(),
                "--prefix".to_owned(),
                prefix.display().to_string(),
            ],
            true,
        ),
        5 => (
            "new-selected",
            vec![selected.display().to_string(), "--version".to_owned()],
            true,
        ),
        _ => return Err(DevError::corrupt("unknown legacy recovery command")),
    };
    Ok((name.to_owned(), arguments, success))
}

fn environment(options: &PairOptions, index: usize) -> BTreeMap<String, String> {
    let root = root(options);
    let mut values = BTreeMap::from([
        ("LANG".to_owned(), "C".to_owned()),
        ("PATH".to_owned(), "/usr/bin:/bin".to_owned()),
        (
            "HOME".to_owned(),
            root.join("runtime").display().to_string(),
        ),
        (
            "TMPDIR".to_owned(),
            root.join("runtime").display().to_string(),
        ),
    ]);
    if index == 4 {
        values.insert(
            "PATH".to_owned(),
            format!("{}:/usr/bin:/bin", root.join("fixture").display()),
        );
        values.insert(
            "FIXTURE_ARCHIVE".to_owned(),
            current(options).display().to_string(),
        );
        values.insert(
            "FIXTURE_URL".to_owned(),
            format!(
                "https://github.com/lkjsxc/lkjscript/releases/download/{}/{}",
                options.tag,
                archive::ARCHIVE_NAME
            ),
        );
        values.insert(
            "FIXTURE_LOG".to_owned(),
            root.join("requests.log").display().to_string(),
        );
    }
    values
}

fn admit_legacy(options: &PairOptions) -> Result<(), DevError> {
    let bytes = process::read_bounded(&archive_path(options), BYTES)?;
    require(
        bytes.len() as u64 == BYTES && archive::sha256_bytes(&bytes)?.as_str() == SHA,
        "legacy manager acquisition differs from immutable expected bytes",
    )?;
    let admitted = lkjscript::release_container::admit_gzip(&bytes)?.verified;
    require(
        admitted.manifest.source.expected_release_tag == TAG
            && admitted.manifest.source.tagged_commit_sha == SOURCE
            && admitted.manifest.legacy_publication_mode() == Some(PublicationMode::Release),
        "legacy manager source/tag differs",
    )
}

fn payloads(options: &PairOptions) -> Result<Vec<FileBinding>, DevError> {
    [
        "lkjscript",
        "LICENSE",
        "THIRD-PARTY-LICENSES.html",
        "RELEASE-MANIFEST.json",
        "INSTALL-RECEIPT.json",
    ]
    .into_iter()
    .map(|name| binding(&slot(options, TAG).join(name)))
    .collect()
}

fn validate_command(
    options: &PairOptions,
    index: usize,
    observed: &Command,
) -> Result<(), DevError> {
    let (name, arguments, success) = command(options, index)?;
    let p = &observed.process;
    require(
        observed.name == name
            && observed.arguments == arguments
            && observed.environment == environment(options, index)
            && p.signal.is_none()
            && !p.stdout_limit_exhausted
            && !p.stderr_limit_exhausted
            && p.stdout_limit_bytes == MAXIMUM_OUTPUT_BYTES
            && p.stderr_limit_bytes == MAXIMUM_OUTPUT_BYTES
            && if success {
                p.status == process::ProcessStatus::Passed
                    && p.exit_code == Some(0)
                    && p.reason.is_none()
            } else {
                p.status == process::ProcessStatus::Failed
                    && p.exit_code.is_some_and(|code| code != 0)
                    && p.reason.as_deref() == Some("nonzero_exit")
            },
        "legacy recovery command was changed, skipped or failed its expected disposition",
    )?;
    for (proof, suffix) in [(&p.stdout, "stdout"), (&p.stderr, "stderr")] {
        let name = format!("{index}-{suffix}.log");
        require(
            proof.path == name && *proof == evidence::proof(&root(options).join(&name), name)?,
            "legacy recovery original logs changed",
        )?;
    }
    if index == 3 || index == 5 {
        let version = if index == 3 {
            "0.1.35"
        } else {
            lkjscript::PRODUCT_VERSION
        };
        require(
            process::read_bounded(&root(options).join(&p.stdout.path), 4096)?
                == format!("lkjscript {version}\n").as_bytes(),
            "legacy rejection/recovery selected the wrong product",
        )?;
    }
    if index == 2 {
        let records = super::super::super::admission::compact(
            "legacy neutral rejection",
            &process::read_bounded(&root(options).join(&p.stdout.path), MAXIMUM_OUTPUT_BYTES)?,
        )?;
        require(
            lifecycle::value(&records, "diagnostic", "code")? == "runtime_archive"
                && lifecycle::value(&records, "diagnostic", "message")?
                    .contains("unknown field `format`"),
            "legacy manager did not specifically reject the neutral format before installation",
        )?;
    }
    Ok(())
}

pub(super) fn run(
    options: &PairOptions,
    current_admission: &archive::VerifiedArchive,
    control: &process::ProcessControl,
) -> Result<LegacyUpgrade, DevError> {
    let root = root(options);
    fs::create_dir(&root)?;
    fs::create_dir(root.join("runtime"))?;
    fs::create_dir(root.join("fixture"))?;
    archive::write_new(&root.join("fixture/curl"), ACQUIRE.as_bytes(), 0o755)?;
    super::super::super::bootstrap::verify(&installer(options), current_admission)?;
    let outcome: Result<LegacyUpgrade, DevError> = (|| {
        let mut commands = Vec::new();
        let mut before = Vec::new();
        let mut rejected = String::new();
        for index in 0..6 {
            require(!control.cancelled(), "legacy recovery cancelled")?;
            let (name, arguments, _) = command(options, index)?;
            let environment = environment(options, index);
            let spec = process::ProcessSpec {
                command: arguments.clone(),
                cwd: root.join("runtime"),
                environment: environment.clone(),
                timeout: Duration::from_secs(300),
                maximum_stdout_bytes: MAXIMUM_OUTPUT_BYTES,
                maximum_stderr_bytes: MAXIMUM_OUTPUT_BYTES,
                stdout_path: root.join(format!("{index}-stdout.log")),
                stderr_path: root.join(format!("{index}-stderr.log")),
                unavailable_exit_code: None,
            };
            let observed = Command {
                name,
                arguments,
                environment,
                process: process::run_supervised(&spec, &root, Some(control)),
            };
            commands.push(observed);
            evidence::publish_json(&root.join("commands.json"), &commands)?;
            validate_command(options, index, &commands[index])?;
            match index {
                0 => admit_legacy(options)?,
                1 => {
                    before = payloads(options)?;
                    require(
                        selected(options)? == pointer(TAG),
                        "legacy manager not selected",
                    )?;
                }
                2 => {
                    rejected = selected(options)?;
                    require(
                        rejected == pointer(TAG)
                            && payloads(options)? == before
                            && !slot(options, &options.tag).exists(),
                        "old reader rejection changed the prior slot/selection",
                    )?;
                }
                _ => (),
            }
        }
        let after = payloads(options)?;
        require(
            before == after && selected(options)? == pointer(&options.tag),
            "new bootstrap did not recover without changing legacy slots",
        )?;
        Ok(LegacyUpgrade {
            legacy_archive: binding(&archive_path(options))?,
            current_archive: binding(&current(options))?,
            current_installer: binding(&installer(options))?,
            commands,
            legacy_payloads_before: before,
            legacy_payloads_after: after,
            selected_after_rejection: rejected,
            selected_after_recovery: selected(options)?,
            cleanup_complete: false,
        })
    })();
    let cleanup = fs::remove_dir_all(root.join("runtime"));
    let mut receipt = outcome?;
    cleanup?;
    require(
        !control.cancelled(),
        "legacy recovery cancelled during cleanup",
    )?;
    receipt.cleanup_complete = true;
    evidence::publish_json(&root.join("receipt.json"), &receipt)?;
    validate(options, &receipt)?;
    Ok(receipt)
}

pub(super) fn validate(options: &PairOptions, receipt: &LegacyUpgrade) -> Result<(), DevError> {
    admit_legacy(options)?;
    require(
        receipt.commands.len() == 6
            && receipt.cleanup_complete
            && !root(options).join("runtime").exists()
            && receipt.legacy_archive == binding(&archive_path(options))?
            && receipt.current_archive == binding(&current(options))?
            && receipt.current_installer == binding(&installer(options))?
            && receipt.legacy_payloads_before == receipt.legacy_payloads_after
            && receipt.legacy_payloads_after == payloads(options)?
            && receipt.selected_after_rejection == pointer(TAG)
            && receipt.selected_after_recovery == pointer(&options.tag)
            && selected(options)? == pointer(&options.tag)
            && process::read_bounded(&root(options).join("fixture/curl"), 16384)?
                == ACQUIRE.as_bytes()
            && process::read_bounded(&root(options).join("receipt.json"), MAXIMUM_RECEIPT_BYTES)?
                == evidence::encode_json(receipt)?
            && process::read_bounded(&root(options).join("commands.json"), MAXIMUM_RECEIPT_BYTES)?
                == evidence::encode_json(&receipt.commands)?,
        "legacy manager recovery evidence is incomplete or changed",
    )?;
    for (index, command) in receipt.commands.iter().enumerate() {
        validate_command(options, index, command)?;
    }
    Ok(())
}
