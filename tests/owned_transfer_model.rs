//! Finite design model, not an implementation of task transfer or a language API.
//! Independent obligations: one custodian, bounded reservations, irrevocable
//! acceptance, no duplicate delivery, and a reachable joined-cleanup state.
use std::collections::{BTreeMap, BTreeSet, VecDeque};

const MESSAGES: usize = 2;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Phase {
    Ready,
    Reserved,
    Pending,
    Accepted,
    Refused,
    Cancelled,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct State {
    open: bool,
    phase: [Phase; MESSAGES],
    sender: [bool; MESSAGES],
    accepted: [bool; MESSAGES],
    queue: Vec<usize>,
    receiver: Option<usize>,
    delivered: [u8; MESSAGES],
    disposed: [u8; MESSAGES],
    completed: [bool; MESSAGES],
}

impl State {
    fn initial() -> Self {
        Self {
            open: true,
            phase: [Phase::Ready; MESSAGES],
            sender: [true; MESSAGES],
            accepted: [false; MESSAGES],
            queue: Vec::new(),
            receiver: None,
            delivered: [0; MESSAGES],
            disposed: [0; MESSAGES],
            completed: [false; MESSAGES],
        }
    }

    fn occupancy(&self) -> usize {
        self.queue.len() + self.phase.iter().filter(|p| **p == Phase::Reserved).count()
    }

    fn valid(&self, capacity: usize) -> bool {
        self.occupancy() <= capacity
            && (0..MESSAGES).all(|id| {
                let queued = self.queue.iter().filter(|entry| **entry == id).count();
                let received = usize::from(self.receiver == Some(id));
                usize::from(self.sender[id]) + queued + received + usize::from(self.disposed[id])
                    == 1
                    && self.disposed[id] <= 1
                    && self.delivered[id] <= 1
                    && (self.delivered[id] == 0 || self.accepted[id])
                    && (queued == 0 || (self.accepted[id] && self.delivered[id] == 0))
                    && (received == 0 || self.delivered[id] == 1)
                    && (!self.completed[id] || (self.delivered[id] == 1 && self.disposed[id] == 1))
                    && match self.phase[id] {
                        Phase::Ready | Phase::Reserved => self.sender[id] && !self.accepted[id],
                        Phase::Refused => !self.accepted[id] && queued + received == 0,
                        Phase::Pending | Phase::Accepted => self.accepted[id] && !self.sender[id],
                        Phase::Cancelled => {
                            !self.sender[id] && (self.accepted[id] || self.disposed[id] == 1)
                        }
                    }
            })
    }

    fn joined(&self) -> bool {
        !self.open
            && self.disposed == [1; MESSAGES]
            && self
                .phase
                .iter()
                .all(|p| matches!(p, Phase::Accepted | Phase::Refused | Phase::Cancelled))
    }

    fn successors(&self, capacity: usize, return_after_acceptance: bool) -> Vec<Self> {
        let mut next = Vec::new();
        let mut push = |edit: &dyn Fn(&mut Self)| {
            let mut state = self.clone();
            edit(&mut state);
            next.push(state);
        };
        if self.open {
            push(&|s| s.open = false);
        }
        for id in 0..MESSAGES {
            match self.phase[id] {
                Phase::Ready => {
                    if self.open && self.occupancy() < capacity {
                        push(&|s| s.phase[id] = Phase::Reserved);
                    } else {
                        push(&|s| s.phase[id] = Phase::Refused);
                    }
                }
                Phase::Reserved => {
                    if self.open {
                        push(&|s| {
                            s.sender[id] = false;
                            s.queue.push(id);
                            s.accepted[id] = true;
                            s.phase[id] = Phase::Pending;
                        });
                    } else {
                        push(&|s| s.phase[id] = Phase::Refused);
                    }
                }
                Phase::Pending => push(&|s| s.phase[id] = Phase::Accepted),
                Phase::Refused if self.sender[id] => {
                    // Only a returned, never-accepted owner can be explicitly retried.
                    push(&|s| s.phase[id] = Phase::Ready);
                    push(&|s| {
                        s.sender[id] = false;
                        s.disposed[id] += 1;
                    });
                }
                _ => {}
            }
            if matches!(
                self.phase[id],
                Phase::Ready | Phase::Reserved | Phase::Pending
            ) {
                push(&|s| {
                    if s.sender[id] {
                        s.sender[id] = false;
                        s.disposed[id] += 1;
                    } else if return_after_acceptance {
                        // Deliberately wrong alternative: a lost receipt restores ownership.
                        s.sender[id] = true;
                    }
                    s.phase[id] = Phase::Cancelled;
                });
            }
        }
        if !self.queue.is_empty() {
            if !self.open {
                push(&|s| {
                    let id = s.queue.remove(0);
                    s.disposed[id] += 1;
                });
            } else if self.receiver.is_none() {
                push(&|s| {
                    let id = s.queue.remove(0);
                    s.delivered[id] += 1;
                    s.receiver = Some(id);
                });
            }
        }
        if let Some(id) = self.receiver {
            for success in [false, true] {
                push(&|s| {
                    s.receiver = None;
                    s.disposed[id] += 1;
                    s.completed[id] = success;
                });
            }
        }
        next
    }
}

#[test]
fn owned_transfer_model_conserves_custody_and_has_joined_cleanup_from_every_state() {
    for capacity in [1, 2] {
        let initial = State::initial();
        let mut indices = BTreeMap::from([(initial.clone(), 0)]);
        let mut states = vec![initial];
        let mut predecessors = vec![Vec::new()];
        let mut cursor = 0;
        while cursor < states.len() {
            assert!(states[cursor].valid(capacity), "{:?}", states[cursor]);
            for next in states[cursor].successors(capacity, false) {
                assert!(next.valid(capacity), "{:?} -> {next:?}", states[cursor]);
                for id in 0..MESSAGES {
                    assert!(!states[cursor].accepted[id] || next.accepted[id]);
                }
                let index = if let Some(index) = indices.get(&next) {
                    *index
                } else {
                    let index = states.len();
                    indices.insert(next.clone(), index);
                    states.push(next);
                    predecessors.push(Vec::new());
                    index
                };
                predecessors[index].push(cursor);
            }
            cursor += 1;
        }
        let terminal = states
            .iter()
            .enumerate()
            .filter_map(|(i, state)| state.joined().then_some(i))
            .collect::<Vec<_>>();
        let mut pending: VecDeque<_> = terminal.iter().copied().collect();
        let mut can_join: BTreeSet<_> = terminal.iter().copied().collect();
        while let Some(index) = pending.pop_front() {
            for parent in &predecessors[index] {
                if can_join.insert(*parent) {
                    pending.push_back(*parent);
                }
            }
        }
        assert_eq!(can_join.len(), states.len(), "cleanup dead end");
        // These cases distinguish acceptance, observation, processing and cleanup.
        assert!(states.iter().any(|s| s.phase.contains(&Phase::Refused)));
        assert!(states.iter().any(|s| {
            (0..MESSAGES).any(|id| s.phase[id] == Phase::Cancelled && s.queue.contains(&id))
        }));
        assert!(
            states
                .iter()
                .any(|s| (0..MESSAGES).any(|id| s.phase[id] == Phase::Pending && s.completed[id]))
        );
        assert!(states.iter().any(|s| {
            (0..MESSAGES).any(|id| s.accepted[id] && s.disposed[id] == 1 && s.delivered[id] == 0)
        }));
        println!(
            "capacity={capacity} states={} joined={}",
            states.len(),
            terminal.len()
        );
    }
}

#[test]
fn owned_transfer_model_rejects_restoring_a_sender_after_acceptance() -> Result<(), &'static str> {
    let initial = State::initial();
    let reserved = initial
        .successors(1, false)
        .into_iter()
        .find(|s| s.phase[0] == Phase::Reserved)
        .ok_or("the first sender must be able to reserve an empty queue")?;
    let accepted = reserved
        .successors(1, false)
        .into_iter()
        .find(|s| s.phase[0] == Phase::Pending)
        .ok_or("an open reservation must be able to commit before observation")?;
    assert!(accepted.valid(1));
    let wrong = accepted
        .successors(1, true)
        .into_iter()
        .find(|s| s.phase[0] == Phase::Cancelled)
        .ok_or("sender cancellation must remain possible after acceptance")?;
    assert!(!wrong.valid(1));
    assert!(wrong.sender[0] && wrong.queue.contains(&0));
    Ok(())
}
