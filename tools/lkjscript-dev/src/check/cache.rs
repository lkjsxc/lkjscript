use super::model::{
    CACHE_CONTRACT_VERSION, CacheRecord, Gate, GateReceipt, GateStatus, MAXIMUM_CACHE_BYTES,
    MAXIMUM_CACHE_ENTRIES_PER_GATE, MAXIMUM_CACHE_RECORD_BYTES,
};
use crate::error::DevError;
use crate::evidence::{self, FileKind, FileProof, VerificationDigest};
use crate::process::{ProcessObservation, ProcessStatus};
use fs2::FileExt;
use serde::Serialize;
use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub(crate) struct VerificationCache {
    repository: PathBuf,
    root: PathBuf,
}

#[derive(Clone, Debug)]
pub(crate) struct CacheLoad {
    pub(crate) cached: Option<CachedGate>,
    pub(crate) reason: String,
}

#[derive(Clone, Debug)]
pub(crate) struct CachedGate {
    pub(crate) process: ProcessObservation,
    pub(crate) outputs: Vec<FileProof>,
    pub(crate) evidence_digest: VerificationDigest,
    pub(crate) source_elapsed_nanoseconds: u64,
    pub(crate) record: String,
}

impl VerificationCache {
    pub(crate) fn new(repository: &Path, root: &Path) -> Self {
        Self {
            repository: repository.to_path_buf(),
            root: root.to_path_buf(),
        }
    }

    pub(crate) fn load(
        &self,
        gate: &Gate,
        fingerprint: &VerificationDigest,
        stdout_path: &Path,
        stderr_path: &Path,
    ) -> CacheLoad {
        match self.load_inner(gate, fingerprint, stdout_path, stderr_path) {
            Ok(load) => load,
            Err(error) => CacheLoad {
                cached: None,
                reason: format!("cache_corrupt:{}", error.kind()),
            },
        }
    }

