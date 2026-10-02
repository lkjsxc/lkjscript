//! Synthetic procfs and injected signals: these tests never spawn or signal a process.
use super::*;
use std::cell::RefCell;
use std::collections::BTreeSet;

struct FakeProc {
    directory: tempfile::TempDir,
    processes: RefCell<BTreeMap<u32, Observation>>,
    replacements: RefCell<BTreeMap<u32, (usize, Observation)>>,
    observations: RefCell<BTreeMap<u32, usize>>,
    attempted_observations: RefCell<Vec<u32>>,
    signals: RefCell<Vec<(Identity, Signal)>>,
    failed_observations: RefCell<BTreeSet<u32>>,
}

impl FakeProc {
    fn new() -> Self {
        Self {
            directory: tempfile::tempdir().unwrap(),
            processes: RefCell::new(BTreeMap::new()),
            replacements: RefCell::new(BTreeMap::new()),
            observations: RefCell::new(BTreeMap::new()),
            attempted_observations: RefCell::new(Vec::new()),
            signals: RefCell::new(Vec::new()),
            failed_observations: RefCell::new(BTreeSet::new()),
        }
    }

    fn process(&self, pid: u32, parent: u32) -> Identity {
        let identity = Identity {
            pid,
            started: 100 + u64::from(pid),
        };
        self.processes.borrow_mut().insert(
            pid,
            Observation {
                identity,
                live: true,
                parent,
            },
        );
        identity
    }

    fn children(&self, pid: u32, children: &[u32]) {
        let task = self
            .directory
            .path()
            .join(pid.to_string())
            .join("task")
            .join(pid.to_string());
        fs::create_dir_all(&task).unwrap();
        let text = children
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(" ");
        fs::write(task.join("children"), text).unwrap();
    }

    fn tree(&self, root: u32) -> Descendants {
        Descendants {
            root,
            root_identity: Some(self.processes.borrow()[&root].identity),
            known: BTreeMap::new(),
            boundary: None,
            cleanup_incomplete: false,
        }
    }

    fn observe(&self, pid: u32) -> Result<Option<Observation>, DevError> {
        self.attempted_observations.borrow_mut().push(pid);
        if self.failed_observations.borrow_mut().remove(&pid) {
            return Err(DevError::infrastructure(
                "injected proc observation failure",
            ));
        }
        let count = {
            let mut observations = self.observations.borrow_mut();
            let count = observations.entry(pid).or_default();
            *count += 1;
            *count
        };
        let replacement = self.replacements.borrow().get(&pid).copied();
        if let Some((at, replacement)) = replacement
            && at == count
        {
            self.processes.borrow_mut().insert(pid, replacement);
        }
        Ok(self.processes.borrow().get(&pid).copied())
    }

    fn signal(&self, identity: Identity, signal: Signal) -> Result<(), DevError> {
        let mut processes = self.processes.borrow_mut();
        let mut killed = false;
        if let Some(current) = processes.get_mut(&identity.pid)
            && current.identity == identity
            && current.live
        {
            self.signals.borrow_mut().push((identity, signal));
            if signal == Signal::KILL {
                current.live = false;
                killed = true;
            }
        }
        // Model the counterexample: killing an owned parent immediately reparents
        // its live children, including those never admitted by discovery.
        if killed {
            for current in processes.values_mut() {
                if current.live && current.parent == identity.pid {
                    current.parent = 1;
                }
            }
        }
        Ok(())
    }

    fn discover(&self, tree: &mut Descendants, limits: Limits) -> Result<(), DevError> {
        tree.discover_with(
            false,
            self.directory.path(),
            limits,
            |pid| self.observe(pid),
            |identity, signal| self.signal(identity, signal),
        )
    }

    fn terminate(&self, tree: &mut Descendants, limits: Limits) -> Result<(), DevError> {
        tree.terminate_with(
            self.directory.path(),
            limits,
            |pid| self.observe(pid),
            |identity, signal| self.signal(identity, signal),
        )
    }

