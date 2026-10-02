//! Bounded cleanup of owned verifier descendants, including children with separate process groups.
#[cfg(test)]
mod inventory_tests;
#[cfg(test)]
mod provenance_tests;
mod traversal;

use crate::error::DevError;
use rustix::process::{Pid, PidfdFlags, Signal, pidfd_open, pidfd_send_signal};
use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::io::Read;
use std::path::Path;
use std::time::{Duration, Instant};
use traversal::Traversal;

const MAXIMUM_PROCESSES: usize = 4096;
const MAXIMUM_DEPTH: usize = 64;
// Cleanup has a separate finite reserve so a sampling boundary is not itself a
// reason to abandon an owned branch. Exhausting this reserve remains a failure.
const CLEANUP_INVENTORY_MULTIPLIER: usize = 2;

#[derive(Clone, Copy)]
struct Limits {
    processes: usize,
    depth: usize,
}

const LIMITS: Limits = Limits {
    processes: MAXIMUM_PROCESSES,
    depth: MAXIMUM_DEPTH,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Identity {
    pid: u32,
    started: u64,
}

#[derive(Clone, Copy, Debug)]
struct Observation {
    identity: Identity,
    live: bool,
    parent: u32,
}

pub(super) struct Descendants {
    root: u32,
    root_identity: Option<Identity>,
    known: BTreeMap<u32, Identity>,
    // One reserved boundary slot, separate from the admitted traversal capacity.
    // It retains the first proven overflow identity for cleanup and observation.
    boundary: Option<Identity>,
    cleanup_incomplete: bool,
}

impl Descendants {
    pub(super) fn new(root: u32) -> Self {
        Self {
            root,
            root_identity: observe(root).ok().flatten().map(|(identity, _)| identity),
            known: BTreeMap::new(),
            boundary: None,
            cleanup_incomplete: false,
        }
    }
    pub(super) fn sample(&mut self) -> Result<(), DevError> {
        if self.root_identity.is_none() {
            return Err(DevError::infrastructure(
                "owned root identity is unavailable",
            ));
        }
        self.discover(false)
    }
    fn refresh_known(
        &mut self,
        mut observation: impl FnMut(u32) -> Result<Option<(Identity, bool)>, DevError>,
    ) -> Result<(), DevError> {
        let mut failure = None;
        // Capacity describes currently owned live identities, not all historical children.
        // A recycled PID is not authority to adopt its replacement; only a child edge
        // validated against its owned parent may do that. Failed observations retain
        // ownership for cleanup.
        self.known.retain(|pid, expected| match observation(*pid) {
            Ok(Some((current, true))) => current == *expected,
            Ok(_) => false,
            Err(error) => {
                failure.get_or_insert(error);
                true
            }
        });
        if let Some(expected) = self.boundary {
            match observation(expected.pid) {
                Ok(Some((current, true))) if current == expected => {}
                Ok(_) => self.boundary = None,
                Err(error) => {
                    failure.get_or_insert(error);
                }
            }
        }
        failure.map_or(Ok(()), Err)
    }
    fn discover(&mut self, stop: bool) -> Result<(), DevError> {
        self.discover_with(stop, Path::new("/proc"), LIMITS, observe_process, signal)
    }
    fn discover_with(
        &mut self,
        stop: bool,
        proc_root: &Path,
        limits: Limits,
        mut observation: impl FnMut(u32) -> Result<Option<Observation>, DevError>,
        mut send_signal: impl FnMut(Identity, Signal) -> Result<(), DevError>,
    ) -> Result<(), DevError> {
        let mut parents = BTreeMap::new();
        let refreshed = self.refresh_known(|pid| {
            let current = observation(pid)?;
            if let Some(current) = current {
                parents.insert(pid, current.parent);
            }
            Ok(current.map(|current| (current.identity, current.live)))
        });
        let mut failure = refreshed.err();
        // Only actual retained branch roots restart at depth zero. A retained child
        // must be reached through its live owned parent, even after the root exits.
        let roots = self.known.keys().copied().filter(|pid| {
            parents
                .get(pid)
                .is_none_or(|parent| !self.known.contains_key(parent))
        });
        let mut traversal = Traversal::new(self.root, roots, limits);
        loop {
            let (pid, depth) = match traversal.pop() {
                Ok(Some(next)) => next,
                Ok(None) => break,
                Err(error) => {
                    // pop removes the rejected item. Keep draining bounded work;
                    // a failure cannot silently discard the remaining frontier.
                    failure.get_or_insert(error);
                    continue;
                }
            };
            let Some(Some(current)) = remember(observation(pid), &mut failure) else {
                continue;
            };
            let identity = current.identity;
            if pid == self.root && self.root_identity != Some(identity) {
                continue;
            }
            if !current.live {
                continue;
            }
            // Pending PIDs were admitted while their owned parent was observed.
            // Neither a fallback root nor a queued numeric PID grants new authority.
            if pid != self.root && self.known.get(&pid) != Some(&identity) {
                continue;
            }
            if stop {
                remember(send_signal(identity, Signal::STOP), &mut failure);
            }
            // A verifier may spawn a runner from a worker thread. Linux records children on
            // their creating task, so visiting only the main task would miss that owned runner.
            let tasks = match fs::read_dir(proc_root.join(pid.to_string()).join("task")) {
                Ok(tasks) => tasks,
                Err(error) if disappeared(&error) => continue,
                Err(error) => {
                    failure.get_or_insert(error.into());
                    continue;
                }
            };
            for (count, task) in tasks.take(MAXIMUM_PROCESSES + 1).enumerate() {
                if count == MAXIMUM_PROCESSES {
                    failure.get_or_insert_with(|| {
                        DevError::infrastructure("owned thread inventory exhausted")
                    });
                    break;
                }
                let task = match task {
                    Ok(task) => task,
                    Err(error) if disappeared(&error) => continue,
                    Err(error) => {
                        failure.get_or_insert(error.into());
                        continue;
                    }
                };
                let Some(Some(children)) =
                    remember(read(&task.path().join("children"), 65536), &mut failure)
                else {
                    continue;
                };
                let Some(children) = remember(
                    std::str::from_utf8(&children)
                        .map_err(|_| DevError::corrupt("non-UTF-8 process children")),
                    &mut failure,
                ) else {
                    continue;
                };
                for child in children.split_whitespace() {
                    let Some(child) = remember(
                        child
                            .parse::<u32>()
                            .map_err(|_| DevError::corrupt("invalid child PID")),
                        &mut failure,
                    ) else {
                        continue;
                    };
                    let Some(Some(child)) = remember(observation(child), &mut failure) else {
                        continue;
                    };
                    if !child.live || child.parent != pid || child.identity.pid == self.root {
                        continue;
                    }
                    // A children-file edge may outlive its PID. Bind the child's stat
                    // identity to the still-owned parent identity before admitting it.
                    let Some(Some(parent)) = remember(observation(pid), &mut failure) else {
                        continue;
                    };
                    if parent.identity != identity || !parent.live {
                        continue;
                    }
                    if self
                        .known
                        .get(&child.identity.pid)
                        .is_some_and(|known| *known != child.identity)
                        || self.boundary.is_some_and(|boundary| {
                            boundary.pid == child.identity.pid && boundary != child.identity
                        })
                    {
                        continue;
                    }
                    if !self.known.contains_key(&child.identity.pid) {
                        // The direct root occupies one of the traversal's finite slots.
                        if self.known.len() >= limits.processes.saturating_sub(1) {
                            failure.get_or_insert_with(|| {
                                DevError::infrastructure("owned process inventory exhausted")
                            });
                            if !stop {
                                remember(send_signal(identity, Signal::STOP), &mut failure);
                            }
                            if self.boundary.is_none() {
                                self.boundary = Some(child.identity);
                            }
                            // Defer killing: it could reparent children whose
                            // identities were never read. Further excess children
                            // have no individual exit slot; their parent is owned.
                            if self.boundary == Some(child.identity) {
                                remember(send_signal(child.identity, Signal::STOP), &mut failure);
                            }
                            continue;
                        }
                        self.known.insert(child.identity.pid, child.identity);
                        if self.boundary == Some(child.identity) {
                            self.boundary = None;
                        }
                    }
                    // Admission precedes enqueueing. A full/deep frontier cannot erase
                    // cleanup authority, and its already queued work is still drained.
                    let queued = traversal.push(child.identity.pid, depth + 1);
                    if queued.is_err() && stop {
                        remember(send_signal(child.identity, Signal::STOP), &mut failure);
                    }
                    remember(queued, &mut failure);
                }
            }
        }
        if failure.is_some() {
            // A later empty/live inventory cannot prove that an unread branch
            // joined. This remains sticky even if its boundary parent exits.
            self.cleanup_incomplete = true;
        }
        failure.map_or(Ok(()), Err)
    }
    pub(super) fn terminate(&mut self) -> Result<(), DevError> {
        self.terminate_with(Path::new("/proc"), LIMITS, observe_process, signal)
    }
    fn terminate_with(
        &mut self,
        proc_root: &Path,
        limits: Limits,
        mut observation: impl FnMut(u32) -> Result<Option<Observation>, DevError>,
        mut send_signal: impl FnMut(Identity, Signal) -> Result<(), DevError>,
    ) -> Result<(), DevError> {
        // Complete a bounded extra discovery pass before Owned::finish kills the
        // direct root/group. Retain the boundary even if it has reparented.
        let cleanup_limits = Limits {
            processes: limits
                .processes
                .saturating_mul(CLEANUP_INVENTORY_MULTIPLIER),
            depth: limits.depth.saturating_mul(CLEANUP_INVENTORY_MULTIPLIER),
        };
        if let Some(boundary) = self.boundary.take() {
            self.known.entry(boundary.pid).or_insert(boundary);
        }
        let discovered = self.discover_with(
            true,
            proc_root,
            cleanup_limits,
            &mut observation,
            &mut send_signal,
        );
        let mut failure = discovered.err();
        // An observation error cannot suppress termination of all the already
        // owned identities. Build a finite postorder from their observed parents;
        // a missing parent observation still retains individual signal authority.
        let mut pending = self.known.clone();
        if let Some(boundary) = self.boundary {
            pending.entry(boundary.pid).or_insert(boundary);
        }
        let mut parents = BTreeMap::new();
        let mut child_counts = BTreeMap::<u32, usize>::new();
        for (&pid, identity) in &pending {
            if let Some(Some(current)) = remember(observation(pid), &mut failure)
                && current.identity == *identity
                && current.live
                && pending.contains_key(&current.parent)
                && current.parent != pid
            {
                parents.insert(pid, current.parent);
                *child_counts.entry(current.parent).or_default() += 1;
            }
        }
        let mut leaves: VecDeque<_> = pending
            .keys()
            .copied()
            .filter(|pid| !child_counts.contains_key(pid))
            .collect();
        while let Some(pid) = leaves.pop_front() {
            if let Some(identity) = pending.remove(&pid) {
                remember(send_signal(identity, Signal::KILL), &mut failure);
            }
            if let Some(parent) = parents.get(&pid)
                && let Some(count) = child_counts.get_mut(parent)
            {
                *count -= 1;
                if *count == 0 {
                    leaves.push_back(*parent);
                }
            }
        }
        // Inconsistent proc observations cannot turn into an unbounded loop or
        // exempt a retained identity from best-effort termination.
        for identity in pending.values() {
            remember(send_signal(*identity, Signal::KILL), &mut failure);
        }
        if self.cleanup_incomplete {
            failure.get_or_insert_with(|| DevError::infrastructure(
                "owned descendant cleanup incomplete; earlier unread branches cannot be certified joined"
            ));
        }
        if let Some(error) = failure {
            Err(error)
        } else {
            Ok(())
        }
    }
    pub(super) fn has_live(&self) -> Result<bool, DevError> {
        self.has_live_with(observe)
    }
    fn has_live_with(
        &self,
        mut observation: impl FnMut(u32) -> Result<Option<(Identity, bool)>, DevError>,
    ) -> Result<bool, DevError> {
        let mut survivors = false;
        let mut failure = None;
        for identity in self.known.values().chain(self.boundary.iter()) {
            if let Some(current) = remember(observation(identity.pid), &mut failure) {
                survivors |= current.is_some_and(|(current, live)| current == *identity && live);
            }
        }
        failure.map_or(Ok(survivors), Err)
    }
    pub(super) fn finish(&mut self, deadline: Instant) -> Result<bool, DevError> {
        self.finish_with(
            deadline,
            Path::new("/proc"),
            LIMITS,
            observe_process,
            signal,
        )
    }
    fn finish_with(
        &mut self,
        deadline: Instant,
        proc_root: &Path,
        limits: Limits,
        mut observation: impl FnMut(u32) -> Result<Option<Observation>, DevError>,
        mut send_signal: impl FnMut(Identity, Signal) -> Result<(), DevError>,
    ) -> Result<bool, DevError> {
        // Observe every retained identity, including the reserve, even when an
        // unread subtree prevents certification. No numeric replacement joins.
        let mut failure = None;
        let live = remember(
            self.has_live_with(|pid| {
                Ok(observation(pid)?.map(|current| (current.identity, current.live)))
            }),
            &mut failure,
        )
        .unwrap_or(true);
        if live {
            remember(
                self.terminate_with(proc_root, limits, &mut observation, &mut send_signal),
                &mut failure,
            );
        }
        loop {
            let remaining = remember(
                self.has_live_with(|pid| {
                    Ok(observation(pid)?.map(|current| (current.identity, current.live)))
                }),
                &mut failure,
            )
            .unwrap_or(true);
            if !remaining {
                if self.cleanup_incomplete {
                    failure.get_or_insert_with(|| DevError::infrastructure(
                        "owned descendant cleanup incomplete; unread branches cannot be certified joined"
                    ));
                }
                return failure.map_or(Ok(live), Err);
            }
            if Instant::now() >= deadline {
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

fn remember<T>(result: Result<T, DevError>, failure: &mut Option<DevError>) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(error) => {
            failure.get_or_insert(error);
            None
        }
    }
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
    Ok(observe_process(pid)?.map(|current| (current.identity, current.live)))
}
fn observe_process(pid: u32) -> Result<Option<Observation>, DevError> {
    let Some(bytes) = read(Path::new(&format!("/proc/{pid}/stat")), 8192)? else {
        return Ok(None);
    };
    parse_observation(pid, &bytes).map(Some)
}
fn parse_observation(pid: u32, bytes: &[u8]) -> Result<Observation, DevError> {
    let text = std::str::from_utf8(bytes).map_err(|_| DevError::corrupt("invalid process stat"))?;
    let tail = text
        .rsplit_once(") ")
        .ok_or_else(|| DevError::corrupt("invalid process stat fields"))?
        .1;
    let mut fields = tail.split_whitespace();
    let state = fields
        .next()
        .ok_or_else(|| DevError::corrupt("missing process state"))?;
    let parent = fields
        .next()
        .ok_or_else(|| DevError::corrupt("invalid process stat fields"))?
        .parse::<u32>()
        .map_err(|_| DevError::corrupt("invalid process stat fields"))?;
    let started = fields
        .nth(17)
        .ok_or_else(|| DevError::corrupt("missing process start identity"))?
        .parse::<u64>()
        .map_err(|_| DevError::corrupt("invalid process start identity"))?;
    Ok(Observation {
        identity: Identity { pid, started },
        live: !matches!(state, "Z" | "X"),
        parent,
    })
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

#[cfg(test)]
mod reparenting_tests {
    use super::*;
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};

    #[test]
    fn sampled_reparented_branch_discovers_its_later_children() {
        let root = tempfile::tempdir().unwrap();
        // Every fixture process also has a finite fallback lifetime if an assertion fails.
        let mut child = Command::new("/bin/sh").args(["-c",
            "setsid sh -c 'echo $$ > branch; i=0; while test ! -f phase && test $i -lt 100; do i=$((i+1)); sleep 0.01; done; sleep 0.6 & echo $! > leaf; wait' & sleep 1.5"])
            .current_dir(root.path()).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
            .process_group(0).spawn().unwrap();
        let mut tree = Descendants::new(child.id());
        let began = Instant::now();
        let branch = loop {
            tree.sample().unwrap();
            if let Ok(text) = fs::read_to_string(root.path().join("branch"))
                && let Ok(pid) = text.trim().parse::<u32>()
                && tree.known.contains_key(&pid)
            {
                break pid;
            }
            assert!(began.elapsed() < Duration::from_secs(2));
            std::thread::sleep(Duration::from_millis(5));
        };
        child.kill().unwrap();
        child.wait().unwrap();
        fs::write(root.path().join("phase"), b"create later child").unwrap();
        let leaf = loop {
            tree.sample().unwrap();
            if let Ok(text) = fs::read_to_string(root.path().join("leaf"))
                && let Ok(pid) = text.trim().parse::<u32>()
            {
                break pid;
            }
            assert!(began.elapsed() < Duration::from_secs(2));
            std::thread::sleep(Duration::from_millis(5));
        };
        tree.sample().unwrap();
        let discovered = tree.known.contains_key(&leaf);
        tree.terminate().unwrap();
        tree.finish(Instant::now() + Duration::from_secs(2))
            .unwrap();
        assert!(
            discovered,
            "later child of the known reparented branch was not sampled"
        );
        for pid in [branch, leaf] {
            assert!(observe(pid).unwrap().is_none_or(|(_, live)| !live));
        }
    }
}
