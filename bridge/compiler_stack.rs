//! Resource-only Rust bootstrap bridge. The owned host replaces this module.
//! Stack size is a virtual-address reservation, not committed or resident memory.
use std::fs;
use std::io;
use std::path::PathBuf;

fn kib(line: &str) -> Option<usize> {
    line.split_whitespace()
        .nth(1)?
        .parse::<usize>()
        .ok()?
        .checked_mul(1024)
}

fn cgroup_extent(name: &str, mut extent: usize) -> usize {
    // A cgroup can inherit a tighter bound from any parent. Missing/v1 metadata
    // leaves the physical/swap observation intact; it is not a guessed quota.
    let Ok(groups) = fs::read_to_string("/proc/self/cgroup") else {
        return extent;
    };
    let Some(group) = groups.lines().find_map(|line| line.strip_prefix("0::")) else {
        return extent;
    };
    let root = PathBuf::from("/sys/fs/cgroup");
    let mut directory = root.join(group.trim_start_matches('/'));
    loop {
        if let Ok(text) = fs::read_to_string(directory.join(name)) {
            if let Ok(limit) = text.trim().parse::<usize>() {
                extent = extent.min(limit);
            }
        }
        if directory == root || !directory.pop() {
            break;
        }
    }
    extent
}

pub fn reservation() -> io::Result<usize> {
    let memory = fs::read_to_string("/proc/meminfo")?;
    let physical = memory
        .lines()
        .find(|line| line.starts_with("MemTotal:"))
        .and_then(kib)
        .ok_or_else(|| io::Error::other("cannot read physical memory extent"))?;
    let swap = memory
        .lines()
        .find(|line| line.starts_with("SwapTotal:"))
        .and_then(kib)
        .ok_or_else(|| io::Error::other("cannot read swap extent"))?;
    let mut extent = cgroup_extent("memory.max", physical)
        .checked_add(cgroup_extent("memory.swap.max", swap))
        .ok_or_else(|| {
            io::Error::other("memory and swap extents overflow the host address type")
        })?;
    let limits = fs::read_to_string("/proc/self/limits")?;
    if let Some(limit) = limits
        .lines()
        .find(|line| line.starts_with("Max address space"))
        .and_then(|line| line.split_whitespace().nth(3))
        .and_then(|text| text.parse::<usize>().ok())
    {
        let status = fs::read_to_string("/proc/self/status")?;
        let mapped = status
            .lines()
            .find(|line| line.starts_with("VmSize:"))
            .and_then(kib)
            .ok_or_else(|| io::Error::other("cannot read mapped address-space extent"))?;
        // Under an explicit address-space limit leave an equal share of the
        // remaining space for the compiler heap and thread/runtime mappings.
        extent = extent.min(limit.saturating_sub(mapped) / 2);
    }
    if extent == 0 {
        return Err(io::Error::other(
            "no address space available for compiler stack",
        ));
    }
    Ok(extent)
}

pub fn run<T: Send>(work: impl FnOnce() -> T + Send) -> io::Result<T> {
    run_with_reservation(reservation()?, work)
}

fn run_with_reservation<T: Send>(
    mut stack: usize,
    work: impl FnOnce() -> T + Send,
) -> io::Result<T> {
    // pthread creation can reject a reservation even below observed memory
    // extents: address-space fragmentation, commit accounting and concurrent
    // workers are live host constraints. A failed spawn must not consume work.
    let pending = std::sync::Mutex::new(Some(work));
    std::thread::scope(|scope| {
        loop {
            let pending = &pending;
            let worker = std::thread::Builder::new()
                .name("lkjscript-compiler".to_string())
                .stack_size(stack)
                .spawn_scoped(scope, move || {
                    let work = pending
                        .lock()
                        .unwrap()
                        .take()
                        .expect("compiler task starts once");
                    work()
                });
            match worker {
                Ok(worker) => {
                    return match worker.join() {
                        Ok(result) => Ok(result),
                        Err(panic) => std::panic::resume_unwind(panic),
                    };
                }
                Err(error) => {
                    if !matches!(
                        error.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::OutOfMemory
                    ) || stack <= 1
                    {
                        return Err(error);
                    }
                    // This adapts only host virtual-address reservation. There is
                    // no source-size/depth test, and no work has started yet.
                    stack /= 2;
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::run_with_reservation;
    use std::process::Command;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn rejected_reservations_preserve_exactly_once_work_and_panic_payload() {
        const CHILD: &str = "LKJSCRIPT_STACK_RESERVATION_RETRY_CHILD";
        if std::env::var_os(CHILD).is_some() {
            // The child has only 256 MiB of virtual address space. This first
            // 1 GiB reservation must fail, independently of host RAM/overcommit.
            struct Token(String);
            let token = Token("once".to_string());
            let calls = AtomicUsize::new(0);
            let calls_ref = &calls;
            let result = run_with_reservation(1024 * 1024 * 1024, move || {
                calls_ref.fetch_add(1, Ordering::SeqCst);
                token
            })
            .unwrap();
            assert_eq!(result.0, "once");
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            let panic = std::panic::catch_unwind(|| {
                let _ =
                    run_with_reservation(1024 * 1024 * 1024, || std::panic::panic_any(37_usize));
            })
            .unwrap_err();
            assert_eq!(panic.downcast_ref::<usize>(), Some(&37));
            return;
        }
        let output = Command::new("bash").arg("-c")
            .arg("ulimit -v 262144; exec \"$1\" --exact compiler_stack::tests::rejected_reservations_preserve_exactly_once_work_and_panic_payload --nocapture")
            .arg("stack-reservation-probe").arg(std::env::current_exe().unwrap())
            .env(CHILD, "1").output().unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
    }
}