    fn finish(&self, tree: &mut Descendants, limits: Limits) -> Result<bool, DevError> {
        tree.finish_with(
            Instant::now(),
            self.directory.path(),
            limits,
            |pid| self.observe(pid),
            |identity, signal| self.signal(identity, signal),
        )
    }
}

#[test]
fn proc_stat_parent_and_start_identity_use_independent_field_offsets() {
    let mut fields = vec!["0"; 20];
    fields[0] = "S";
    fields[1] = "10";
    fields[19] = "12345";
    let stat = format!("20 (name ) with spaces) {}", fields.join(" "));
    let current = parse_observation(20, stat.as_bytes()).unwrap();
    assert_eq!(
        current.identity,
        Identity {
            pid: 20,
            started: 12345
        }
    );
    assert_eq!(current.parent, 10);
    assert!(current.live);
    fields[0] = "Z";
    let zombie = format!("20 (name) {}", fields.join(" "));
    assert!(!parse_observation(20, zombie.as_bytes()).unwrap().live);
    fields[1] = "invalid";
    let malformed = format!("20 (name) {}", fields.join(" "));
    assert!(
        parse_observation(20, malformed.as_bytes())
            .unwrap_err()
            .to_string()
            .contains("invalid process stat fields")
    );
}

#[test]
fn new_pid_replaced_after_edge_read_is_neither_admitted_nor_signalled() {
    let proc = FakeProc::new();
    proc.process(10, 1);
    let original = proc.process(20, 10);
    proc.children(10, &[20]);
    let mut tree = proc.tree(10);
    // The edge still names 20, but its first stat observation after reading that
    // edge now belongs to an unrelated parent and a different start identity.
    proc.replacements.borrow_mut().insert(
        20,
        (
            1,
            Observation {
                identity: Identity {
                    started: original.started + 1,
                    ..original
                },
                live: true,
                parent: 999,
            },
        ),
    );
    proc.terminate(&mut tree, LIMITS).unwrap();
    assert!(!tree.known.contains_key(&20));
    assert!(proc.processes.borrow()[&20].live);
    assert!(
        proc.signals
            .borrow()
            .iter()
            .all(|(identity, _)| identity.pid != 20)
    );
}

#[test]
fn changed_parent_identity_cannot_authorize_a_new_child() {
    let proc = FakeProc::new();
    let root = proc.process(10, 1);
    proc.process(20, 10);
    proc.children(10, &[20]);
    let mut tree = proc.tree(10);
    // The numeric PPID still matches, but the parent changed between the root
    // observation and the child's admission check.
    proc.replacements.borrow_mut().insert(
        10,
        (
            2,
            Observation {
                identity: Identity {
                    started: root.started + 1,
                    ..root
                },
                live: true,
                parent: 1,
            },
        ),
    );
    proc.discover(&mut tree, LIMITS).unwrap();
    assert!(tree.known.is_empty());
    assert!(proc.signals.borrow().is_empty());
}

#[test]
fn replacement_after_admission_does_not_inherit_cleanup_authority() {
    let proc = FakeProc::new();
    proc.process(10, 1);
    let original = proc.process(20, 10);
    proc.children(10, &[20]);
    let mut tree = proc.tree(10);
    proc.replacements.borrow_mut().insert(
        20,
        (
            2,
            Observation {
                identity: Identity {
                    started: original.started + 1,
                    ..original
                },
                live: true,
                parent: 999,
            },
        ),
    );
    proc.terminate(&mut tree, LIMITS).unwrap();
    assert_eq!(tree.known.get(&20), Some(&original));
    assert!(proc.processes.borrow()[&20].live);
    assert!(
        proc.signals
            .borrow()
            .iter()
            .all(|(identity, _)| identity.pid != 20)
    );
}

