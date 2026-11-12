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
    let stack = reservation()?;
    std::thread::scope(|scope| {
        let worker = std::thread::Builder::new()
            .name("lkjscript-compiler".to_string())
            .stack_size(stack)
            .spawn_scoped(scope, work)?;
        match worker.join() {
            Ok(result) => Ok(result),
            Err(panic) => std::panic::resume_unwind(panic),
        }
    })
}