    pub(crate) fn store(&self, gate: &Gate, receipt: &GateReceipt) -> Result<String, DevError> {
        if !gate.cacheable
            || receipt.status != GateStatus::Passed
            || receipt.execution != super::model::ExecutionKind::Fresh
        {
            return Err(DevError::infrastructure(
                "attempted to cache an ineligible gate result",
            ));
        }
        let process = receipt.process.as_ref().ok_or_else(|| {
            DevError::infrastructure("passed gate receipt has no process observation")
        })?;
        if process.status != ProcessStatus::Passed {
            return Err(DevError::infrastructure(
                "passed gate has a non-passing child observation",
            ));
        }
        let lock = self.lock()?;
        let result = (|| {
            self.store_log(&process.stdout)?;
            self.store_log(&process.stderr)?;
            let record = CacheRecord {
                cache_contract_version: CACHE_CONTRACT_VERSION,
                gate: gate.name.clone(),
                input_fingerprint: receipt.input_fingerprint.clone(),
                identity_command: gate.identity_command().to_vec(),
                source_elapsed_nanoseconds: receipt.elapsed_nanoseconds,
                process: process.clone(),
                outputs: receipt.outputs.clone(),
                evidence_digest: receipt.evidence_digest.clone(),
            };
            let path = self.record_path(gate, &receipt.input_fingerprint)?;
            evidence::publish_json(&path, &record)?;
            self.rotate_gate(&path)?;
            self.prune(&path)?;
            Ok(evidence::relative(&self.repository, &path))
        })();
        let unlock = FileExt::unlock(&lock).map_err(|error| {
            DevError::infrastructure(format!("unlock verification cache: {error}"))
        });
        match (result, unlock) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error),
        }
    }

    pub(crate) fn record_path(
        &self,
        gate: &Gate,
        fingerprint: &VerificationDigest,
    ) -> Result<PathBuf, DevError> {
        validate_component(&gate.name, "gate")?;
        validate_digest(fingerprint)?;
        Ok(self
            .root
            .join("evidence")
            .join(&gate.name)
            .join(format!("{}.json", fingerprint.as_str())))
    }

    fn load_inner(
        &self,
        gate: &Gate,
        fingerprint: &VerificationDigest,
        stdout_path: &Path,
        stderr_path: &Path,
    ) -> Result<CacheLoad, DevError> {
        let lock = self.lock()?;
        let result = (|| {
            let record_path = self.record_path(gate, fingerprint)?;
            let metadata = match fs::symlink_metadata(&record_path) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    return Ok(CacheLoad {
                        cached: None,
                        reason: self.miss_reason(&record_path),
                    });
                }
                Err(error) => {
                    return Err(DevError::corrupt(format!(
                        "inspect cache record '{}': {error}",
                        record_path.display()
                    )));
                }
            };
            if metadata.file_type().is_symlink()
                || !metadata.is_file()
                || metadata.len() > MAXIMUM_CACHE_RECORD_BYTES
            {
                return Ok(CacheLoad {
                    cached: None,
                    reason: "unsafe_or_oversized_record".to_owned(),
                });
            }
            let bytes = read_bounded(&record_path, MAXIMUM_CACHE_RECORD_BYTES)?;
            let record: CacheRecord = match serde_json::from_slice(&bytes) {
                Ok(record) => record,
                Err(_) => {
                    return Ok(CacheLoad {
                        cached: None,
                        reason: "record_corrupt".to_owned(),
                    });
                }
            };
            if record.cache_contract_version != CACHE_CONTRACT_VERSION
                || record.gate != gate.name
                || record.input_fingerprint != *fingerprint
                || record.identity_command != gate.identity_command()
                || record.process.status != ProcessStatus::Passed
            {
                return Ok(CacheLoad {
                    cached: None,
                    reason: "record_binding".to_owned(),
                });
            }
            let current_outputs = output_proofs(&self.repository, &gate.required_outputs)?;
            if current_outputs != record.outputs {
                return Ok(CacheLoad {
                    cached: None,
                    reason: "declared_output_changed".to_owned(),
                });
            }
            if gate_evidence_digest(&gate.name, fingerprint, &record.process, &record.outputs)?
                != record.evidence_digest
            {
                return Ok(CacheLoad {
                    cached: None,
                    reason: "evidence_digest".to_owned(),
                });
            }
            let stdout = match self.restore_log(&record.process.stdout, stdout_path)? {
                Some(proof) => proof,
                None => {
                    return Ok(CacheLoad {
                        cached: None,
                        reason: "stdout_log_missing_or_corrupt".to_owned(),
                    });
                }
            };
            let stderr = match self.restore_log(&record.process.stderr, stderr_path)? {
                Some(proof) => proof,
                None => {
                    return Ok(CacheLoad {
                        cached: None,
                        reason: "stderr_log_missing_or_corrupt".to_owned(),
                    });
                }
            };
            let mut process = record.process;
            process.stdout = stdout;
            process.stderr = stderr;
            Ok(CacheLoad {
                cached: Some(CachedGate {
                    process,
                    outputs: record.outputs,
                    evidence_digest: record.evidence_digest,
                    source_elapsed_nanoseconds: record.source_elapsed_nanoseconds,
                    record: evidence::relative(&self.repository, &record_path),
                }),
                reason: "hit".to_owned(),
            })
        })();
        let unlock = FileExt::unlock(&lock).map_err(|error| {
            DevError::infrastructure(format!("unlock verification cache: {error}"))
        });
        match (result, unlock) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error),
        }
    }

    fn lock(&self) -> Result<File, DevError> {
        fs::create_dir_all(&self.root).map_err(|error| {
            DevError::infrastructure(format!(
                "create verification cache '{}': {error}",
                self.root.display()
            ))
        })?;
        let path = self.root.join("LOCK");
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|error| {
                DevError::infrastructure(format!("open cache lock '{}': {error}", path.display()))
            })?;
        file.lock_exclusive().map_err(|error| {
            DevError::infrastructure(format!("lock verification cache: {error}"))
        })?;
        Ok(file)
    }

    fn store_log(&self, observation: &FileProof) -> Result<(), DevError> {
        if observation.kind != FileKind::File {
            return Err(DevError::infrastructure("child log is not a regular file"));
        }
        let digest = observation
            .digest
            .as_ref()
            .ok_or_else(|| DevError::infrastructure("child log digest is absent"))?;
        let source = resolve_proof_path(&self.repository, &observation.path);
        let bytes = read_bounded(&source, observation.bytes.unwrap_or(0))?;
        if VerificationDigest::of(&bytes) != *digest {
            return Err(DevError::infrastructure(format!(
                "child log '{}' changed before cache publication",
                source.display()
            )));
        }
        let destination = self.log_path(digest)?;
        evidence::publish(&destination, &bytes)?;
        Ok(())
    }

    fn restore_log(
        &self,
        observation: &FileProof,
        destination: &Path,
    ) -> Result<Option<FileProof>, DevError> {
        let Some(digest) = observation.digest.as_ref() else {
            return Ok(None);
        };
        let path = self.log_path(digest)?;
        let proof = match evidence::proof(&path, evidence::relative(&self.repository, &path)) {
            Ok(proof) => proof,
            Err(_) => return Ok(None),
        };
        if proof.kind != FileKind::File
            || proof.bytes != observation.bytes
            || proof.digest.as_ref() != Some(digest)
        {
            return Ok(None);
        }
        let bytes = read_bounded(&path, observation.bytes.unwrap_or(0))?;
        evidence::publish(destination, &bytes)?;
        evidence::proof(
            destination,
            evidence::relative(&self.repository, destination),
        )
        .map(Some)
    }

    fn log_path(&self, digest: &VerificationDigest) -> Result<PathBuf, DevError> {
        validate_digest(digest)?;
        Ok(self
            .root
            .join("logs")
            .join(format!("{}.log", digest.as_str())))
    }

    fn miss_reason(&self, path: &Path) -> String {
        if path
            .parent()
            .and_then(|parent| fs::read_dir(parent).ok())
            .is_some_and(|mut entries| entries.any(|entry| entry.is_ok()))
        {
            "input_changed".to_owned()
        } else {
            "not_found".to_owned()
        }
    }

    fn rotate_gate(&self, current: &Path) -> Result<(), DevError> {
        let Some(parent) = current.parent() else {
            return Err(DevError::infrastructure("cache record has no parent"));
        };
        let mut records = regular_files(parent, "json")?;
        records.sort_by_key(|path| {
            fs::metadata(path)
                .and_then(|metadata| metadata.modified())
                .ok()
        });
        let excess = records.len().saturating_sub(MAXIMUM_CACHE_ENTRIES_PER_GATE);
        for obsolete in records.into_iter().take(excess) {
            if obsolete != current {
                fs::remove_file(&obsolete).map_err(|error| {
                    DevError::infrastructure(format!(
                        "remove old cache record '{}': {error}",
                        obsolete.display()
                    ))
                })?;
            }
        }
        Ok(())
    }

    fn prune(&self, current: &Path) -> Result<(), DevError> {
        let evidence_root = self.root.join("evidence");
        let logs_root = self.root.join("logs");
        let mut records = Vec::new();
        if let Ok(gates) = fs::read_dir(&evidence_root) {
            for gate in gates.flatten() {
                let path = gate.path();
                let metadata = fs::symlink_metadata(&path).map_err(|error| {
                    DevError::infrastructure(format!(
                        "inspect cache gate directory '{}': {error}",
                        path.display()
                    ))
                })?;
                if metadata.is_dir() && !metadata.file_type().is_symlink() {
                    records.extend(regular_files(&path, "json")?);
                }
            }
        }
        records.sort_by_key(|path| {
            fs::metadata(path)
                .and_then(|metadata| metadata.modified())
                .ok()
        });
        let mut retained_bytes =
            directory_file_bytes(&evidence_root)?.saturating_add(directory_file_bytes(&logs_root)?);
        while records.len() > 1 && retained_bytes > MAXIMUM_CACHE_BYTES {
            let Some(index) = records.iter().position(|path| path != current) else {
                break;
            };
            let obsolete = records.remove(index);
            let bytes = fs::metadata(&obsolete)
                .map(|value| value.len())
                .unwrap_or(0);
            fs::remove_file(&obsolete).map_err(|error| {
                DevError::infrastructure(format!(
                    "remove oversized cache record '{}': {error}",
                    obsolete.display()
                ))
            })?;
            retained_bytes = retained_bytes.saturating_sub(bytes);
        }
        let mut referenced = BTreeSet::new();
        for record in records {
            if let Ok(bytes) = read_bounded(&record, MAXIMUM_CACHE_RECORD_BYTES)
                && let Ok(record) = serde_json::from_slice::<CacheRecord>(&bytes)
            {
                if let Some(digest) = record.process.stdout.digest {
                    referenced.insert(digest.as_str().to_owned());
                }
                if let Some(digest) = record.process.stderr.digest {
                    referenced.insert(digest.as_str().to_owned());
                }
            }
        }
        for log in regular_files(&logs_root, "log")? {
            let stem = log.file_stem().and_then(|value| value.to_str());
            if stem.is_some_and(|value| !referenced.contains(value)) {
                fs::remove_file(&log).map_err(|error| {
                    DevError::infrastructure(format!(
                        "remove unreferenced cache log '{}': {error}",
                        log.display()
                    ))
                })?;
            }
        }
        Ok(())
    }
}