#[test]
fn exact_process_limit_and_repeated_thread_edges_preserve_live_ownership() {
    let limits = Limits {
        processes: 4,
        depth: 3,
    };
    let proc = FakeProc::new();
    proc.process(10, 1);
    for child in 20..=22 {
        proc.process(child, 10);
    }
    proc.children(10, &[20, 21, 22]);
    let worker = proc.directory.path().join("10/task/11");
    fs::create_dir_all(&worker).unwrap();
    fs::write(worker.join("children"), b"20 21 22").unwrap();
    let mut tree = proc.tree(10);
    for _ in 0..2 {
        proc.discover(&mut tree, limits).unwrap();
        assert_eq!(tree.known.keys().copied().collect::<Vec<_>>(), [20, 21, 22]);
        assert!(proc.signals.borrow().is_empty());
    }
    proc.terminate(&mut tree, limits).unwrap();
    for child in 20..=22 {
        assert!(!proc.processes.borrow()[&child].live);
    }
}

#[test]
fn cleanup_reserve_discovers_overflow_grandchildren_before_killing_their_parents() {
    let limits = Limits {
        processes: 4,
        depth: 3,
    };
    for sample_first in [false, true] {
        let proc = FakeProc::new();
        let root = proc.process(10, 1);
        for child in 20..=23 {
            proc.process(child, 10);
        }
        // 24 exercises pending sibling work; 25 is specifically a child of the
        // overflow child 23. Identity signals do not depend on process groups.
        proc.process(24, 20);
        proc.process(25, 23);
        proc.children(10, &[20, 21, 22, 23]);
        proc.children(20, &[24]);
        proc.children(23, &[25]);
        let mut tree = proc.tree(10);
        if sample_first {
            let error = proc.discover(&mut tree, limits).unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("owned process inventory exhausted")
            );
            assert!(proc.processes.borrow()[&23].live);
            assert!(proc.processes.borrow()[&24].live);
            assert_eq!(tree.known.len(), limits.processes - 1);
        }
        let result = proc.terminate(&mut tree, limits);
        if sample_first {
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("cleanup incomplete")
            );
        } else {
            result.unwrap();
        }
        assert!(tree.boundary.is_none());
        assert!(tree.known.contains_key(&24));
        assert!(tree.known.contains_key(&25));
        assert!(
            proc.observations.borrow()[&20] >= 2,
            "pending 20 must be drained"
        );
        for child in 20..=25 {
            assert!(!proc.processes.borrow()[&child].live);
        }
        assert_eq!(proc.processes.borrow()[&25].parent, 23);
        assert!(tree.known.len() < limits.processes * CLEANUP_INVENTORY_MULTIPLIER);
        let signals = proc.signals.borrow();
        assert!(signals.contains(&(root, Signal::STOP)));
        let killed: Vec<_> = signals
            .iter()
            .filter(|entry| entry.1 == Signal::KILL)
            .map(|entry| entry.0.pid)
            .collect();
        assert!(
            killed.iter().position(|pid| *pid == 25) < killed.iter().position(|pid| *pid == 23)
        );
        assert!(
            killed.iter().position(|pid| *pid == 24) < killed.iter().position(|pid| *pid == 20)
        );
        assert!(!killed.contains(&10));
        drop(signals);
        assert!(
            !tree
                .has_live_with(|pid| {
                    Ok(proc
                        .observe(pid)?
                        .map(|current| (current.identity, current.live)))
                })
                .unwrap()
        );
        // Owned::finish kills its root/group only after this bounded pass. There
        // are no admitted survivors to reparent, but an earlier failure is retained.
        proc.signal(root, Signal::KILL).unwrap();
        if sample_first {
            assert!(
                proc.finish(&mut tree, limits)
                    .unwrap_err()
                    .to_string()
                    .contains("cannot be certified joined")
            );
        } else {
            assert!(!proc.finish(&mut tree, limits).unwrap());
        }
    }
}

