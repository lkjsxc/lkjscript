//! One lifecycle for execution, stream completion, cancellation and joined cleanup.
use super::{
    CONTROL_KILL, CONTROL_NONE, CONTROL_TERMINATE, POLL_INTERVAL, ProcessCompletion,
    ProcessControl, ProcessResources, ProcessSpec, ProcessStatus, exhausted_reason,
    output::{self, Output},
    owned::Owned,
    sample_linux_process,
};
use crate::error::DevError;
use rustix::process::Signal;
use std::os::unix::process::ExitStatusExt;
use std::process::{Child, ExitStatus};
use std::time::{Duration, Instant};

const CLEANUP_TIMEOUT: Duration = Duration::from_secs(5);

pub(super) struct Finished {
    pub(super) completion: ProcessCompletion,
    pub(super) resources: ProcessResources,
    pub(super) stdout_exhausted: bool,
    pub(super) stderr_exhausted: bool,
}

pub(super) fn run(
    child: Child,
    mut stdout: Output,
    mut stderr: Output,
    spec: &ProcessSpec,
    started: Instant,
    control: Option<&ProcessControl>,
) -> Finished {
    let mut owned = Owned::new(child);
    let mut resources = ProcessResources::default();
    let primary = drive(
        &mut owned,
        &mut stdout,
        &mut stderr,
        spec,
        started,
        control,
        &mut resources,
    );
    let cleanup_deadline = Instant::now() + CLEANUP_TIMEOUT;
    // Cleanup always runs, including stream-read/write, observation and control errors.
    let joined = owned.finish(cleanup_deadline);
    let drained = output::finish(&mut stdout, &mut stderr, cleanup_deadline);
    let synced_stdout = stdout.synchronize();
    let synced_stderr = stderr.synchronize();
    let mut failures = Vec::new();
    let terminal = match primary {
        Ok(value) => value,
        Err(error) => {
            failures.push(error.to_string());
            None
        }
    };
    let (exit, survivors) = match joined {
        Ok((exit, survivors)) => (Some(exit), survivors),
        Err(error) => {
            failures.push(error.to_string());
            (None, false)
        }
    };
    for result in [drained, synced_stdout, synced_stderr] {
        if let Err(error) = result {
            failures.push(error.to_string());
        }
    }
    if survivors && terminal.is_none() {
        failures.push("owned descendants survived child exit and were terminated".to_owned());
    }
    let (mut status, mut reason) =
        classify(terminal, exit, spec, stdout.exhausted, stderr.exhausted);
    if !failures.is_empty() {
        if let Some(primary) = reason {
            failures.insert(0, format!("primary={primary}"));
        }
        status = ProcessStatus::InfrastructureFailure;
        reason = Some(failures.join("; "));
    }
    Finished {
        completion: ProcessCompletion {
            status,
            exit_code: exit.and_then(|value| value.code()),
            signal: exit.and_then(|value| value.signal()),
            reason,
        },
        resources,
        stdout_exhausted: stdout.exhausted,
        stderr_exhausted: stderr.exhausted,
    }
}

fn drive(
    owned: &mut Owned,
    stdout: &mut Output,
    stderr: &mut Output,
    spec: &ProcessSpec,
    started: Instant,
    control: Option<&ProcessControl>,
    resources: &mut ProcessResources,
) -> Result<Option<ProcessStatus>, DevError> {
    let mut sent_control = CONTROL_NONE;
    loop {
        sample_linux_process(owned.id(), resources);
        owned.sample()?;
        let exited = owned.exited()?;
        let left = stdout.drain();
        let right = stderr.drain();
        let progressed = left? | right?;
        if stdout.exhausted || stderr.exhausted {
            return Ok(Some(ProcessStatus::OutputExhausted));
        }
        if let Some(control) = control {
            let requested = control.requested();
            if requested >= CONTROL_KILL {
                return Ok(Some(ProcessStatus::Signaled));
            }
            if requested > sent_control {
                owned.signal(if requested == CONTROL_TERMINATE {
                    Signal::TERM
                } else {
                    Signal::INT
                })?;
                sent_control = requested;
            }
        }
        if exited && stdout.complete() && stderr.complete() {
            return Ok(None);
        }
        if started.elapsed() >= spec.timeout {
            return Ok(Some(ProcessStatus::Timeout));
        }
        if !progressed {
            std::thread::sleep(POLL_INTERVAL);
        }
    }
}

fn classify(
    terminal: Option<ProcessStatus>,
    exit: Option<ExitStatus>,
    spec: &ProcessSpec,
    stdout: bool,
    stderr: bool,
) -> (ProcessStatus, Option<String>) {
    match terminal {
        Some(ProcessStatus::OutputExhausted) => (
            ProcessStatus::OutputExhausted,
            Some(exhausted_reason(stdout, stderr)),
        ),
        Some(ProcessStatus::Timeout) => (ProcessStatus::Timeout, Some("timeout".to_owned())),
        Some(ProcessStatus::Signaled) => (ProcessStatus::Signaled, Some("control_kill".to_owned())),
        Some(_) => (
            ProcessStatus::InfrastructureFailure,
            Some("invalid terminal child-process state".to_owned()),
        ),
        None if stdout || stderr => (
            ProcessStatus::OutputExhausted,
            Some(exhausted_reason(stdout, stderr)),
        ),
        None if exit.is_some_and(|value| value.success()) => (ProcessStatus::Passed, None),
        None if spec
            .unavailable_exit_code
            .is_some_and(|code| exit.and_then(|value| value.code()) == Some(code)) =>
        {
            (
                ProcessStatus::Unavailable,
                Some("configured_unavailable_exit".to_owned()),
            )
        }
        None if exit.is_some_and(|value| value.signal().is_some()) => {
            (ProcessStatus::Signaled, Some("signal".to_owned()))
        }
        None => (ProcessStatus::Failed, Some("nonzero_exit".to_owned())),
    }
}
