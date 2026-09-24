use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdout, Command, Output, Stdio};
use std::thread;
use std::time::Duration;

const EXECUTABLE_BUSY_ATTEMPTS: usize = 12;
const EXECUTABLE_BUSY_DELAY: Duration = Duration::from_millis(50);

pub fn copy_executable(source: &Path, destination: &Path) {
    let stage = destination.with_extension("stage");
    let mut input = File::open(source).expect("open executable for isolated copy");
    let permissions = input
        .metadata()
        .expect("inspect executable permissions")
        .permissions();
    let mut output = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&stage)
        .expect("create private executable stage");
    // File-to-file copy offload has produced zeroed executable ranges in the host
    // filesystem. Read the actual bytes and verify the closed stage before execution;
    // a started child is never retried to conceal a damaged copy.
    let mut expected = blake3::Hasher::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = match input.read(&mut buffer) {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            result => result.expect("read executable bytes"),
        };
        if count == 0 {
            break;
        }
        expected.update(&buffer[..count]);
        output
            .write_all(&buffer[..count])
            .expect("write executable bytes into private stage");
    }
    output
        .set_permissions(permissions)
        .expect("preserve executable permissions");
    output.sync_all().expect("synchronize executable stage");
    drop(output);
    drop(input);
    let mut copied = File::open(&stage).expect("open closed executable stage");
    let mut observed = blake3::Hasher::new();
    loop {
        let count = match copied.read(&mut buffer) {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            result => result.expect("read staged executable bytes"),
        };
        if count == 0 {
            break;
        }
        observed.update(&buffer[..count]);
    }
    assert_eq!(
        observed.finalize(),
        expected.finalize(),
        "copied executable must retain every source byte"
    );
    drop(copied);
    std::fs::rename(stage, destination).expect("publish closed executable copy");
    File::open(destination.parent().expect("copied executable parent"))
        .and_then(|directory| directory.sync_all())
        .expect("synchronize copied executable visibility");
}

pub fn output(command: &mut Command) -> io::Result<Output> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    output_with(
        || command.spawn(),
        SpawnedChild::wait_with_output,
        || thread::sleep(EXECUTABLE_BUSY_DELAY),
    )
}

pub fn spawn(command: &mut Command) -> io::Result<SpawnedChild> {
    spawn_with(|| command.spawn(), || thread::sleep(EXECUTABLE_BUSY_DELAY))
}

fn output_with(
    spawn_child: impl FnMut() -> io::Result<Child>,
    collect: impl FnOnce(SpawnedChild) -> io::Result<Output>,
    delay: impl FnMut(),
) -> io::Result<Output> {
    // The retry owner can only produce a child. Collection and every observation
    // of a started child occur once, outside the spawn-recovery boundary.
    collect(spawn_with(spawn_child, delay)?)
}

fn spawn_with(
    mut spawn_child: impl FnMut() -> io::Result<Child>,
    mut delay: impl FnMut(),
) -> io::Result<SpawnedChild> {
    let mut attempt = 1;
    loop {
        match spawn_child() {
            Err(error)
                if error.kind() == io::ErrorKind::ExecutableFileBusy
                    && attempt < EXECUTABLE_BUSY_ATTEMPTS =>
            {
                delay();
                attempt += 1;
            }
            result => {
                return result.map(|child| SpawnedChild {
                    child,
                    joined: false,
                });
            }
        }
    }
}

pub struct SpawnedChild {
    child: Child,
    joined: bool,
}

impl SpawnedChild {
    pub fn id(&self) -> u32 {
        self.child.id()
    }

    pub fn try_wait(&mut self) -> io::Result<Option<std::process::ExitStatus>> {
        let status = self.child.try_wait()?;
        self.joined |= status.is_some();
        Ok(status)
    }

    pub fn take_stdout(&mut self) -> Option<ChildStdout> {
        self.child.stdout.take()
    }

    pub fn kill(&mut self) -> io::Result<()> {
        self.child.kill()
    }

