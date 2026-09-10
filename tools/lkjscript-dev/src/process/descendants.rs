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
                Err(error) if disappeared(&error) => continue,
                Err(error) => return Err(error.into()),
            };
            for (count, task) in tasks.take(MAXIMUM_PROCESSES + 1).enumerate() {
                if count == MAXIMUM_PROCESSES {
                    return Err(DevError::infrastructure("owned thread inventory exhausted"));
                }
                let task = match task {
                    Ok(task) => task,
                    Err(error) if disappeared(&error) => continue,
                    Err(error) => return Err(error.into()),
                };
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
        Err(error) if disappeared(&error) => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    read_contents(file, maximum)
}

fn disappeared(error: &std::io::Error) -> bool {
    error.kind() == std::io::ErrorKind::NotFound
        || error.raw_os_error() == Some(rustix::io::Errno::SRCH.raw_os_error())
}

fn read_contents(file: impl Read, maximum: u64) -> Result<Option<Vec<u8>>, DevError> {
    let mut bytes = Vec::new();
    // procfs can open successfully and then report ESRCH when the observed task exits.
    // Discard partial observations; absence grants no authority over a reused PID.
    match file.take(maximum + 1).read_to_end(&mut bytes) {
        Ok(_) => (),
        Err(error) if disappeared(&error) => return Ok(None),
        Err(error) => return Err(error.into()),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, Cursor};
    use std::process::{Command, Stdio};

    #[test]
    fn opened_proc_stat_of_reaped_child_is_absent() {
        let mut child = Command::new("/bin/sleep")
            .arg("60")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("owned child");
        let opened = fs::File::open(format!("/proc/{}/stat", child.id()));
        child.kill().expect("terminate owned child");
        child.wait().expect("reap owned child");
        let file = opened.expect("open proc stat before exit");
        let mut raw = &file;
        let error = raw
            .read_to_end(&mut Vec::new())
            .expect_err("reaped procfs task");
        assert_eq!(
            error.raw_os_error(),
            Some(rustix::io::Errno::SRCH.raw_os_error())
        );
        assert_eq!(read_contents(file, 8192).expect("vanished task"), None);
    }

    #[test]
    fn partial_disappearance_discards_bytes_but_other_errors_and_bounds_reject() {
        struct Ending {
            prefix: Cursor<Vec<u8>>,
            error: i32,
        }
        impl Read for Ending {
            fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
                let count = self.prefix.read(output)?;
                if count == 0 {
                    Err(io::Error::from_raw_os_error(self.error))
                } else {
                    Ok(count)
                }
            }
        }
        for error in [rustix::io::Errno::SRCH, rustix::io::Errno::NOENT] {
            assert_eq!(
                read_contents(
                    Ending {
                        prefix: Cursor::new(b"partial task identity".to_vec()),
                        error: error.raw_os_error(),
                    },
                    8192
                )
                .expect("disappearing observation"),
                None,
            );
        }
        assert!(
            read_contents(
                Ending {
                    prefix: Cursor::new(b"partial task identity".to_vec()),
                    error: rustix::io::Errno::ACCESS.raw_os_error(),
                },
                8192
            )
            .is_err()
        );
        assert_eq!(
            read_contents(Cursor::new(b"1234"), 4).expect("exact bound"),
            Some(b"1234".to_vec())
        );
        assert!(read_contents(Cursor::new(b"12345"), 4).is_err());
    }
}
