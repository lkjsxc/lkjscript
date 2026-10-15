//! Seven process boundaries against exact bytes, supplementing complete source semantics.
use super::{archive, boundary_evidence, file, public_inventory, require, source_outcomes};
use crate::{error::DevError, evidence, process};
use serde::Serialize;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

const OUTPUT_LIMIT: u64 = 16 * 1024 * 1024;
const MAXIMUM_WORKERS: usize = 16;

pub(super) fn valid_workers(workers: usize) -> bool {
    (1..=MAXIMUM_WORKERS).contains(&workers)
}

pub(super) struct Accepted {
    pub(super) receipt: PathBuf,
    pub(super) workers: usize,
    pub(super) boundaries: usize,
}

pub(super) use crate::check::AdmittedSource;

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
    purpose: &'static str,
    verifier: super::ArtifactIdentity,
    source_semantics: Option<crate::check::SourceProof>,
    source_public_cases: Option<usize>,
    workers: usize,
    candidate: super::ArtifactIdentity,
    harness: Option<super::ArtifactIdentity>,
    inventory: Option<public_inventory::Inventory>,
    boundaries: Vec<boundary_evidence::Accepted>,
    phases: Vec<Phase>,
    cleanup_complete: bool,
    failure: Option<String>,
}

struct Runner<'a> {
    source: Option<&'a AdmittedSource>,
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
        self.source_unchanged()?;
        let build = self.run(
            "build",
            [
                "cargo",
                "test",
                // Match source acceptance's workspace feature unification, so
                // Cargo can reuse its admitted test-profile dependencies/harness.
                "--workspace",
                "--all-features",
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
        let result = self.evaluate(&harness, &original, candidate, work.path());
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
        self.source_unchanged()?;
        self.receipt.cleanup_complete = true;
        Ok(())
    }

    fn source_unchanged(&self) -> Result<(), DevError> {
        require(
            self.receipt.verifier
                == file(&std::env::current_exe()?, super::verifier::EXECUTABLE_NAME)?,
            "native boundary verifier changed",
        )?;
        if let Some(source) = self.source {
            require(
                source.source_commit() == self.receipt.harness_source_commit
                    && self.receipt.source_semantics.as_ref() == Some(source.proof()),
                "native boundary source binding differs from its admitted prerequisite",
            )?;
            source.unchanged()?;
        }
        Ok(())
    }

    fn evaluate(
        &mut self,
        harness: &Path,
        original: &Path,
        candidate: &Path,
        work: &Path,
    ) -> Result<(), DevError> {
        require(
            self.receipt.candidate == file(candidate, "lkjscript")?,
            "native public candidate differs from the admitted executable",
        )?;
        require(
            valid_workers(self.receipt.workers),
            "native public harness worker count is outside 1..=16",
        )?;
        let home = work.join("home");
        let temporary = work.join("tmp");
        let boundary_root = self.root.join("boundaries");
        fs::create_dir(&home)?;
        fs::create_dir(&temporary)?;
        fs::create_dir(&boundary_root)?;
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
            (
                "LKJSCRIPT_NATIVE_BOUNDARY_EVIDENCE".to_owned(),
                boundary_root.display().to_string(),
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
        if let Some(source) = self.source {
            let (stdout, stderr) = source.workspace_logs()?;
            source_outcomes::harness(&stderr, source.repository(), original)?;
            source_outcomes::passed(&stdout, &inventory)?;
            source.unchanged()?;
            self.receipt.source_public_cases = Some(inventory.all.len());
        }
        let mut command = vec![
            harness.display().to_string(),
            "--exact".into(),
            format!("--test-threads={}", self.receipt.workers),
            "--color=never".into(),
        ];
        command.extend(inventory.selected.iter().cloned());
        self.receipt.inventory = Some(inventory);
        // The producer selects once; the receipt binds that count to this argv.
        // The complete semantic matrices were admitted by their original source
        // owner. This execution supplements them with seven explicit boundaries.
        let observed = self.run(
            "execute",
            command,
            work,
            environment,
            Duration::from_secs(15 * 60),
        )?;
        let inventory = self
            .receipt
            .inventory
            .as_ref()
            .ok_or_else(|| DevError::corrupt("missing public harness inventory"))?;
        public_inventory::passed(&observed, inventory)?;
        self.receipt.boundaries = boundary_evidence::read(&boundary_root, &self.receipt.candidate)?;
        self.save()?;
        require(
            self.receipt.candidate == file(candidate, "lkjscript")?,
            "native public candidate changed during execution",
        )
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
    source: &AdmittedSource,
    root: &Path,
    control: &process::ProcessControl,
) -> Result<Accepted, DevError> {
    run(repository, candidate, commit, Some(source), root, control)
}

fn diagnose(
    repository: &Path,
    candidate: &Path,
    commit: &str,
    root: &Path,
    control: &process::ProcessControl,
) -> Result<Accepted, DevError> {
    run(repository, candidate, commit, None, root, control)
}

fn run(
    repository: &Path,
    candidate: &Path,
    commit: &str,
    source_semantics: Option<&AdmittedSource>,
    root: &Path,
    control: &process::ProcessControl,
) -> Result<Accepted, DevError> {
    source(repository, commit)?;
    super::super::require_absolute_regular_executable(candidate, "native public candidate")?;
    super::super::require_absolute_extraction_output(root)?;
    fs::create_dir(root)?;
    fs::set_permissions(root, fs::Permissions::from_mode(0o700))?;
    let workers = std::thread::available_parallelism()
        .map_or(1, |available| available.get().min(MAXIMUM_WORKERS));
    let mut runner = Runner {
        source: source_semantics,
        root,
        control,
        receipt: Receipt {
            schema: super::SchemaIdentity {
                identity: "lkjscript-native-boundary-harness".to_owned(),
                version: 1,
            },
            status: "incomplete",
            phase: "created".to_owned(),
            harness_source_commit: commit.to_owned(),
            purpose: if source_semantics.is_some() {
                "candidate_acceptance_supplement"
            } else {
                "diagnostic_behavior_evidence"
            },
            verifier: file(&std::env::current_exe()?, super::verifier::EXECUTABLE_NAME)?,
            source_semantics: source_semantics.map(|source| source.proof().clone()),
            source_public_cases: None,
            workers,
            candidate: file(candidate, "lkjscript")?,
            harness: None,
            inventory: None,
            boundaries: Vec::new(),
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
    result.map(|()| Accepted {
        receipt: root.join("receipt.json"),
        workers: runner.receipt.workers,
        boundaries: runner.receipt.boundaries.len(),
    })
}

// This non-publishing source witness accepts ordinary and linked worktrees.
// Publishing owners retain their own stricter checkout and provenance admission.
fn harness_repository(current: &Path) -> Result<PathBuf, DevError> {
    archive::ensure_regular(
        &current.join("Cargo.toml"),
        "native harness root Cargo.toml",
    )?;
    let root =
        super::super::command_text("git", &["rev-parse", "--show-toplevel"], current, 16 * 1024)?;
    require(
        Path::new(&root) == current,
        "native harness requires the actual checkout root",
    )?;
    Ok(current.to_path_buf())
}

pub(crate) fn command(arguments: impl Iterator<Item = OsString>) -> Result<u8, DevError> {
    let mut values = super::verifier::parse_values(arguments, &["--candidate", "--evidence-root"])?;
    let candidate = PathBuf::from(super::verifier::required(&mut values, "--candidate")?);
    let root = PathBuf::from(super::verifier::required(&mut values, "--evidence-root")?);
    let repository = harness_repository(&std::env::current_dir()?)?;
    let commit = super::super::command_text("git", &["rev-parse", "HEAD"], &repository, 1024)?;
    let cancellation = super::super::transferred::Cancellation::new()?;
    let result = diagnose(
        &repository,
        &candidate,
        &commit,
        &root,
        &cancellation.control,
    );
    let joined = cancellation.finish();
    // Joining the control observer is part of command completion even on failure.
    let accepted = match (result, joined) {
        (Ok(accepted), Ok(())) => accepted,
        (Err(primary), Err(cleanup)) => {
            return Err(DevError::infrastructure(format!(
                "{primary}; observer cleanup: {cleanup}"
            )));
        }
        (Err(error), Ok(())) | (Ok(_), Err(error)) => return Err(error),
    };
    println!(
        "{}",
        serde_json::json!({"status":"passed", "purpose":"diagnostic_behavior_evidence", "receipt":accepted.receipt, "workers":accepted.workers})
    );
    Ok(0)
}

#[cfg(test)]
#[path = "public_harness_tests.rs"]
mod tests;
