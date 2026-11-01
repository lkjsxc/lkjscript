//! Native process and immutable repository-copy support for raw-source controls.
//! These are explicit test wall-clock policies, not compiler admission limits.
use super::{Check, equal, sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

struct JoinedChild(Child);

impl Drop for JoinedChild {
    fn drop(&mut self) {
        // Child is declared after its temporary directory, so every early return
        // joins it before its executable or file-backed I/O is removed.
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub(super) struct NativeOutput {
    pub(super) stdout: Vec<u8>,
    pub(super) stderr: Vec<u8>,
    pub(super) status: ExitStatus,
}

pub(super) fn run_native(
    label: &str,
    executable: &[u8],
    input: &[u8],
    deadline: Duration,
) -> Check<NativeOutput> {
    crate::release_container::inspect_static_elf_bytes(executable)
        .map_err(|error| format!("{label}: static ELF: {error:?}"))?;
    let directory = tempfile::tempdir().map_err(|error| format!("{label}: tempdir: {error}"))?;
    let binary = directory.path().join("program");
    let input_path = directory.path().join("input.bin");
    let output_path = directory.path().join("output.bin");
    let diagnostic_path = directory.path().join("diagnostic.txt");
    crate::platform::owned_output::publish_executable_create_new(
        &binary,
        executable,
        None,
        "raw source artifact acceptance",
    )
    .map_err(|error| format!("{label}: executable publication: {error:?}"))?;
    fs::write(&input_path, input).map_err(|error| format!("{label}: input: {error}"))?;
    let stdin = File::open(&input_path).map_err(|error| format!("{label}: open input: {error}"))?;
    let stdout =
        File::create(&output_path).map_err(|error| format!("{label}: output file: {error}"))?;
    let stderr = File::create(&diagnostic_path)
        .map_err(|error| format!("{label}: diagnostic file: {error}"))?;
    let start = Instant::now();
    let mut child = JoinedChild(
        Command::new(&binary)
            .env_clear()
            .current_dir(directory.path())
            .stdin(Stdio::from(stdin))
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .spawn()
            .map_err(|error| format!("{label}: spawn: {error}"))?,
    );
    let status = loop {
        if let Some(status) = child
            .0
            .try_wait()
            .map_err(|error| format!("{label}: wait: {error}"))?
        {
            break status;
        }
        if start.elapsed() >= deadline {
            return Err(format!(
                "{label}: explicit caller test deadline {deadline:?} exceeded"
            ));
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let output =
        fs::read(&output_path).map_err(|error| format!("{label}: read output: {error}"))?;
    let errors = fs::read(&diagnostic_path)
        .map_err(|error| format!("{label}: read diagnostics: {error}"))?;
    eprintln!(
        "raw source native case={label} executable_sha256={} request_sha256={} request_bytes={} \
         output_sha256={} output_bytes={} diagnostic_sha256={} exit={:?} elapsed_ms={} \
         caller_deadline_seconds={} environment=empty arguments=[] io=regular-files capture_quota=none",
        sha256(executable),
        sha256(input),
        input.len(),
        sha256(&output),
        output.len(),
        sha256(&errors),
        status.code(),
        start.elapsed().as_millis(),
        deadline.as_secs(),
    );
    Ok(NativeOutput {
        stdout: output,
        stderr: errors,
        status,
    })
}

pub(super) fn run_filter(
    label: &str,
    executable: &[u8],
    request: &[u8],
    diagnostic: i64,
    deadline: Duration,
) -> Check<Vec<u8>> {
    let output = run_native(label, executable, request, deadline)?;
    equal(
        output.status.code(),
        Some(if diagnostic == 0 { 0 } else { 65 }),
        label,
    )?;
    if diagnostic == 0 {
        equal(
            output.stderr.as_slice(),
            &[][..],
            &format!("{label}: success stderr"),
        )?;
    } else {
        equal(
            output.stdout.as_slice(),
            &[][..],
            &format!("{label}: failure output"),
        )?;
        equal(
            output.stderr.as_slice(),
            format!("compiler_rejected:{diagnostic}\n").as_bytes(),
            &format!("{label}: failure diagnostic"),
        )?;
    }
    Ok(output.stdout)
}

pub(super) fn run_scalar(
    label: &str,
    image: &[u8],
    expected: i64,
    deadline: Duration,
) -> Check<()> {
    let output = run_native(label, image, &[], deadline)?;
    equal(
        output.status.code(),
        Some(0),
        &format!("{label}: scalar status"),
    )?;
    equal(
        output.stderr.as_slice(),
        &[][..],
        &format!("{label}: scalar stderr"),
    )?;
    equal(
        output.stdout.as_slice(),
        expected.to_string().as_bytes(),
        &format!("{label}: scalar value"),
    )
}

type FileInventory = BTreeMap<PathBuf, (u64, String)>;

#[derive(Clone, Debug, Eq, PartialEq)]
struct RepositoryInventory {
    files: FileInventory,
    directories: BTreeSet<PathBuf>,
}

fn file_identity(path: &Path) -> Check<(u64, String)> {
    use sha2::{Digest, Sha256};
    let mut file = File::open(path).map_err(|error| format!("repository file: {error}"))?;
    let mut digest = Sha256::new();
    let mut bytes = 0u64;
    let mut block = [0u8; 65_536];
    loop {
        let count = file
            .read(&mut block)
            .map_err(|error| format!("repository read: {error}"))?;
        if count == 0 {
            break;
        }
        bytes = bytes
            .checked_add(u64::try_from(count).map_err(|_| "file read extent".to_owned())?)
            .ok_or_else(|| "physical file extent exceeds u64".to_owned())?;
        digest.update(&block[..count]);
    }
    Ok((
        bytes,
        crate::platform::semantic_id::encode_hex(&digest.finalize()),
    ))
}

fn inventory(root: &Path) -> Check<RepositoryInventory> {
    let mut files = BTreeMap::new();
    let mut directories = BTreeSet::from([PathBuf::new()]);
    let mut pending = vec![PathBuf::new()];
    while let Some(relative) = pending.pop() {
        let entries = fs::read_dir(root.join(&relative))
            .map_err(|error| format!("repository inventory {relative:?}: {error}"))?;
        for entry in entries {
            let entry = entry.map_err(|error| format!("repository entry: {error}"))?;
            let relative = relative.join(entry.file_name());
            let kind = entry
                .file_type()
                .map_err(|error| format!("repository file type: {error}"))?;
            if kind.is_dir() {
                directories.insert(relative.clone());
                pending.push(relative);
            } else if kind.is_file() {
                files.insert(relative, file_identity(&entry.path())?);
            } else {
                return Err(format!(
                    "repository copy requires ordinary files/directories: {relative:?}"
                ));
            }
        }
    }
    Ok(RepositoryInventory { files, directories })
}

pub(super) struct RepositoryCopy {
    // Owns the lifetime of path. The underscore suppresses an otherwise unused
    // field warning; its destructor is part of the temporary-copy contract.
    _directory: tempfile::TempDir,
    pub(super) path: PathBuf,
    before: RepositoryInventory,
}

impl RepositoryCopy {
    pub(super) fn create(original: &Path) -> Check<Self> {
        let before = inventory(original)?;
        let directory =
            tempfile::tempdir().map_err(|error| format!("repository tempdir: {error}"))?;
        let path = directory.path().join("project");
        fs::create_dir(&path).map_err(|error| format!("repository root: {error}"))?;
        // Copy immutable bytes, including the accepted HEAD and its revision
        // chain. Public GraphRepository APIs own all subsequent reads/writes.
        for relative in &before.directories {
            fs::create_dir_all(path.join(relative))
                .map_err(|error| format!("repository directory {relative:?}: {error}"))?;
        }
        for relative in before.files.keys() {
            let destination = path.join(relative);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| format!("repository parent: {error}"))?;
            }
            fs::copy(original.join(relative), &destination)
                .map_err(|error| format!("repository copy {relative:?}: {error}"))?;
        }
        equal(
            inventory(&path)?,
            before.clone(),
            "exact copied repository bytes",
        )?;
        equal(
            inventory(original)?,
            before.clone(),
            "original quiescent during repository copy",
        )?;
        eprintln!(
            "raw source repository copy files={} directories={} head_sha256={}",
            before.files.len(),
            before.directories.len(),
            sha256(&fs::read(original.join("HEAD")).map_err(|error| format!("HEAD: {error}"))?)
        );
        Ok(Self {
            _directory: directory,
            path,
            before,
        })
    }

    pub(super) fn verify_original(&self, original: &Path) -> Check<()> {
        equal(
            inventory(original)?,
            self.before.clone(),
            "maintained repository byte inventory after test",
        )
    }
}