pub(crate) fn output_proofs(
    repository: &Path,
    outputs: &[PathBuf],
) -> Result<Vec<FileProof>, DevError> {
    outputs
        .iter()
        .map(|path| evidence::proof(path, evidence::relative(repository, path)))
        .collect()
}

pub(crate) fn gate_evidence_digest(
    gate: &str,
    fingerprint: &VerificationDigest,
    process: &ProcessObservation,
    outputs: &[FileProof],
) -> Result<VerificationDigest, DevError> {
    #[derive(Serialize)]
    struct StreamIdentity<'a> {
        bytes: Option<u64>,
        digest: Option<&'a VerificationDigest>,
    }
    #[derive(Serialize)]
    struct EvidenceIdentity<'a> {
        gate: &'a str,
        input_fingerprint: &'a VerificationDigest,
        status: ProcessStatus,
        exit_code: Option<i32>,
        signal: Option<i32>,
        stdout_limit_bytes: u64,
        stderr_limit_bytes: u64,
        stdout_limit_exhausted: bool,
        stderr_limit_exhausted: bool,
        stdout: StreamIdentity<'a>,
        stderr: StreamIdentity<'a>,
        outputs: &'a [FileProof],
    }
    let bytes = serde_json::to_vec(&EvidenceIdentity {
        gate,
        input_fingerprint: fingerprint,
        status: process.status,
        exit_code: process.exit_code,
        signal: process.signal,
        stdout_limit_bytes: process.stdout_limit_bytes,
        stderr_limit_bytes: process.stderr_limit_bytes,
        stdout_limit_exhausted: process.stdout_limit_exhausted,
        stderr_limit_exhausted: process.stderr_limit_exhausted,
        stdout: StreamIdentity {
            bytes: process.stdout.bytes,
            digest: process.stdout.digest.as_ref(),
        },
        stderr: StreamIdentity {
            bytes: process.stderr.bytes,
            digest: process.stderr.digest.as_ref(),
        },
        outputs,
    })
    .map_err(|error| DevError::infrastructure(format!("encode gate evidence: {error}")))?;
    Ok(VerificationDigest::of(&bytes))
}

