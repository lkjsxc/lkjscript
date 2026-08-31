use super::model::CommandEvidence;
use crate::error::DevError;
use crate::evidence::VerificationDigest;
use crate::process::{self, ProcessSpec, ProcessStatus};
use lkjscript::platform::control::{CompactRecord, parse_records};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const MAXIMUM_STDOUT_BYTES: u64 = 4 * 1_048_576;
const MAXIMUM_STDERR_BYTES: u64 = 1_048_576;
const MAXIMUM_COMMAND_SECONDS: u64 = 3_600;

pub(crate) struct Invocation {
    pub(crate) ordinal: u64,
    pub(crate) records: Vec<CompactRecord>,
}

pub(crate) struct Runner {
    repository: PathBuf,
    evidence_root: PathBuf,
    started: Instant,
    maximum_wall: Duration,
    maximum_run_bytes: u64,
    commands: Vec<CommandEvidence>,
}

impl Runner {
    pub(crate) fn new(
        repository: &Path,
        evidence_root: &Path,
        started: Instant,
        maximum_wall: Duration,
        maximum_run_bytes: u64,
    ) -> Self {
        Self {
            repository: repository.to_path_buf(),
            evidence_root: evidence_root.to_path_buf(),
            started,
            maximum_wall,
            maximum_run_bytes,
            commands: Vec::new(),
        }
    }

    pub(crate) fn invoke(
        &mut self,
        name: &str,
        binary: &Path,
        arguments: Vec<String>,
        expected_command: &str,
        expected_status: &str,
    ) -> Result<Invocation, DevError> {
        self.admit_resources()?;
        let remaining = self
            .maximum_wall
            .checked_sub(self.started.elapsed())
            .ok_or_else(|| DevError::unavailable("scale maximum wall time was reached"))?;
        if remaining.is_zero() {
            return Err(DevError::unavailable("scale maximum wall time was reached"));
        }
        let ordinal = self.commands.len() as u64;
        let mut command = vec![binary.to_string_lossy().into_owned()];
        command.extend(arguments);
        let logs = self.evidence_root.join("logs");
        fs::create_dir_all(&logs).map_err(|error| {
            DevError::infrastructure(format!(
                "create scale log directory '{}': {error}",
                logs.display()
            ))
        })?;
        let stdout_path = logs.join(format!("{ordinal:06}-{name}.stdout.log"));
        let stderr_path = logs.join(format!("{ordinal:06}-{name}.stderr.log"));
        let timeout = remaining.min(Duration::from_secs(MAXIMUM_COMMAND_SECONDS));
        let observation = process::run(
            &ProcessSpec {
                command: command.clone(),
                cwd: self.repository.clone(),
                environment: process::environment(),
                timeout,
                maximum_stdout_bytes: MAXIMUM_STDOUT_BYTES,
                maximum_stderr_bytes: MAXIMUM_STDERR_BYTES,
                stdout_path: stdout_path.clone(),
                stderr_path: stderr_path.clone(),
                unavailable_exit_code: None,
            },
            &self.repository,
        );
        let response = process::read_bounded(&stdout_path, MAXIMUM_STDOUT_BYTES);
        let response_digest = response
            .as_ref()
            .ok()
            .map(|bytes| VerificationDigest::of(bytes));
        self.commands.push(CommandEvidence {
            ordinal,
            name: name.to_owned(),
            command,
            classification: observation.status,
            response_digest,
            response_records: None,
            process: observation.clone(),
        });
        self.admit_resources()?;
        if observation.status == ProcessStatus::Timeout {
            return Err(DevError::unavailable(format!(
                "public command '{name}' reached its scale wall-time allocation"
            )));
        }
        if observation.status != ProcessStatus::Passed {
            return Err(DevError::infrastructure(format!(
                "public command '{name}' ended as {:?}: {}",
                observation.status,
                observation.reason.as_deref().unwrap_or("unknown")
            )));
        }
        if observation.stderr.bytes.unwrap_or(0) != 0 {
            let excerpt = process::excerpt(&stderr_path, 512)
                .unwrap_or_else(|_| "stderr unavailable".to_owned());
            return Err(DevError::infrastructure(format!(
                "public command '{name}' wrote stderr: {excerpt}"
            )));
        }
        let response = response?;
        let records = decode_response(
            name,
            &stdout_path.to_string_lossy(),
            &response,
            expected_command,
            expected_status,
        )?;
        self.commands[ordinal as usize].response_records = Some(records.len() as u64);
        Ok(Invocation { ordinal, records })
    }

    pub(crate) fn admit_resources(&self) -> Result<(), DevError> {
        if self.started.elapsed() >= self.maximum_wall {
            return Err(DevError::unavailable("scale maximum wall time was reached"));
        }
        let bytes = directory_bytes(&self.evidence_root)?;
        if bytes > self.maximum_run_bytes {
            return Err(DevError::unavailable(format!(
                "scale run uses {bytes} bytes, exceeding its {}-byte local-space budget",
                self.maximum_run_bytes
            )));
        }
        Ok(())
    }

