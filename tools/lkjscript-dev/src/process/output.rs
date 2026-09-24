//! Fair, nonblocking ownership of a child pipe and its bounded retained log.
use super::{POLL_INTERVAL, READ_CHUNK_BYTES, prepare_log};
use crate::error::DevError;
use rustix::fs::{OFlags, fcntl_getfl, fcntl_setfl};
use std::fs::File;
use std::io::{self, PipeReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Instant;

pub(super) struct Output {
    input: Option<PipeReader>,
    log: File,
    path: PathBuf,
    maximum: u64,
    retained: u64,
    pub(super) exhausted: bool,
}

impl Output {
    pub(super) fn new(path: &Path, maximum: u64, closed: bool) -> Result<(Self, Stdio), DevError> {
        let log = prepare_log(path)?;
        let (input, writer) = io::pipe()?;
        let flags = fcntl_getfl(&input).map_err(io::Error::from)?;
        fcntl_setfl(&input, flags | OFlags::NONBLOCK).map_err(io::Error::from)?;
        // A broken-output fixture must close the reader BEFORE the child can write.
        let input = if closed {
            drop(input);
            None
        } else {
            Some(input)
        };
        Ok((
            Self {
                input,
                log,
                path: path.to_path_buf(),
                maximum,
                retained: 0,
                exhausted: false,
            },
            Stdio::from(writer),
        ))
    }

    pub(super) fn complete(&self) -> bool {
        self.input.is_none()
    }

    pub(super) fn drain(&mut self) -> Result<bool, DevError> {
        let Some(input) = &mut self.input else {
            return Ok(false);
        };
        let mut buffer = [0_u8; READ_CHUNK_BYTES];
        let mut progressed = false;
        // A continuously writable stream cannot monopolize deadline/control observation
        // or starve the other stream. Interrupted reads also consume a turn.
        for _ in 0..4 {
            let count = match input.read(&mut buffer) {
                Ok(0) => {
                    self.input = None;
                    return Ok(true);
                }
                Ok(count) => count,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => {
                    return Err(DevError::infrastructure(format!(
                        "read child pipe for '{}': {error}",
                        self.path.display()
                    )));
                }
            };
            progressed = true;
            let keep = self.maximum.saturating_sub(self.retained).min(count as u64) as usize;
            self.exhausted |= keep != count;
            self.log.write_all(&buffer[..keep]).map_err(|error| {
                DevError::infrastructure(format!(
                    "write child log '{}': {error}",
                    self.path.display()
                ))
            })?;
            // retained + keep <= maximum, even when maximum == u64::MAX.
            self.retained += keep as u64;
        }
        Ok(progressed)
    }

    pub(super) fn synchronize(&self) -> Result<(), DevError> {
        self.log.sync_all().map_err(|error| {
            DevError::infrastructure(format!(
                "synchronize child log '{}': {error}",
                self.path.display()
            ))
        })
    }
}

pub(super) fn finish(
    stdout: &mut Output,
    stderr: &mut Output,
    deadline: Instant,
) -> Result<(), DevError> {
    loop {
        // Service both even if one reports an error.
        let left = stdout.drain();
        let right = stderr.drain();
        let progressed = left? | right?;
        if stdout.complete() && stderr.complete() {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(DevError::infrastructure(format!(
                "child output did not reach EOF after cleanup: stdout={}, stderr={}",
                stdout.complete(),
                stderr.complete()
            )));
        }
        if !progressed {
            std::thread::sleep(POLL_INTERVAL);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn a_writer_outside_process_ownership_cannot_block_stream_cleanup() {
        let root = tempfile::tempdir().unwrap();
        let (mut stdout, held_writer) = Output::new(&root.path().join("out"), 16, false).unwrap();
        let (mut stderr, closed_writer) = Output::new(&root.path().join("err"), 16, false).unwrap();
        drop(closed_writer);
        let started = Instant::now();
        let result = finish(
            &mut stdout,
            &mut stderr,
            started + Duration::from_millis(20),
        );
        assert!(result.unwrap_err().message().contains("did not reach EOF"));
        assert!(started.elapsed() < Duration::from_millis(500));
        assert!(!stdout.complete());
        assert!(stderr.complete());
        drop(held_writer);
        finish(
            &mut stdout,
            &mut stderr,
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap();
    }

    #[test]
    fn log_write_failure_still_terminates_and_reaps_owned_processes() {
        use super::super::{ProcessSpec, ProcessStatus, launch, supervisor};
        let root = tempfile::tempdir().unwrap();
        let spec = ProcessSpec {
            command: vec![
                "/bin/sh".into(),
                "-c".into(),
                "sleep 0.8 & printf '%s %s' $$ $! > ids; sleep 0.04; printf x; wait".into(),
            ],
            cwd: root.path().to_path_buf(),
            environment: std::collections::BTreeMap::from([(
                "PATH".into(),
                "/usr/bin:/bin".into(),
            )]),
            timeout: Duration::from_secs(2),
            maximum_stdout_bytes: 16,
            maximum_stderr_bytes: 16,
            stdout_path: root.path().join("stdout"),
            stderr_path: root.path().join("stderr"),
            unavailable_exit_code: None,
        };
        let (mut stdout, out) = Output::new(&spec.stdout_path, 16, false).unwrap();
        stdout.log = std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/full")
            .unwrap();
        let (stderr, err) = Output::new(&spec.stderr_path, 16, false).unwrap();
        let child = launch::spawn(&spec, Stdio::null(), out, err, launch::Program::Search).unwrap();
        let result = supervisor::run(child, stdout, stderr, &spec, Instant::now(), None);
        assert_eq!(
            result.completion.status,
            ProcessStatus::InfrastructureFailure
        );
        assert!(
            result
                .completion
                .reason
                .unwrap()
                .contains("write child log")
        );
        let ids = std::fs::read_to_string(root.path().join("ids")).unwrap();
        for pid in ids.split_whitespace() {
            if let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) {
                assert!(stat.rsplit_once(") ").unwrap().1.starts_with('Z'));
            }
        }
    }
}
