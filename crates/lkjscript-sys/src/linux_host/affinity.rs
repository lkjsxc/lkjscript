use lkjscript_resource::{
    CpuSet, PlacementMode, ResourceError, ResourceResult, WorkerBinder, WorkerId, MAX_CPU, MAX_CPUS,
};

use super::error::{HostResult, LinuxHostError};
use super::linux_abi;

const MASK_BYTES: usize = (MAX_CPU as usize + 8) / 8;

pub fn current_process_affinity() -> HostResult<CpuSet> {
    read_affinity(linux_abi::process_id())
}

pub fn current_thread_affinity() -> HostResult<CpuSet> {
    read_affinity(0)
}

fn read_affinity(pid: i32) -> HostResult<CpuSet> {
    let mut mask = vec![0_u8; MASK_BYTES];
    linux_abi::get_affinity(pid, &mut mask)
        .map_err(|errno| LinuxHostError::new("affinity-read", format!("errno {errno}")))?;
    let mut cpus = Vec::new();
    for (byte_index, byte) in mask.into_iter().enumerate() {
        for bit in 0..8 {
            if byte & (1 << bit) != 0 {
                if cpus.len() == MAX_CPUS {
                    return Err(LinuxHostError::new("affinity-bound", "too many CPUs"));
                }
                cpus.push((byte_index * 8 + bit) as u32);
            }
        }
    }
    CpuSet::new(cpus).map_err(Into::into)
}

fn write_thread_affinity(allowed: &CpuSet) -> HostResult<()> {
    let mut mask = vec![0_u8; MASK_BYTES];
    for cpu in allowed.as_slice() {
        let index = *cpu as usize;
        mask[index / 8] |= 1 << (index % 8);
    }
    linux_abi::set_affinity(0, &mask)
        .map_err(|errno| LinuxHostError::new("affinity-write", format!("errno {errno}")))?;
    let observed = current_thread_affinity()?;
    if observed != *allowed {
        return Err(LinuxHostError::new(
            "affinity-readback",
            format!("requested {allowed:?}, observed {observed:?}"),
        ));
    }
    Ok(())
}

#[derive(Debug)]
pub struct AffinityGuard {
    saved: CpuSet,
    active: bool,
}

impl AffinityGuard {
    pub fn bind(allowed: &CpuSet) -> HostResult<Self> {
        let saved = current_thread_affinity()?;
        write_thread_affinity(allowed)?;
        Ok(Self {
            saved,
            active: true,
        })
    }

    pub fn restore(mut self) -> HostResult<()> {
        write_thread_affinity(&self.saved)?;
        self.active = false;
        Ok(())
    }
}

impl Drop for AffinityGuard {
    fn drop(&mut self) {
        if self.active {
            let _ = write_thread_affinity(&self.saved);
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LinuxWorkerBinder {
    enabled: bool,
}

impl LinuxWorkerBinder {
    pub const fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    pub const fn for_mode(mode: PlacementMode) -> Self {
        Self::new(!matches!(mode, PlacementMode::KernelManaged))
    }

    pub const fn enabled(self) -> bool {
        self.enabled
    }
}

impl WorkerBinder for LinuxWorkerBinder {
    fn bind(&self, _worker: WorkerId, allowed: &CpuSet) -> ResourceResult<()> {
        if !self.enabled {
            return Ok(());
        }
        write_thread_affinity(allowed).map_err(|error| ResourceError::new(error.code, error.detail))
    }
}
