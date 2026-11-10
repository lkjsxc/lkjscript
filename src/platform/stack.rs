//! Semantic work uses a virtual stack reservation derived from the host's physical memory.
//!
//! Linux commits stack pages on demand. Reserving address space does not reserve resident
//! memory, guarantee allocation success, or hide an external OOM kill. Keep owned recursive
//! values inside the operation so normal return, errors and unwinding destroy them here.

use std::cell::Cell;
use std::io;
use std::thread;

thread_local! {
    static SEMANTIC_STACK: Cell<bool> = const { Cell::new(false) };
}

fn reservation_from_meminfo(meminfo: &str) -> io::Result<usize> {
    let kib = meminfo
        .lines()
        .find_map(|line| {
            let mut fields = line.split_whitespace();
            (fields.next()? == "MemTotal:")
                .then(|| fields.next()?.parse::<usize>().ok())
                .flatten()
        })
        .ok_or_else(|| io::Error::other("physical memory is absent from /proc/meminfo"))?;
    kib.checked_mul(2 * 1024)
        .filter(|bytes| *bytes != 0)
        .ok_or_else(|| io::Error::other("physical-memory stack reservation does not fit this host"))
}

/// Twice physical memory, as virtual address space. The operating system admits the mapping.
pub fn reservation_bytes() -> io::Result<usize> {
    reservation_from_meminfo(&std::fs::read_to_string("/proc/meminfo")?)
}

/// Execute an operation on the semantic stack. Nested operations reuse it.
/// Construct and drop recursive values inside the operation. Returned values are later
/// dropped on the caller's stack; failed worker creation also drops captures there.
pub fn run<R: Send>(name: &str, operation: impl FnOnce() -> R + Send) -> io::Result<R> {
    if SEMANTIC_STACK.get() {
        return Ok(operation());
    }
    let bytes = reservation_bytes()?;
    thread::scope(|scope| {
        thread::Builder::new()
            .name(name.to_owned())
            .stack_size(bytes)
            .spawn_scoped(scope, move || {
                SEMANTIC_STACK.set(true);
                operation()
            })?
            .join()
            .map_err(|_| io::Error::other("semantic worker panicked"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::hash::{Hash, Hasher};

    #[derive(Clone, Debug, Eq, PartialEq, Hash)]
    struct Spine(Option<Box<Spine>>);

    #[test]
    fn physical_memory_sets_reservation_without_a_project_depth_limit() {
        assert_eq!(
            reservation_from_meminfo("MemTotal: 1024 kB\n").unwrap(),
            2 * 1024 * 1024
        );
        assert!(reservation_from_meminfo("MemTotal: 0 kB\n").is_err());
        assert!(reservation_from_meminfo("MemFree: 1 kB\n").is_err());
        assert!(reservation_from_meminfo(&format!("MemTotal: {} kB\n", usize::MAX)).is_err());
    }

    #[test]
    fn deep_recursive_lifecycle_runs_and_cleans_up_on_the_reserved_stack() {
        run("recursive-lifecycle", || {
            let worker = thread::current().id();
            run("nested-operation", || {
                assert_eq!(thread::current().id(), worker)
            })
            .unwrap();
            for depth in [1_000, 10_000, 100_000] {
                let mut value = Spine(None);
                for _ in 0..depth {
                    value = Spine(Some(Box::new(value)));
                }
                let copy = value.clone();
                assert_eq!(value, copy);
                let mut left = std::collections::hash_map::DefaultHasher::new();
                let mut right = std::collections::hash_map::DefaultHasher::new();
                value.hash(&mut left);
                copy.hash(&mut right);
                assert_eq!(left.finish(), right.finish());
                drop(copy);
                let rejected: Result<(), Spine> = Err(value);
                drop(rejected);
            }
        })
        .unwrap();
    }
}
