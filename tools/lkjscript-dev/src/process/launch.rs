//! Select a child program without changing its declared argv[0] or process ownership.
use super::ProcessSpec;
use std::io;
use std::path::Path;
use std::process::{Child, Command, Stdio};

#[derive(Clone, Copy, Default)]
pub(super) enum Program<'a> {
    #[default]
    Search,
    Selected(Option<&'a Path>),
}

#[cfg(target_os = "linux")]
pub(super) fn spawn(
    specification: &ProcessSpec,
    stdin: Stdio,
    stdout: Stdio,
    stderr: Stdio,
    program: Program<'_>,
) -> io::Result<Child> {
    use std::os::unix::process::CommandExt;
    let mut command = match program {
        Program::Search => Command::new(&specification.command[0]),
        Program::Selected(Some(path)) if path.is_absolute() => Command::new(path),
        Program::Selected(Some(_)) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "selected child program must be absolute",
            ));
        }
        Program::Selected(None) => return Err(io::ErrorKind::NotFound.into()),
    };
    command
        .arg0(&specification.command[0])
        .args(&specification.command[1..])
        .current_dir(&specification.cwd)
        .stdin(stdin)
        .stdout(stdout)
        .stderr(stderr)
        .env_clear()
        .envs(&specification.environment)
        .process_group(0);
    command.spawn()
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::super::{ProcessStatus, run_selected};
    use super::*;
    use std::collections::BTreeMap;
    use std::fs;
    use std::time::Duration;

    fn specification(root: &Path, name: &str, command: &[&str]) -> ProcessSpec {
        ProcessSpec {
            command: command.iter().map(|value| (*value).to_owned()).collect(),
            cwd: root.to_path_buf(),
            environment: BTreeMap::new(),
            timeout: Duration::from_secs(2),
            maximum_stdout_bytes: 1_024,
            maximum_stderr_bytes: 1_024,
            stdout_path: root.join(format!("{name}.stdout")),
            stderr_path: root.join(format!("{name}.stderr")),
            unavailable_exit_code: None,
        }
    }

    #[test]
    fn selected_program_keeps_declared_argv_zero_and_does_not_search() {
        let root = tempfile::tempdir().expect("owned program binding");
        let spec = specification(
            root.path(),
            "argv",
            &["declared-command-not-on-path", "-c", "printf '%s' \"$0\""],
        );
        let result = run_selected(&spec, root.path(), Some(Path::new("/bin/sh")));
        assert_eq!(result.status, ProcessStatus::Passed);
        assert_eq!(
            fs::read(&spec.stdout_path).expect("declared argv[0]"),
            b"declared-command-not-on-path"
        );
    }

    #[test]
    fn missing_selection_cannot_fall_back_to_the_declared_program() {
        let root = tempfile::tempdir().expect("owned absent selection");
        let spec = specification(root.path(), "missing", &["/bin/true"]);
        let result = run_selected(&spec, root.path(), None);
        assert_eq!(result.status, ProcessStatus::Unavailable);
        assert_eq!(result.reason.as_deref(), Some("command_not_found"));
        assert_eq!(result.exit_code, None);
        let spec = specification(root.path(), "relative", &["/bin/true"]);
        let result = run_selected(&spec, root.path(), Some(Path::new("relative")));
        assert_eq!(result.status, ProcessStatus::InfrastructureFailure);
    }

    #[test]
    fn selected_program_uses_the_existing_timeout_and_output_owner() {
        let root = tempfile::tempdir().expect("owned selected process bounds");
        let mut spec = specification(root.path(), "timeout", &["selected-sleep", "5"]);
        spec.timeout = Duration::from_millis(30);
        let result = run_selected(&spec, root.path(), Some(Path::new("/bin/sleep")));
        assert_eq!(result.status, ProcessStatus::Timeout);
        let spec = specification(root.path(), "output", &["selected-yes"]);
        let result = run_selected(&spec, root.path(), Some(Path::new("/usr/bin/yes")));
        assert_eq!(result.status, ProcessStatus::OutputExhausted);
        assert!(result.stdout.bytes.is_some_and(|bytes| bytes <= 1_024));
        assert!(result.stdout_limit_exhausted);
    }
}