    pub fn wait_with_output(mut self) -> io::Result<Output> {
        drop(self.child.stdin.take());
        let stdout = self.child.stdout.take();
        let stderr = self.child.stderr.take();
        thread::scope(|scope| {
            let stdout_reader =
                thread::Builder::new().spawn_scoped(scope, move || read_pipe(stdout))?;
            let stderr_reader =
                match thread::Builder::new().spawn_scoped(scope, move || read_pipe(stderr)) {
                    Ok(reader) => reader,
                    Err(error) => {
                        self.stop_and_join();
                        return Err(error);
                    }
                };
            let status = self.child.wait();
            self.joined = status.is_ok();
            if !self.joined {
                self.stop_and_join();
            }
            // Join both readers before propagating any wait/read error. The child
            // stays owned, so early assertions in interactive callers also reap it.
            let stdout = stdout_reader
                .join()
                .map_err(|_| io::Error::other("stdout reader panicked"));
            let stderr = stderr_reader
                .join()
                .map_err(|_| io::Error::other("stderr reader panicked"));
            Ok(Output {
                status: status?,
                stdout: stdout??,
                stderr: stderr??,
            })
        })
    }

    fn stop_and_join(&mut self) {
        if !self.joined {
            let _ = self.child.kill();
            self.joined = self.child.wait().is_ok();
        }
    }
}

impl Drop for SpawnedChild {
    fn drop(&mut self) {
        self.stop_and_join();
    }
}