fn read_bounded(path: &Path, maximum: u64) -> Result<Vec<u8>, DevError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        DevError::corrupt(format!("inspect cache file '{}': {error}", path.display()))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > maximum {
        return Err(DevError::corrupt(format!(
            "cache file '{}' is unsafe or exceeds {maximum} bytes",
            path.display()
        )));
    }
    let capacity = usize::try_from(metadata.len())
        .map_err(|_| DevError::corrupt("cache file length does not fit this platform"))?;
    let mut bytes = Vec::with_capacity(capacity);
    File::open(path)
        .and_then(|file| file.take(maximum.saturating_add(1)).read_to_end(&mut bytes))
        .map_err(|error| {
            DevError::corrupt(format!("read cache file '{}': {error}", path.display()))
        })?;
    if bytes.len() as u64 != metadata.len() {
        return Err(DevError::corrupt(format!(
            "cache file '{}' changed during read",
            path.display()
        )));
    }
    Ok(bytes)
}

fn resolve_proof_path(repository: &Path, value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else {
        repository.join(path)
    }
}

fn validate_component(value: &str, label: &str) -> Result<(), DevError> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return Err(DevError::infrastructure(format!(
            "unsafe cache {label} '{value}'"
        )));
    }
    Ok(())
}

fn validate_digest(digest: &VerificationDigest) -> Result<(), DevError> {
    let value = digest.as_str();
    let encoded = value
        .strip_prefix("verification_")
        .ok_or_else(|| DevError::infrastructure("foreign verification digest domain"))?;
    if encoded.len() != 64 || !encoded.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(DevError::infrastructure("unsafe verification digest"));
    }
    Ok(())
}

