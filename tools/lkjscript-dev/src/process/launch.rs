//! Spawn a child with the declared command and owned process group.
use super::ProcessSpec;
use std::io;
use std::process::{Child, Command, Stdio};

#[cfg(target_os = "linux")]
pub(super) fn spawn(
    specification: &ProcessSpec,
    stdin: Stdio,
    stdout: Stdio,
    stderr: Stdio,
) -> io::Result<Child> {
    use std::os::unix::process::CommandExt;
    Command::new(&specification.command[0])
        .args(&specification.command[1..])
        .current_dir(&specification.cwd)
        .stdin(stdin)
        .stdout(stdout)
        .stderr(stderr)
        .env_clear()
        .envs(&specification.environment)
        .process_group(0)
        .spawn()
}
