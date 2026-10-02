//! Per-sample depth-first work, counting each PID once rather than each discovery path.
use super::Limits;
use crate::error::DevError;
use std::collections::BTreeSet;

pub(super) struct Traversal {
    roots: Vec<u32>,
    pending: Vec<(u32, usize)>,
    queued: BTreeSet<u32>,
    visited: BTreeSet<u32>,
    limits: Limits,
}

impl Traversal {
    pub(super) fn new(root: u32, known: impl Iterator<Item = u32>, limits: Limits) -> Self {
        let mut roots: Vec<_> = known.collect();
        roots.push(root);
        Self {
            roots,
            pending: Vec::new(),
            queued: BTreeSet::new(),
            visited: BTreeSet::new(),
            limits,
        }
    }

    pub(super) fn pop(&mut self) -> Result<Option<(u32, usize)>, DevError> {
        loop {
            // Exhaust a connected tree before trying retained, possibly reparented roots.
            // Otherwise an already-known child could reset its depth to zero each sample.
            let (pid, depth) = if let Some(next) = self.pending.pop() {
                self.queued.remove(&next.0);
                next
            } else if let Some(pid) = self.roots.pop() {
                (pid, 0)
            } else {
                return Ok(None);
            };
            if self.visited.contains(&pid) {
                continue;
            }
            if self.visited.len() >= self.limits.processes || depth > self.limits.depth {
                return Err(DevError::infrastructure(
                    "owned descendant inventory exhausted",
                ));
            }
            self.visited.insert(pid);
            return Ok(Some((pid, depth)));
        }
    }

    pub(super) fn push(&mut self, pid: u32, depth: usize) -> Result<(), DevError> {
        // Even an already queued/visited PID cannot hide an over-deep edge.
        if depth > self.limits.depth {
            return Err(DevError::infrastructure(
                "owned descendant inventory exhausted",
            ));
        }
        if self.visited.contains(&pid) || self.queued.contains(&pid) {
            return Ok(());
        }
        if self
            .visited
            .len()
            .checked_add(self.queued.len())
            .is_none_or(|count| count >= self.limits.processes)
        {
            return Err(DevError::infrastructure(
                "owned descendant traversal exhausted",
            ));
        }
        self.queued.insert(pid);
        self.pending.push((pid, depth));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::{LIMITS, MAXIMUM_DEPTH, MAXIMUM_PROCESSES};
    use super::*;

    #[test]
    fn known_children_and_repeated_edges_use_one_slot_each() {
        let maximum = u32::try_from(MAXIMUM_PROCESSES).unwrap();
        let mut walk = Traversal::new(1, 2..=maximum, LIMITS);
        assert_eq!(walk.pop().unwrap(), Some((1, 0)));
        for pid in 2..=maximum {
            walk.push(pid, 1).unwrap();
            walk.push(pid, 1).unwrap();
        }
        assert_eq!(walk.pending.len(), MAXIMUM_PROCESSES - 1);
        assert!(walk.push(maximum + 1, 1).is_err());
        let mut seen = BTreeSet::from([1]);
        while let Some((pid, depth)) = walk.pop().unwrap() {
            assert_eq!(depth, 1);
            assert!(seen.insert(pid));
            walk.push(pid, 1).unwrap();
        }
        assert_eq!(seen, (1..=maximum).collect());
        assert!(walk.pending.is_empty());
        assert!(walk.queued.is_empty());
    }

    #[test]
    fn connected_known_chain_keeps_its_depth_and_rejects_one_over() {
        // Retained descendants are reached through their actual branch root.
        let mut walk = Traversal::new(1, std::iter::empty(), LIMITS);
        for depth in 0..=MAXIMUM_DEPTH {
            let pid = u32::try_from(depth).unwrap() + 1;
            assert_eq!(walk.pop().unwrap(), Some((pid, depth)));
            if depth == MAXIMUM_DEPTH {
                assert!(walk.push(pid + 1, depth + 1).is_err());
            } else {
                walk.push(pid + 1, depth + 1).unwrap();
            }
        }
        assert_eq!(walk.pop().unwrap(), None);
    }

    #[test]
    fn repeated_edge_cannot_hide_depth_exhaustion() {
        let mut walk = Traversal::new(1, std::iter::empty(), LIMITS);
        assert_eq!(walk.pop().unwrap(), Some((1, 0)));
        walk.push(2, 1).unwrap();
        assert!(walk.push(2, MAXIMUM_DEPTH + 1).is_err());
        assert_eq!(walk.pop().unwrap(), Some((2, 1)));
        assert!(walk.push(2, MAXIMUM_DEPTH + 1).is_err());
    }

    #[test]
    fn disconnected_known_branch_can_discover_later_children() {
        let mut walk = Traversal::new(1, [2].into_iter(), LIMITS);
        assert_eq!(walk.pop().unwrap(), Some((1, 0)));
        assert_eq!(walk.pop().unwrap(), Some((2, 0)));
        walk.push(3, 1).unwrap();
        assert_eq!(walk.pop().unwrap(), Some((3, 1)));
        assert_eq!(walk.pop().unwrap(), None);
    }

    #[test]
    fn fallback_roots_cannot_exceed_the_distinct_process_bound() {
        let maximum = u32::try_from(MAXIMUM_PROCESSES).unwrap();
        let mut walk = Traversal::new(1, 2..=maximum + 1, LIMITS);
        for _ in 0..MAXIMUM_PROCESSES {
            assert!(walk.pop().unwrap().is_some());
        }
        assert!(walk.pop().is_err());
    }

    #[test]
    fn rejected_pop_consumes_work_and_remaining_roots_can_be_drained() {
        let limits = Limits {
            processes: 1,
            depth: 0,
        };
        let mut walk = Traversal::new(1, [2, 3].into_iter(), limits);
        assert_eq!(walk.pop().unwrap(), Some((1, 0)));
        assert!(walk.pop().is_err());
        assert!(walk.pop().is_err());
        assert_eq!(walk.pop().unwrap(), None);
        assert!(walk.pending.is_empty());
        assert!(walk.roots.is_empty());
        assert!(walk.queued.is_empty());
    }
}
