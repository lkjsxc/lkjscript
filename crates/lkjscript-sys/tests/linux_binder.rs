use lkjscript_resource::{CpuSet, PlacementMode, WorkerBinder, WorkerId};
use lkjscript_sys::{current_thread_affinity, LinuxWorkerBinder};

#[test]
fn kernel_managed_is_noop_and_pinned_binder_reads_back_exactly(
) -> Result<(), Box<dyn std::error::Error>> {
    let original = current_thread_affinity()?;
    let cpu = *original.as_slice().first().ok_or("empty affinity")?;
    let exact = CpuSet::new([cpu])?;
    let managed = LinuxWorkerBinder::for_mode(PlacementMode::KernelManaged);
    assert!(!managed.enabled());
    managed.bind(WorkerId::new(0, 1), &exact)?;
    assert_eq!(current_thread_affinity()?, original);

    let handle = std::thread::spawn(move || {
        let binder = LinuxWorkerBinder::for_mode(PlacementMode::CpuPinned);
        binder.bind(WorkerId::new(1, 1), &exact)?;
        current_thread_affinity()
            .map_err(|error| lkjscript_resource::ResourceError::new(error.code, error.detail))
    });
    match handle.join().map_err(|_| "binder worker panicked")? {
        Ok(observed) => assert_eq!(observed.as_slice(), &[cpu]),
        Err(error)
            if error.code == "affinity-write"
                && (error.detail == "errno 1" || error.detail == "errno 13") => {}
        Err(error) => return Err(error.into()),
    }
    Ok(())
}