fn read_pipe(mut pipe: Option<impl Read>) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    if let Some(pipe) = &mut pipe {
        pipe.read_to_end(&mut bytes)?;
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};

    #[test]
    fn executable_copy_preserves_sparse_bytes_and_permissions() {
        let temporary = tempfile::tempdir().unwrap();
        let source = temporary.path().join("source");
        let destination = temporary.path().join("copy");
        let mut file = File::create(&source).unwrap();
        let mut expected = vec![0_u8; 2 * 1024 * 1024];
        for (offset, value, length) in [(0, 17, 8192), (1_048_543, 167, 8192), (2_097_151, 255, 1)]
        {
            expected[offset..offset + length].fill(value);
            file.seek(SeekFrom::Start(offset as u64)).unwrap();
            file.write_all(&expected[offset..offset + length]).unwrap();
        }
        file.sync_all().unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(std::fs::Permissions::from_mode(0o751))
                .unwrap();
        }
        drop(file);
        copy_executable(&source, &destination);
        assert_eq!(std::fs::read(&destination).unwrap(), expected);
        assert_eq!(std::fs::read(&source).unwrap(), expected);
        assert_eq!(
            std::fs::metadata(&destination).unwrap().permissions(),
            std::fs::metadata(&source).unwrap().permissions()
        );
        assert!(!destination.with_extension("stage").exists());
    }

    const EFFECT_FILE: &str = "LKJSCRIPT_TEST_SPAWN_EFFECT_FILE";
    const FAIL_AFTER_EFFECT: &str = "LKJSCRIPT_TEST_SPAWN_FAIL_AFTER_EFFECT";
    const WAIT_AFTER_EFFECT: &str = "LKJSCRIPT_TEST_SPAWN_WAIT_AFTER_EFFECT";

    fn effect_command(path: &Path) -> Command {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", "support::tests::once_only_child", "--nocapture"])
            .env_clear()
            .env(EFFECT_FILE, path)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }

    #[test]
    fn once_only_child() {
        let Some(path) = std::env::var_os(EFFECT_FILE) else {
            return;
        };
        let mut effect = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();
        writeln!(effect, "started").unwrap();
        effect.sync_all().unwrap();
        if std::env::var_os(WAIT_AFTER_EFFECT).is_some() {
            writeln!(std::io::stdout(), "effect-recorded").unwrap();
            std::io::stdin().read_exact(&mut [0]).unwrap();
        }
        assert!(std::env::var_os(FAIL_AFTER_EFFECT).is_none());
    }

    #[test]
    fn copied_binary_spawn_recovers_only_transient_busy_before_start() {
        let temporary = tempfile::tempdir().unwrap();
        let effect = temporary.path().join("effect");
        let mut command = effect_command(&effect);
        let mut attempts = 0;
        let mut delays = 0;
        let child = spawn_with(
            || {
                attempts += 1;
                if attempts < 3 {
                    Err(io::Error::from(io::ErrorKind::ExecutableFileBusy))
                } else {
                    command.spawn()
                }
            },
            || delays += 1,
        )
        .unwrap();
        assert!(child.wait_with_output().unwrap().status.success());
        assert_eq!(attempts, 3);
        assert_eq!(delays, 2);
        assert_eq!(std::fs::read(effect).unwrap(), b"started\n");
    }

    #[test]
    fn copied_binary_spawn_exhausts_busy_and_rejects_nonbusy_immediately() {
        for (kind, expected_attempts) in [
            (io::ErrorKind::ExecutableFileBusy, EXECUTABLE_BUSY_ATTEMPTS),
            (io::ErrorKind::PermissionDenied, 1),
        ] {
            let mut attempts = 0;
            let mut delays = 0;
            let result = spawn_with(
                || {
                    attempts += 1;
                    Err(io::Error::from(kind))
                },
                || delays += 1,
            );
            assert_eq!(result.err().unwrap().kind(), kind);
            assert_eq!(attempts, expected_attempts);
            assert_eq!(delays, expected_attempts - 1);
        }
    }

    #[test]
    fn copied_binary_spawn_does_not_replay_after_collection_or_exit_failure() {
        for collection_failure in [true, false] {
            let temporary = tempfile::tempdir().unwrap();
            let effect = temporary.path().join("effect");
            let mut command = effect_command(&effect);
            if !collection_failure {
                command.env(FAIL_AFTER_EFFECT, "1");
            }
            let mut attempts = 0;
            let mut delays = 0;
            let result = output_with(
                || {
                    attempts += 1;
                    command.spawn()
                },
                |child| {
                    let output = child.wait_with_output()?;
                    if collection_failure {
                        assert!(output.status.success());
                        // Deliberately use the retryable spawn error kind after a
                        // joined child's effect. It cannot authorize another spawn.
                        Err(io::Error::from(io::ErrorKind::ExecutableFileBusy))
                    } else {
                        Ok(output)
                    }
                },
                || delays += 1,
            );
            if collection_failure {
                assert_eq!(
                    result.unwrap_err().kind(),
                    io::ErrorKind::ExecutableFileBusy
                );
            } else {
                assert!(!result.unwrap().status.success());
            }
            assert_eq!(attempts, 1);
            assert_eq!(delays, 0);
            assert_eq!(std::fs::read(effect).unwrap(), b"started\n");
        }
    }

    #[test]
    fn copied_binary_spawn_joins_a_started_child_after_kill() {
        let temporary = tempfile::tempdir().unwrap();
        let effect = temporary.path().join("effect");
        let mut command = effect_command(&effect);
        command.env(WAIT_AFTER_EFFECT, "1").stdin(Stdio::piped());
        let mut child = spawn(&mut command).unwrap();
        let mut stdout = BufReader::new(child.take_stdout().unwrap());
        let mut ready = false;
        for _ in 0..8 {
            let mut line = String::new();
            assert_ne!(stdout.read_line(&mut line).unwrap(), 0);
            if line.contains("effect-recorded") {
                ready = true;
                break;
            }
        }
        assert!(ready);
        // The readiness handshake and retained stdin keep this child independently live.
        // Exercise the shared observation API in every integration-test crate using it.
        let expected_pid = child.child.id();
        assert_eq!(child.id(), expected_pid);
        assert_eq!(child.try_wait().unwrap(), None);
        assert!(!child.joined);
        child.kill().unwrap();
        drop(stdout);
        let expected_status = child.child.wait().unwrap();
        assert!(!expected_status.success());
        assert_eq!(child.try_wait().unwrap(), Some(expected_status));
        assert!(child.joined);
        assert_eq!(child.try_wait().unwrap(), Some(expected_status));
        assert_eq!(child.id(), expected_pid);
        assert_eq!(child.wait_with_output().unwrap().status, expected_status);
        assert_eq!(std::fs::read(effect).unwrap(), b"started\n");
    }
}
