//! Bounded cleanup of owned verifier descendants, including children with separate process groups.
use crate::error::DevError;
use rustix::process::{Pid, PidfdFlags, Signal, pidfd_open, pidfd_send_signal};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::Path;
use std::time::{Duration, Instant};

const MAXIMUM_PROCESSES: usize = 4096;
const MAXIMUM_DEPTH: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Identity {
    pid: u32,
    started: u64,
}

pub(super) struct Descendants {
    root: u32,
    root_identity: Option<Identity>,
    known: BTreeMap<u32, Identity>,
}

impl Descendants {
    pub(super) fn new(root: u32) -> Self {
        Self {
            root,
            root_identity: observe(root).ok().flatten().map(|(identity, _)| identity),
            known: BTreeMap::new(),
        }
    }
    pub(super) fn sample(&mut self) -> Result<(), DevError> {
        self.discover(false)
    }
    fn discover(&mut self, stop: bool) -> Result<(), DevError> {
        let mut pending = vec![(self.root, 0)];
        let mut visited = BTreeSet::new();
        while let Some((pid, depth)) = pending.pop() {
            if !visited.insert(pid) {
                continue;
            }
            if visited.len() > MAXIMUM_PROCESSES || depth > MAXIMUM_DEPTH {
                return Err(DevError::infrastructure(
                    "owned descendant inventory exhausted",
                ));
            }
            let Some((identity, live)) = observe(pid)? else {
                continue;
            };
            if pid == self.root && self.root_identity != Some(identity) {
                continue;
            }
            if !live {
                continue;
            }
            if self.known.get(&pid).is_some_and(|known| *known != identity) {
                continue;
            }
            if pid != self.root {
                if self.known.len() >= MAXIMUM_PROCESSES {
                    return Err(DevError::infrastructure(
                        "owned process inventory exhausted",
                    ));
                }
                self.known.insert(pid, identity);
            }
            if stop {
                signal(identity, Signal::STOP)?;
            }
            // A verifier may spawn a runner from a worker thread. Linux records children on
            // their creating task, so visiting only the main task would miss that owned runner.
            let tasks = match fs::read_dir(format!("/proc/{pid}/task")) {
                Ok(tasks) => tasks,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error.into()),
            };
            for (count, task) in tasks.take(MAXIMUM_PROCESSES + 1).enumerate() {
                if count == MAXIMUM_PROCESSES {
                    return Err(DevError::infrastructure("owned thread inventory exhausted"));
                }
                let task = task?;
                let Some(children) = read(&task.path().join("children"), 65536)? else {
                    continue;
                };
                let children = std::str::from_utf8(&children)
                    .map_err(|_| DevError::corrupt("non-UTF-8 process children"))?;
                for child in children.split_whitespace() {
                    if pending
                        .len()
                        .checked_add(visited.len())
                        .is_none_or(|count| count >= MAXIMUM_PROCESSES)
                    {
                        return Err(DevError::infrastructure(
                            "owned descendant traversal exhausted",
                        ));
                    }
                    let child = child
                        .parse::<u32>()
                        .map_err(|_| DevError::corrupt("invalid child PID"))?;
                    pending.push((child, depth + 1));
                }
            }
        }
        Ok(())
    }
    pub(super) fn terminate(&mut self) -> Result<(), DevError> {
        // Stop parents before discovering children so a terminating tree cannot keep forking.
        let discovered = self.discover(true);
        let mut failure = discovered.err();
        for identity in self.known.values().rev() {
            if let Err(error) = signal(*identity, Signal::KILL) {
                failure = Some(error);
            }
        }
        if let Some(error) = failure {
            Err(error)
        } else {
            Ok(())
        }
    }
    pub(super) fn finish(&mut self) -> Result<bool, DevError> {
        let mut survivors = false;
        for identity in self.known.values() {
            survivors |=
                observe(identity.pid)?.is_some_and(|(current, live)| current == *identity && live);
        }
        if !survivors {
            return Ok(false);
        }
        self.terminate()?;
        let started = Instant::now();
        loop {
            let mut live = false;
            for identity in self.known.values() {
                if observe(identity.pid)?
                    .is_some_and(|(current, running)| current == *identity && running)
                {
                    live = true;
                }
            }
            if !live {
                return Ok(true);
            }
            if started.elapsed() > Duration::from_secs(5) {
                return Err(DevError::infrastructure(
                    "owned descendants did not terminate",
                ));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

fn read(path: &Path, maximum: u64) -> Result<Option<Vec<u8>>, DevError> {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let mut bytes = Vec::new();
    file.take(maximum + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > maximum {
        return Err(DevError::corrupt("process inventory input exhausted"));
    }
    Ok(Some(bytes))
}
fn observe(pid: u32) -> Result<Option<(Identity, bool)>, DevError> {
    let Some(bytes) = read(Path::new(&format!("/proc/{pid}/stat")), 8192)? else {
        return Ok(None);
    };
    let text =
        std::str::from_utf8(&bytes).map_err(|_| DevError::corrupt("invalid process stat"))?;
    let tail = text
        .rsplit_once(") ")
        .ok_or_else(|| DevError::corrupt("invalid process stat fields"))?
        .1;
    let mut fields = tail.split_whitespace();
    let state = fields
        .next()
        .ok_or_else(|| DevError::corrupt("missing process state"))?;
    let started = fields
        .nth(18)
        .ok_or_else(|| DevError::corrupt("missing process start identity"))?
        .parse::<u64>()
        .map_err(|_| DevError::corrupt("invalid process start identity"))?;
    Ok(Some((
        Identity { pid, started },
        !matches!(state, "Z" | "X"),
    )))
}
fn signal(identity: Identity, signal: Signal) -> Result<(), DevError> {
    let pid =
        Pid::from_raw(i32::try_from(identity.pid).map_err(|_| DevError::corrupt("PID overflow"))?)
            .ok_or_else(|| DevError::corrupt("zero PID"))?;
    let fd = match pidfd_open(pid, PidfdFlags::empty()) {
        Ok(fd) => fd,
        Err(rustix::io::Errno::SRCH) => return Ok(()),
        Err(error) => {
            return Err(DevError::infrastructure(format!(
                "open owned process identity: {error}"
            )));
        }
    };
    if observe(identity.pid)?.is_some_and(|(current, live)| current == identity && live) {
        match pidfd_send_signal(&fd, signal) {
            Ok(()) => (),
            Err(rustix::io::Errno::SRCH) => (),
            Err(error) => {
                return Err(DevError::infrastructure(format!(
                    "signal owned child: {error}"
                )));
            }
        }
    }
    Ok(())
}
