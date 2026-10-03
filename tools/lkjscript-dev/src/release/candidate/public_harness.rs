//! Required native-language witnesses against the exact extracted candidate bytes.
//! Source acceptance and archive/source correspondence remain independently owned.
use super::{archive, file, public_inventory, require};
use crate::{error::DevError, evidence, process};
use serde::Serialize;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

const OUTPUT_LIMIT: u64 = 16 * 1024 * 1024;

#[derive(Serialize)]
struct Phase {
    name: String,
    command: Vec<String>,
    process: process::ProcessObservation,
}

#[derive(Serialize)]
struct Receipt {
    schema: super::SchemaIdentity,
    status: &'static str,
    phase: String,
    harness_source_commit: String,
    candidate: super::ArtifactIdentity,
    harness: Option<super::ArtifactIdentity>,
    inventory: Option<public_inventory::Inventory>,
    phases: Vec<Phase>,
    cleanup_complete: bool,
    failure: Option<String>,
}

struct Runner<'a> {
    root: &'a Path,
    control: &'a process::ProcessControl,
    receipt: Receipt,
}

impl Runner<'_> {
    fn save(&self) -> Result<(), DevError> {
        evidence::publish_json(&self.root.join("receipt.json"), &self.receipt).map(|_| ())
    }

    fn run(
        &mut self,
        name: &str,
        command: Vec<String>,
        cwd: &Path,
        environment: BTreeMap<String, String>,
        timeout: Duration,
    ) -> Result<String, DevError> {
        self.receipt.phase = name.to_owned();
        self.save()?;
        let stdout = self.root.join(format!("{name}.stdout.log"));
        let observed = process::run_supervised(
            &process::ProcessSpec {
                command: command.clone(),
                cwd: cwd.to_path_buf(),
                environment,
                timeout,
                maximum_stdout_bytes: OUTPUT_LIMIT,
                maximum_stderr_bytes: OUTPUT_LIMIT,
                stdout_path: stdout.clone(),
                stderr_path: self.root.join(format!("{name}.stderr.log")),
                unavailable_exit_code: None,
            },
            self.root,
            Some(self.control),
        );
        let passed = super::passed(&observed) && !self.control.cancelled();
        self.receipt.phases.push(Phase {
            name: name.to_owned(),
            command,
            process: observed,
        });
        self.save()?;
        require(
            passed,
            &format!("native public harness {name} failed; original logs retained"),
        )?;
        String::from_utf8(process::read_bounded(&stdout, OUTPUT_LIMIT)?)
            .map_err(|_| DevError::corrupt("native public harness output is not UTF-8"))
    }

    fn execute(&mut self, repository: &Path, candidate: &Path) -> Result<(), DevError> {
        let build = self.run(
            "build",
            [
                "cargo",
                "test",
                "--locked",
                "--test",
                "public_cli",
                "--no-run",
                "--message-format=json",
            ]
            .map(str::to_owned)
            .to_vec(),
            repository,
            process::environment(),
            Duration::from_secs(30 * 60),
        )?;
        let original = public_inventory::cargo_harness(&build, repository)?;
        super::super::require_absolute_regular_executable(
            &original,
            "source-built public harness",
        )?;
        let harness = self.root.join("public_cli");
        archive::copy_new(&original, &harness, 0o555)?;
        self.receipt.harness = Some(file(&harness, "public_cli")?);
        // Do not inherit credentials, Cargo locations, or authoring-checkout cwd.
        let work = tempfile::Builder::new()
            .prefix("lkjscript-native-candidate-")
            .tempdir_in("/tmp")?;
        let result = self.evaluate(&harness, candidate, work.path());
        let cleanup = work.close();
        match (result, cleanup) {
            (Ok(()), Ok(())) => {}
            (Err(primary), Err(cleanup)) => {
                return Err(DevError::infrastructure(format!(
                    "{primary}; native harness cleanup: {cleanup}"
                )));
            }
            (Err(error), Ok(())) => return Err(error),
            (Ok(()), Err(error)) => return Err(error.into()),
        }
        require(
            self.receipt.candidate == file(candidate, "lkjscript")?
                && self.receipt.harness == Some(file(&harness, "public_cli")?)
                && !self.control.cancelled(),
            "native harness inputs changed or execution was cancelled",
        )?;
        source(repository, &self.receipt.harness_source_commit)?;
        self.receipt.cleanup_complete = true;
        Ok(())
    }

    fn evaluate(&mut self, harness: &Path, candidate: &Path, work: &Path) -> Result<(), DevError> {
        let home = work.join("home");
        let temporary = work.join("tmp");
        fs::create_dir(&home)?;
        fs::create_dir(&temporary)?;
        let environment = BTreeMap::from([
            ("PATH".to_owned(), String::new()),
            ("HOME".to_owned(), home.display().to_string()),
            ("TMPDIR".to_owned(), temporary.display().to_string()),
            ("LC_ALL".to_owned(), "C".to_owned()),
            ("TZ".to_owned(), "UTC".to_owned()),
            (
                "LKJSCRIPT_RELEASE_CANDIDATE".to_owned(),
                candidate.display().to_string(),
            ),
        ]);
        let listing = self.run(
            "inventory",
            vec![
                harness.display().to_string(),
                "--list".into(),
                "--format=terse".into(),
                "--color=never".into(),
            ],
            work,
            environment.clone(),
            Duration::from_secs(60),
        )?;
        let inventory = public_inventory::inventory(&listing)?;
        let mut command = vec![
            harness.display().to_string(),
            "--exact".into(),
            "--test-threads=1".into(),
            "--color=never".into(),
        ];
        command.extend(inventory.selected.iter().cloned());
        self.receipt.inventory = Some(inventory);
        let observed = self.run(
            "execute",
            command,
            work,
            environment,
            Duration::from_secs(30 * 60),
        )?;
        let inventory = self
            .receipt
            .inventory
            .as_ref()
            .ok_or_else(|| DevError::corrupt("missing public harness inventory"))?;
        public_inventory::passed(&observed, inventory)
    }
}

