//! Keep the direct child waitable until the last group signal; never signal a reaped PID.
use super::{POLL_INTERVAL, descendants::Descendants};
use crate::error::DevError;
use rustix::process::{Pid, Signal, WaitId, WaitIdOptions, kill_process_group, waitid};
use std::process::{Child, ExitStatus};
use std::time::Instant;

pub(super) struct Owned {
    child: Child,
    group: Pid,
    descendants: Descendants,
}

impl Owned {
    pub(super) fn new(child: Child) -> Self {
        Self {
            group: Pid::from_child(&child),
            descendants: Descendants::new(child.id()),
            child,
        }
    }
    pub(super) fn id(&self) -> u32 {
        self.child.id()
    }
    pub(super) fn sample(&mut self) -> Result<(), DevError> {
        self.descendants.sample()
    }
    pub(super) fn exited(&self) -> Result<bool, DevError> {
        match waitid(
            WaitId::Pid(self.group),
            WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT,
        ) {
            Ok(status) => Ok(status.is_some()),
            Err(rustix::io::Errno::INTR) => Ok(false),
            Err(error) => Err(DevError::infrastructure(format!(
                "observe waitable child: {error}"
            ))),
        }
    }
    pub(super) fn signal(&self, signal: Signal) -> Result<(), DevError> {
        match kill_process_group(self.group, signal) {
            Ok(()) | Err(rustix::io::Errno::SRCH) => Ok(()),
            Err(error) => Err(DevError::infrastructure(format!(
                "signal owned process group: {error}"
            ))),
        }
    }
    // Consuming self makes group signaling after reap unavailable to callers.
    pub(super) fn finish(mut self, deadline: Instant) -> Result<(ExitStatus, bool), DevError> {
        let mut failures = Vec::new();
        remember(self.descendants.sample(), &mut failures);
        let mut survivors = remember(self.descendants.has_live(), &mut failures).unwrap_or(false);
        remember(self.descendants.terminate(), &mut failures);
        remember(self.signal(Signal::KILL), &mut failures);
        // A direct child may have changed its group. Its unreaped PID is still ours.
        if let Err(error) = self.child.kill()
            && error.raw_os_error() != Some(rustix::io::Errno::SRCH.raw_os_error())
        {
            failures.push(format!("kill direct child: {error}"));
        }
        survivors |= remember(self.descendants.finish(deadline), &mut failures).unwrap_or(false);
        // This is the first reaping operation. No group signal follows it, even on error.
        let status = loop {
            match self.child.try_wait() {
                Ok(Some(status)) => break Some(status),
                Ok(None) if Instant::now() < deadline => std::thread::sleep(POLL_INTERVAL),
                Ok(None) => {
                    failures.push("direct child did not terminate during cleanup".to_owned());
                    break None;
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {
                    if Instant::now() >= deadline {
                        failures.push("child reap interrupted past cleanup deadline".to_owned());
                        break None;
                    }
                }
                Err(error) => {
                    failures.push(format!("reap direct child: {error}"));
                    break None;
                }
            }
        };
        if !failures.is_empty() {
            return Err(DevError::infrastructure(failures.join("; ")));
        }
        status
            .map(|status| (status, survivors))
            .ok_or_else(|| DevError::infrastructure("missing child completion"))
    }
}

fn remember<T>(result: Result<T, DevError>, failures: &mut Vec<String>) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(error) => {
            failures.push(error.to_string());
            None
        }
    }
}