#[test]
fn root_exit_and_successive_retained_chain_growth_do_not_reset_depth() {
    for depth in [3, MAXIMUM_DEPTH] {
        let limits = Limits {
            processes: MAXIMUM_PROCESSES,
            depth,
        };
        let proc = FakeProc::new();
        proc.process(10, 1);
        proc.process(20, 10);
        proc.children(10, &[20]);
        let mut tree = proc.tree(10);
        proc.discover(&mut tree, limits).unwrap();
        proc.processes.borrow_mut().remove(&10);
        proc.processes.borrow_mut().get_mut(&20).unwrap().parent = 1;
        let mut tail = 20;
        for _ in 0..depth {
            let child = tail + 1;
            proc.process(child, tail);
            proc.children(tail, &[child]);
            proc.discover(&mut tree, limits).unwrap();
            assert!(tree.known.contains_key(&child));
            tail = child;
        }
        let excess = tail + 1;
        proc.process(excess, tail);
        proc.children(tail, &[excess]);
        let grandchild = excess + 1;
        proc.process(grandchild, excess);
        proc.children(excess, &[grandchild]);
        let error = proc.discover(&mut tree, limits).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("owned descendant inventory exhausted")
        );
        assert!(
            tree.known.contains_key(&excess),
            "depth failure must retain cleanup authority"
        );
        assert!(proc.terminate(&mut tree, limits).is_err());
        for pid in 20..=grandchild {
            assert!(!proc.processes.borrow()[&pid].live);
        }
        assert_eq!(proc.processes.borrow()[&grandchild].parent, excess);
        assert!(tree.known.contains_key(&grandchild));
        let killed: Vec<_> = proc
            .signals
            .borrow()
            .iter()
            .filter(|entry| entry.1 == Signal::KILL)
            .map(|entry| entry.0.pid)
            .collect();
        assert_eq!(killed, (20..=grandchild).rev().collect::<Vec<_>>());
    }
}

#[test]
fn exhausted_cleanup_reserve_kills_retained_identities_without_claiming_containment() {
    let limits = Limits {
        processes: 4,
        depth: 3,
    };
    let proc = FakeProc::new();
    proc.process(10, 1);
    for pid in 20..=29 {
        proc.process(pid, 10);
    }
    proc.process(30, 27);
    proc.children(10, &(20..=29).collect::<Vec<_>>());
    proc.children(27, &[30]);
    let mut tree = proc.tree(10);
    assert!(
        proc.terminate(&mut tree, limits)
            .unwrap_err()
            .to_string()
            .contains("owned process inventory exhausted")
    );
    assert_eq!(tree.known.len(), 7);
    assert_eq!(tree.boundary.unwrap().pid, 27);
    for pid in 20..=27 {
        assert!(!proc.processes.borrow()[&pid].live);
    }
    for pid in [28, 29, 30] {
        assert!(proc.processes.borrow()[&pid].live);
        assert!(
            proc.signals
                .borrow()
                .iter()
                .all(|(identity, _)| identity.pid != pid)
        );
    }
    assert!(
        proc.finish(&mut tree, limits)
            .unwrap_err()
            .to_string()
            .contains("cannot be certified joined")
    );
}

#[test]
fn finish_with_prior_failure_still_terminates_and_observes_owned_children() {
    let proc = FakeProc::new();
    proc.process(10, 1);
    let owned = proc.process(20, 1);
    proc.process(21, 20);
    proc.children(20, &[21]);
    let mut tree = proc.tree(10);
    tree.known.insert(20, owned);
    tree.cleanup_incomplete = true;
    assert!(
        proc.finish(&mut tree, LIMITS)
            .unwrap_err()
            .to_string()
            .contains("cannot be certified joined")
    );
    assert!(!proc.processes.borrow()[&20].live);
    assert!(!proc.processes.borrow()[&21].live);
    assert_eq!(proc.processes.borrow()[&21].parent, 20);
}