fn source(repository: &Path, expected: &str) -> Result<(), DevError> {
    super::super::ensure_clean_checkout(repository)?;
    let actual = super::super::command_text("git", &["rev-parse", "HEAD"], repository, 1024)?;
    require(actual == expected, "public harness source checkout changed")
}

pub(super) fn accept(
    repository: &Path,
    candidate: &Path,
    commit: &str,
    root: &Path,
    control: &process::ProcessControl,
) -> Result<PathBuf, DevError> {
    source(repository, commit)?;
    super::super::require_absolute_regular_executable(candidate, "native public candidate")?;
    super::super::require_absolute_extraction_output(root)?;
    fs::create_dir(root)?;
    fs::set_permissions(root, fs::Permissions::from_mode(0o700))?;
    let mut runner = Runner {
        root,
        control,
        receipt: Receipt {
            schema: super::SchemaIdentity {
                identity: "lkjscript-native-public-harness".to_owned(),
                version: 1,
            },
            status: "incomplete",
            phase: "created".to_owned(),
            harness_source_commit: commit.to_owned(),
            candidate: file(candidate, "lkjscript")?,
            harness: None,
            inventory: None,
            phases: Vec::new(),
            cleanup_complete: false,
            failure: None,
        },
    };
    runner.save()?;
    let result = runner.execute(repository, candidate);
    if let Err(error) = &result {
        runner.receipt.status = if control.cancelled() {
            "cancelled"
        } else {
            "failed"
        };
        runner.receipt.failure = Some(error.to_string());
    } else {
        runner.receipt.status = "passed";
        runner.receipt.phase = "complete".to_owned();
    }
    runner.save()?;
    result.map(|()| root.join("receipt.json"))
}

pub(crate) fn command(arguments: impl Iterator<Item = OsString>) -> Result<u8, DevError> {
    let mut values = super::verifier::parse_values(arguments, &["--candidate", "--evidence-root"])?;
    let candidate = PathBuf::from(super::verifier::required(&mut values, "--candidate")?);
    let root = PathBuf::from(super::verifier::required(&mut values, "--evidence-root")?);
    let repository = super::super::repository_root()?;
    let commit = super::super::command_text("git", &["rev-parse", "HEAD"], &repository, 1024)?;
    let cancellation = super::super::transferred::Cancellation::new()?;
    let result = accept(
        &repository,
        &candidate,
        &commit,
        &root,
        &cancellation.control,
    );
    let joined = cancellation.finish();
    // Joining the control observer is part of command completion even on failure.
    let receipt = match (result, joined) {
        (Ok(receipt), Ok(())) => receipt,
        (Err(primary), Err(cleanup)) => {
            return Err(DevError::infrastructure(format!(
                "{primary}; observer cleanup: {cleanup}"
            )));
        }
        (Err(error), Ok(())) | (Ok(_), Err(error)) => return Err(error),
    };
    println!(
        "{}",
        serde_json::json!({"status":"passed", "receipt":receipt})
    );
    Ok(0)
}

#[cfg(test)]
#[path = "public_harness_tests.rs"]
mod tests;
