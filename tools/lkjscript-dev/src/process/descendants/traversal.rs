//! Per-sample depth-first work, counting each PID once rather than each discovery path.
use super::{MAXIMUM_DEPTH, MAXIMUM_PROCESSES};
use crate::error::DevError;
use std::collections::BTreeSet;

pub(super) struct Traversal {
    roots: Vec<u32>,
    pending: Vec<(u32, usize)>,
    queued: BTreeSet<u32>,
    visited: BTreeSet<u32>,
}

impl Traversal {
    pub(super) fn new(root: u32, known: impl Iterator<Item = u32>) -> Self {
        let mut roots: Vec<_> = known.collect();
        roots.push(root);
        Self {
            roots,
            pending: Vec::new(),
            queued: BTreeSet::new(),
            visited: BTreeSet::new(),
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
            if self.visited.len() >= MAXIMUM_PROCESSES || depth > MAXIMUM_DEPTH {
                return Err(DevError::infrastructure(
                    "owned descendant inventory exhausted",
                ));
            }
            self.visited.insert(pid);
            return Ok(Some((pid, depth)));
        }
    }

    pub(super) fn push(&mut self, pid: u32, depth: usize) -> Result<(), DevError> {
        if self.visited.contains(&pid) || self.queued.contains(&pid) {
            return Ok(());
        }
        if self
            .visited
            .len()
            .checked_add(self.queued.len())
            .is_none_or(|count| count >= MAXIMUM_PROCESSES)
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
    use super::*;

    #[test]
    fn known_children_and_repeated_edges_use_one_slot_each() {
        let maximum = u32::try_from(MAXIMUM_PROCESSES).unwrap();
        let mut walk = Traversal::new(1, 2..=maximum);
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
        let maximum = u32::try_from(MAXIMUM_DEPTH).unwrap();
        let mut walk = Traversal::new(1, 2..=maximum + 2);
        for depth in 0..=MAXIMUM_DEPTH {
            let pid = u32::try_from(depth).unwrap() + 1;
            assert_eq!(walk.pop().unwrap(), Some((pid, depth)));
            walk.push(pid + 1, depth + 1).unwrap();
        }
        assert!(walk.pop().is_err());
    }

    #[test]
    fn disconnected_known_branch_can_discover_later_children() {
        let mut walk = Traversal::new(1, [2].into_iter());
        assert_eq!(walk.pop().unwrap(), Some((1, 0)));
        assert_eq!(walk.pop().unwrap(), Some((2, 0)));
        walk.push(3, 1).unwrap();
        assert_eq!(walk.pop().unwrap(), Some((3, 1)));
        assert_eq!(walk.pop().unwrap(), None);
    }

    #[test]
    fn fallback_roots_cannot_exceed_the_distinct_process_bound() {
        let maximum = u32::try_from(MAXIMUM_PROCESSES).unwrap();
        let mut walk = Traversal::new(1, 2..=maximum + 1);
        for _ in 0..MAXIMUM_PROCESSES {
            assert!(walk.pop().unwrap().is_some());
        }
        assert!(walk.pop().is_err());
    }
}
