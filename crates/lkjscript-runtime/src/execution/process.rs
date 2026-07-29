use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

use lkjscript_core::ExecutionOutcome;

use super::protocol::{
    read_response, write_bootstrap, write_request, ProcessBootstrap, ProcessRequest,
    ProcessResponse,
};
use crate::state::IsolatedProcessSpec;

const START_TIMEOUT: Duration = Duration::from_secs(2);
const STOP_TIMEOUT: Duration = Duration::from_secs(2);

pub(crate) struct ProcessCell {
    child: Child,
    input: ChildStdin,
    output: Option<ChildStdout>,
    process: u32,
}

impl ProcessCell {
    pub(crate) fn start(
        spec: &IsolatedProcessSpec,
        bootstrap: &ProcessBootstrap,
    ) -> Result<Self, String> {
        let mut child = Command::new(&spec.worker)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("spawn process cell: {error}"))?;
        let mut input = match child.stdin.take() {
            Some(input) => input,
            None => {
                terminate(&mut child);
                return Err("process cell stdin was not piped".into());
            }
        };
        let output = match child.stdout.take() {
            Some(output) => output,
            None => {
                terminate(&mut child);
                return Err("process cell stdout was not piped".into());
            }
        };
        if let Err(error) = write_bootstrap(&mut input, bootstrap) {
            terminate(&mut child);
            return Err(format!("write process bootstrap: {error}"));
        }
        let (output, response) = match timed_read(output, START_TIMEOUT, &mut child) {
            Ok(value) => value,
            Err(error) => {
                terminate(&mut child);
                return Err(error);
            }
        };
        let process = match response {
            ProcessResponse::Ready { process } if process == child.id() => process,
            ProcessResponse::Ready { .. } => {
                terminate(&mut child);
                return Err("process cell ready identity mismatch".into());
            }
            ProcessResponse::ReadyFailure { diagnostic } => {
                terminate(&mut child);
                return Err(format!("process cell bootstrap failed: {diagnostic}"));
            }
            _ => {
                terminate(&mut child);
                return Err("process cell sent unexpected bootstrap response".into());
            }
        };
        Ok(Self {
            child,
            input,
            output: Some(output),
            process,
        })
    }

    pub(crate) const fn process(&self) -> u32 {
        self.process
    }

    pub(crate) fn invoke(
        &mut self,
        cell: u64,
        arguments: Vec<String>,
    ) -> Result<(ExecutionOutcome, Vec<u8>, u64), String> {
        self.ensure_running()?;
        write_request(&mut self.input, &ProcessRequest::Invoke { cell, arguments })
            .map_err(|error| self.fail(format!("write process invocation: {error}")))?;
        let response = self.read().map_err(|error| self.fail(error))?;
        match response {
            ProcessResponse::Outcome {
                cell: received,
                outcome,
                output,
                flushes,
            } if received == cell => Ok((outcome, output, flushes)),
            ProcessResponse::Outcome { .. } => {
                Err(self.fail("process outcome identity mismatch".into()))
            }
            _ => Err(self.fail("unexpected process invocation response".into())),
        }
    }

    pub(crate) fn stop(&mut self) -> Result<(), String> {
        if self
            .child
            .try_wait()
            .map_err(|error| error.to_string())?
            .is_some()
        {
            return Ok(());
        }
        if let Err(error) = write_request(&mut self.input, &ProcessRequest::Stop) {
            terminate(&mut self.child);
            return Err(format!("write process stop: {error}"));
        }
        let output = self
            .output
            .take()
            .ok_or_else(|| "process output is unavailable".to_string())?;
        let (output, response) = timed_read(output, STOP_TIMEOUT, &mut self.child)?;
        self.output = Some(output);
        if response != ProcessResponse::Stopped {
            terminate(&mut self.child);
            return Err("unexpected process stop response".into());
        }
        wait_or_terminate(&mut self.child, STOP_TIMEOUT)?;
        Ok(())
    }

    fn read(&mut self) -> Result<ProcessResponse, String> {
        let output = self
            .output
            .as_mut()
            .ok_or_else(|| "process output is unavailable".to_string())?;
        read_response(output).map_err(|error| format!("read process response: {error}"))
    }

    fn ensure_running(&mut self) -> Result<(), String> {
        match self.child.try_wait().map_err(|error| error.to_string())? {
            Some(status) => Err(format!("process cell exited unexpectedly: {status}")),
            None => Ok(()),
        }
    }

    fn fail(&mut self, message: String) -> String {
        terminate(&mut self.child);
        message
    }
}

impl Drop for ProcessCell {
    fn drop(&mut self) {
        terminate(&mut self.child);
    }
}

include!("process/wait.rs");