fn regular_files(root: &Path, extension: &str) -> Result<Vec<PathBuf>, DevError> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(DevError::infrastructure(format!(
                "read cache directory '{}': {error}",
                root.display()
            )));
        }
    };
    let mut files = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some(extension) {
            continue;
        }
        let metadata = fs::symlink_metadata(&path).map_err(|error| {
            DevError::infrastructure(format!("inspect cache entry '{}': {error}", path.display()))
        })?;
        if metadata.is_file() && !metadata.file_type().is_symlink() {
            files.push(path);
        }
    }
    Ok(files)
}

fn directory_file_bytes(root: &Path) -> Result<u64, DevError> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => {
            return Err(DevError::infrastructure(format!(
                "read cache size directory '{}': {error}",
                root.display()
            )));
        }
    };
    let mut total = 0_u64;
    for entry in entries.flatten() {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(|error| {
            DevError::infrastructure(format!("inspect cache path '{}': {error}", path.display()))
        })?;
        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            total = total.saturating_add(directory_file_bytes(&path)?);
        } else if metadata.is_file() && !metadata.file_type().is_symlink() {
            total = total.saturating_add(metadata.len());
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::model::{CacheLookupStatus, CacheObservation, ExecutionKind};
    use crate::process::{self, ProcessSpec};
    use std::collections::BTreeMap;
    use std::time::Duration;

    #[test]
    fn exact_cache_hit_restores_logs_and_corruption_misses() {
        let temporary = tempfile::tempdir().expect("temporary cache repository");
        let run = temporary.path().join("run");
        fs::create_dir(&run).expect("create run directory");
        let process = process::run(
            &ProcessSpec {
                command: vec!["/bin/true".to_owned()],
                cwd: temporary.path().to_path_buf(),
                environment: BTreeMap::new(),
                timeout: Duration::from_secs(1),
                maximum_stdout_bytes: 1024,
                maximum_stderr_bytes: 1024,
                stdout_path: run.join("gate.stdout.log"),
                stderr_path: run.join("gate.stderr.log"),
                unavailable_exit_code: None,
            },
            temporary.path(),
        );
        let gate = Gate::new("gate", vec!["/bin/true".to_owned()]);
        let fingerprint = VerificationDigest::of(b"input");
        let evidence_digest = gate_evidence_digest("gate", &fingerprint, &process, &[])
            .expect("gate evidence digest");
        let receipt = GateReceipt {
            name: "gate".to_owned(),
            status: GateStatus::Passed,
            execution: ExecutionKind::Fresh,
            command: gate.command.clone(),
            dependencies: Vec::new(),
            failed_dependencies: Vec::new(),
            started_unix_nanoseconds: 1,
            completed_unix_nanoseconds: 2,
            elapsed_nanoseconds: process.elapsed_nanoseconds,
            process: Some(process),
            outputs: Vec::new(),
            input_fingerprint: fingerprint.clone(),
            evidence_digest,
            cache: CacheObservation {
                eligible: true,
                lookup: CacheLookupStatus::Miss,
                reason: None,
                record: None,
                write: None,
            },
            reason: None,
            stdout_excerpt: None,
            stderr_excerpt: None,
        };
        let cache = VerificationCache::new(temporary.path(), &temporary.path().join("cache"));
        cache.store(&gate, &receipt).expect("store cache evidence");
        let hit = cache.load(
            &gate,
            &fingerprint,
            &run.join("hit.stdout.log"),
            &run.join("hit.stderr.log"),
        );
        assert!(hit.cached.is_some());

        let record = cache
            .record_path(&gate, &fingerprint)
            .expect("cache record path");
        evidence::publish(&record, b"{not-json\n").expect("corrupt cache record");
        let corrupt = cache.load(
            &gate,
            &fingerprint,
            &run.join("corrupt.stdout.log"),
            &run.join("corrupt.stderr.log"),
        );
        assert!(corrupt.cached.is_none());
        assert_eq!(corrupt.reason, "record_corrupt");
    }
}
