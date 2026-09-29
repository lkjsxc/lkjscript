use super::*;
use std::process::{Child, Command, Stdio};

#[test]
fn failed_observations_retain_cleanup_ownership_and_do_not_hide_errors() {
    let mut tree = Descendants::new(0);
    for pid in 1..=5 {
        tree.known.insert(pid, Identity { pid, started: 10 });
    }
    let mut observations = Vec::new();
    let error = tree
        .refresh_known(|pid| {
            observations.push(pid);
            match pid {
                1 => Err(DevError::infrastructure("injected proc read failure")),
                2 => Ok(None),
                3 => Ok(Some((Identity { pid, started: 10 }, false))),
                4 => Ok(Some((Identity { pid, started: 11 }, true))),
                5 => Ok(Some((Identity { pid, started: 10 }, true))),
                _ => panic!("unowned observation"),
            }
        })
        .expect_err("observation failure is not clean completion");
    assert!(error.to_string().contains("injected proc read failure"));
    assert_eq!(observations, [1, 2, 3, 4, 5]);
    assert_eq!(tree.known.keys().copied().collect::<Vec<_>>(), [1, 5]);
    tree.refresh_known(|pid| Ok(Some((Identity { pid, started: 10 }, true))))
        .expect("healthy observations recover without forgetting live branches");
    assert_eq!(tree.known.len(), 2);
}

#[test]
fn serial_departures_do_not_accumulate_a_lifetime_quota() {
    let mut tree = Descendants::new(0);
    for started in 0..u64::try_from(MAXIMUM_PROCESSES * 2).unwrap() {
        tree.known.insert(7, Identity { pid: 7, started });
        tree.refresh_known(|pid| {
            assert_eq!(pid, 7);
            Ok(Some((Identity { pid, started }, false)))
        })
        .unwrap();
        assert!(tree.known.is_empty());
    }
}

struct ChildGuard(Child);

impl ChildGuard {
    fn sleeping() -> Self {
        Self(
            Command::new("/bin/sleep")
                .arg("60")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("owned finite fixture"),
        )
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn sample_retires_departed_inventory_before_admitting_work() {
    let child = ChildGuard::sleeping();
    let mut tree = Descendants::new(child.0.id());
    // Reproduce the retained state of a long serial workload without forking
    // thousands of concurrent processes. Every seeded identity is absent.
    for offset in 0..MAXIMUM_PROCESSES {
        let pid = u32::MAX - u32::try_from(offset).unwrap();
        assert!(observe(pid).unwrap().is_none());
        tree.known.insert(pid, Identity { pid, started: 1 });
    }
    tree.sample()
        .expect("departed history is not live capacity");
    assert!(tree.known.is_empty());
}

#[test]
fn sample_does_not_adopt_recycled_unrelated_seed() {
    let root = ChildGuard::sleeping();
    let unrelated = ChildGuard::sleeping();
    let (identity, live) = observe(unrelated.0.id()).unwrap().unwrap();
    assert!(live);
    let stale = Identity {
        started: identity.started.wrapping_add(1),
        ..identity
    };
    let mut tree = Descendants::new(root.0.id());
    tree.known.insert(stale.pid, stale);
    tree.sample()
        .expect("recycled identity is retired, not adopted");
    assert!(!tree.known.contains_key(&stale.pid));
    assert_eq!(observe(identity.pid).unwrap(), Some((identity, true)));
}

#[test]
fn sample_retires_reaped_child_but_retains_live_owned_branch() {
    let root = ChildGuard::sleeping();
    let mut departed = ChildGuard::sleeping();
    let live = ChildGuard::sleeping();
    let departed_identity = observe(departed.0.id()).unwrap().unwrap().0;
    let live_identity = observe(live.0.id()).unwrap().unwrap().0;
    let mut tree = Descendants::new(root.0.id());
    tree.known.insert(departed_identity.pid, departed_identity);
    tree.known.insert(live_identity.pid, live_identity);
    departed.0.kill().unwrap();
    departed.0.wait().unwrap();
    tree.sample().expect("refresh actual process identities");
    assert_eq!(
        tree.known.values().copied().collect::<Vec<_>>(),
        [live_identity]
    );
}