#[test]
fn observation_failure_does_not_abandon_other_owned_branches() {
    let proc = FakeProc::new();
    proc.process(10, 1);
    let unreadable = proc.process(20, 1);
    let healthy = proc.process(30, 1);
    proc.process(31, 30);
    proc.children(30, &[31]);
    let mut tree = proc.tree(10);
    tree.known.insert(20, unreadable);
    tree.known.insert(30, healthy);
    // Fail refresh once: healthy pending work is still observed, but that is
    // insufficient evidence to certify that every unread branch joined.
    proc.failed_observations.borrow_mut().insert(20);
    let error = proc.terminate(&mut tree, LIMITS).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("injected proc observation failure")
    );
    assert!(tree.known.contains_key(&20));
    assert!(tree.known.contains_key(&31));
    for pid in [20, 30, 31] {
        assert!(!proc.processes.borrow()[&pid].live);
    }
    assert!(tree.cleanup_incomplete);
    assert!(proc.finish(&mut tree, LIMITS).is_err());
}

#[test]
fn cleanup_read_failure_still_terminates_known_identities_and_reports_the_gap() {
    let proc = FakeProc::new();
    proc.process(10, 1);
    let unreadable = proc.process(20, 10);
    proc.process(21, 20);
    proc.process(30, 10);
    proc.process(31, 30);
    proc.children(10, &[20, 30]);
    proc.children(20, &[21]);
    proc.children(30, &[31]);
    fs::write(proc.directory.path().join("20/task/20/children"), [0xff]).unwrap();
    let mut tree = proc.tree(10);
    let error = proc.terminate(&mut tree, LIMITS).unwrap_err();
    assert!(error.to_string().contains("non-UTF-8 process children"));
    assert_eq!(tree.known.get(&20), Some(&unreadable));
    assert!(tree.known.contains_key(&31));
    assert!(proc.observations.borrow()[&30] >= 2);
    assert!(!proc.observations.borrow().contains_key(&21));
    for pid in [20, 30, 31] {
        assert!(!proc.processes.borrow()[&pid].live);
    }
    // No trustworthy edge admitted 21. The failure is honest about that gap,
    // while never exempting proven owned identities from cleanup.
    assert_eq!(proc.processes.borrow()[&21].parent, 1);
    assert!(proc.processes.borrow()[&21].live);
    assert!(!tree.known.contains_key(&21));
    assert!(
        proc.signals
            .borrow()
            .iter()
            .all(|(identity, _)| identity.pid != 21)
    );
    assert!(
        proc.finish(&mut tree, LIMITS)
            .unwrap_err()
            .to_string()
            .contains("cannot be certified joined")
    );
}

#[test]
fn boundary_exit_observation_keeps_exact_identity_and_continues_after_errors() {
    let proc = FakeProc::new();
    proc.process(10, 1);
    let owned = proc.process(20, 10);
    let boundary = proc.process(23, 10);
    let mut tree = proc.tree(10);
    tree.known.insert(20, owned);
    tree.boundary = Some(boundary);
    tree.cleanup_incomplete = true;
    proc.failed_observations.borrow_mut().insert(20);
    let before = proc.attempted_observations.borrow().len();
    assert!(
        tree.has_live_with(|pid| {
            Ok(proc
                .observe(pid)?
                .map(|current| (current.identity, current.live)))
        })
        .is_err()
    );
    assert_eq!(&proc.attempted_observations.borrow()[before..], &[20, 23]);
    assert_eq!(tree.boundary, Some(boundary));
    proc.processes.borrow_mut().get_mut(&20).unwrap().live = false;
    // Recycled numeric PID has no authority to stand in for the boundary.
    proc.processes.borrow_mut().insert(
        23,
        Observation {
            identity: Identity {
                started: boundary.started + 1,
                ..boundary
            },
            live: true,
            parent: 999,
        },
    );
    assert!(
        !tree
            .has_live_with(|pid| {
                Ok(proc
                    .observe(pid)?
                    .map(|current| (current.identity, current.live)))
            })
            .unwrap()
    );
    tree.refresh_known(|pid| {
        Ok(proc
            .observe(pid)?
            .map(|current| (current.identity, current.live)))
    })
    .unwrap();
    assert!(tree.boundary.is_none());
    assert!(tree.known.is_empty());
    assert!(
        proc.finish(&mut tree, LIMITS)
            .unwrap_err()
            .to_string()
            .contains("cannot be certified joined")
    );
    assert!(proc.signals.borrow().is_empty());
}
