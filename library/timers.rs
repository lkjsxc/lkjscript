//! Indexed deadlines: one heap entry per live slot, updated in place.
//! Repeated activity never accumulates stale timers.
use crate::Option;

#[derive(Clone, Copy)]
struct Deadline {
    slot: usize,
    at: usize,
}
pub struct DeadlineQueue {
    heap: Vec<Deadline>,
    positions: Vec<usize>,
}
impl DeadlineQueue {
    pub fn new() -> DeadlineQueue {
        DeadlineQueue { heap: Vec::new(), positions: Vec::new() }
    }
    pub fn len(&self) -> usize { self.heap.len() }
    pub fn is_empty(&self) -> bool { self.heap.is_empty() }
    pub fn next_deadline(&self) -> Option<usize> {
        if self.heap.is_empty() { Option::None }
        else { Option::Some(self.heap[0usize].at) }
    }
    fn before(&self, left: usize, right: usize) -> bool {
        let first = self.heap[left];
        let second = self.heap[right];
        first.at < second.at || (first.at == second.at && first.slot < second.slot)
    }
    fn exchange(&mut self, left: usize, right: usize) {
        let first = self.heap[left];
        let second = self.heap[right];
        self.heap[left] = second;
        self.heap[right] = first;
        self.positions[first.slot] = right;
        self.positions[second.slot] = left;
    }
    fn up(&mut self, mut position: usize) {
        while position > 0usize {
            let parent = (position - 1usize) / 2usize;
            if !self.before(position, parent) { break; }
            self.exchange(position, parent);
            position = parent;
        }
    }
    fn down(&mut self, mut position: usize) {
        loop {
            let left = position * 2usize + 1usize;
            if left >= self.heap.len() { break; }
            let right = left + 1usize;
            let child = if right < self.heap.len() && self.before(right, left) { right } else { left };
            if !self.before(child, position) { break; }
            self.exchange(position, child);
            position = child;
        }
    }
    pub fn schedule(&mut self, slot: usize, at: usize) {
        while self.positions.len() <= slot { self.positions.push(!0usize); }
        let position = self.positions[slot];
        if position == !0usize {
            let added = self.heap.len();
            self.heap.push(Deadline { slot, at });
            self.positions[slot] = added;
            self.up(added);
        } else {
            let previous = self.heap[position].at;
            self.heap[position].at = at;
            if at < previous { self.up(position); }
            else if at > previous { self.down(position); }
        }
    }
    pub fn cancel(&mut self, slot: usize) -> bool {
        if slot >= self.positions.len() { return false; }
        let position = self.positions[slot];
        if position == !0usize { return false; }
        self.positions[slot] = !0usize;
        let last = self.heap.pop().unwrap();
        if position < self.heap.len() {
            self.heap[position] = last;
            self.positions[last.slot] = position;
            if position > 0usize && self.before(position, (position - 1usize) / 2usize) {
                self.up(position);
            } else { self.down(position); }
        }
        true
    }
    pub fn due(&mut self, now: usize) -> Option<usize> {
        if self.heap.is_empty() || self.heap[0usize].at > now { return Option::None; }
        let slot = self.heap[0usize].slot;
        self.cancel(slot);
        Option::Some(slot)
    }
}
#[test]
fn updates_replace_instead_of_accumulating() {
    let mut timers = DeadlineQueue::new();
    for deadline in 0usize..1000usize { timers.schedule(7usize, deadline); }
    assert_eq!(timers.len(), 1usize);
    assert_eq!(timers.next_deadline(), Option::Some(999usize));
    assert_eq!(timers.due(998usize), Option::None);
    assert_eq!(timers.due(999usize), Option::Some(7usize));
    assert!(timers.is_empty());
}
#[test]
fn order_changes_and_slot_reuse() {
    let mut timers = DeadlineQueue::new();
    for slot in 0usize..64usize { timers.schedule(slot, 1000usize + slot); }
    for slot in 0usize..64usize {
        if slot % 2usize == 0usize { assert!(timers.cancel(slot)); }
        else { timers.schedule(slot, 64usize - slot); }
    }
    assert_eq!(timers.len(), 32usize);
    for expected in 0usize..32usize {
        assert_eq!(timers.due(1000usize), Option::Some(63usize - expected * 2usize));
    }
    assert!(timers.is_empty());
    timers.schedule(7usize, 10usize);
    assert!(timers.cancel(7usize));
    timers.schedule(7usize, 20usize);
    assert_eq!(timers.due(19usize), Option::None);
    assert_eq!(timers.due(20usize), Option::Some(7usize));
    assert!(!timers.cancel(7usize));
}