    pub(crate) fn into_commands(self) -> Vec<CommandEvidence> {
        self.commands
    }
}

fn decode_response(
    name: &str,
    path: &str,
    response: &[u8],
    expected_command: &str,
    expected_status: &str,
) -> Result<Vec<CompactRecord>, DevError> {
    if response.last() != Some(&b'\n') {
        return Err(DevError::corrupt(format!(
            "public command '{name}' returned an incomplete compact response"
        )));
    }
    let records = parse_records(path, response).map_err(|errors| {
        let first = errors
            .first()
            .map_or("unknown", |error| error.code.as_str());
        DevError::corrupt(format!(
            "public command '{name}' returned invalid compact records: {first}"
        ))
    })?;
    require_result(&records, expected_command, expected_status)?;
    Ok(records)
}

pub(crate) fn require_result(
    records: &[CompactRecord],
    command: &str,
    status: &str,
) -> Result<(), DevError> {
    let result = records
        .iter()
        .find(|record| record.operation == "result")
        .ok_or_else(|| DevError::corrupt("compact response omitted its result record"))?;
    if result.operation != "result"
        || field(result, "command") != Some(command)
        || field(result, "status") != Some(status)
    {
        return Err(DevError::corrupt(format!(
            "compact response did not identify {command} as {status}"
        )));
    }
    Ok(())
}

pub(crate) fn record<'a>(
    records: &'a [CompactRecord],
    operation: &str,
) -> Result<&'a CompactRecord, DevError> {
    records
        .iter()
        .find(|record| record.operation == operation)
        .ok_or_else(|| DevError::corrupt(format!("compact response omitted '{operation}'")))
}

pub(crate) fn field<'a>(record: &'a CompactRecord, name: &str) -> Option<&'a str> {
    record
        .fields
        .iter()
        .find(|field| field.name == name)
        .map(|field| field.value.as_str())
}

pub(crate) fn required_field<'a>(
    record: &'a CompactRecord,
    name: &str,
) -> Result<&'a str, DevError> {
    field(record, name).ok_or_else(|| {
        DevError::corrupt(format!(
            "compact record '{}' omitted '{name}'",
            record.operation
        ))
    })
}

pub(crate) fn u64_field(record: &CompactRecord, name: &str) -> Result<u64, DevError> {
    required_field(record, name)?.parse().map_err(|_| {
        DevError::corrupt(format!(
            "compact record '{}.{}' is not an unsigned integer",
            record.operation, name
        ))
    })
}

pub(crate) fn bool_field(record: &CompactRecord, name: &str) -> Result<bool, DevError> {
    match required_field(record, name)? {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(DevError::corrupt(format!(
            "compact record '{}.{}' is not a boolean",
            record.operation, name
        ))),
    }
}

pub(crate) fn directory_bytes(path: &Path) -> Result<u64, DevError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        DevError::infrastructure(format!("inspect scale path '{}': {error}", path.display()))
    })?;
    if metadata.file_type().is_symlink() {
        return Err(DevError::infrastructure(format!(
            "scale path '{}' is a symlink",
            path.display()
        )));
    }
    if metadata.is_file() {
        return Ok(metadata.len());
    }
    if !metadata.is_dir() {
        return Err(DevError::infrastructure(format!(
            "scale path '{}' is not a file or directory",
            path.display()
        )));
    }
    let mut bytes = 0_u64;
    for entry in fs::read_dir(path).map_err(|error| {
        DevError::infrastructure(format!(
            "read scale directory '{}': {error}",
            path.display()
        ))
    })? {
        let entry = entry.map_err(|error| {
            DevError::infrastructure(format!("read scale directory entry: {error}"))
        })?;
        bytes = bytes
            .checked_add(directory_bytes(&entry.path())?)
            .ok_or_else(|| DevError::infrastructure("scale byte count overflow"))?;
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::decode_response;

    #[test]
    fn result_may_follow_work_records_but_is_exact() {
        let records = decode_response(
            "check",
            "fixture",
            b"authority revision=rev_fixture\nresult status=success command=check\n",
            "check",
            "success",
        )
        .expect("current check response");
        assert_eq!(records.len(), 2);
        assert!(
            decode_response(
                "check",
                "fixture",
                b"result status=success command=build\n",
                "check",
                "success",
            )
            .is_err()
        );
    }

    #[test]
    fn incomplete_compact_output_is_one_clear_failure() {
        let error = decode_response(
            "status",
            "fixture",
            b"result status=success command=status",
            "status",
            "success",
        )
        .expect_err("truncated response must fail");
        assert_eq!(error.kind(), "corrupt");
        assert!(error.message().contains("incomplete compact response"));
    }
}
