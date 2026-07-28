#[allow(dead_code)]
#[path = "linux_host/fixture.rs"]
mod fixture;

use fixture::Fixture;
use lkjscript_sys::{discover_linux_host_at, ConfigValue, LinuxFactSource, SchedExtState};

#[test]
fn rejects_malformed_oversize_and_affinity_mismatch_files() -> Result<(), Box<dyn std::error::Error>>
{
    let malformed = Fixture::new(&[0], "0", "0")?;
    malformed.write("sys/devices/system/cpu/online", "0-")?;
    assert_eq!(
        discover_linux_host_at(malformed.path())
            .err()
            .ok_or("accepted malformed")?
            .code,
        "cpu-online"
    );

    let oversize = Fixture::new(&[0], "0", "0")?;
    oversize.write("proc/self/status", &"x".repeat(1_048_577))?;
    assert_eq!(
        discover_linux_host_at(oversize.path())
            .err()
            .ok_or("accepted oversize")?
            .code,
        "file-bound"
    );

    let mismatch = Fixture::new(&[0, 1], "0-1", "0-1")?;
    mismatch.status("proc/self/status", "0", "1")?;
    assert_eq!(
        discover_linux_host_at(mismatch.path())
            .err()
            .ok_or("accepted mismatch")?
            .code,
        "affinity-mismatch"
    );
    Ok(())
}

#[test]
fn rejects_fixture_symlink_escape() -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::symlink;

    let fixture = Fixture::new(&[0], "0", "0")?;
    fixture.remove("sys/devices/system/cpu/cpu0/topology/die_id")?;
    symlink(
        "/etc/passwd",
        fixture
            .path()
            .join("sys/devices/system/cpu/cpu0/topology/die_id"),
    )?;
    let error = discover_linux_host_at(fixture.path())
        .err()
        .ok_or("accepted symlink escape")?;
    assert_eq!(error.code, "symlink-escape");
    Ok(())
}

#[test]
fn sched_ext_disabled_and_unknown_remain_distinct() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new(&[0], "0", "0")?;
    let unknown = discover_linux_host_at(fixture.path())?;
    assert_eq!(
        unknown.scheduler.sched_ext_state.value,
        SchedExtState::Unknown
    );
    fixture.write(
        "proc/config.gz",
        "compressed bytes are intentionally not inferred",
    )?;
    let compressed = discover_linux_host_at(fixture.path())?;
    assert_eq!(compressed.scheduler.sched_cache.value, ConfigValue::Unknown);
    assert_eq!(
        compressed.scheduler.sched_cache.source,
        LinuxFactSource::Procfs
    );
    assert!(!compressed.scheduler.sched_cache.certain);
    fixture.write("sys/kernel/sched_ext/state", "disabled")?;
    let disabled = discover_linux_host_at(fixture.path())?;
    assert_eq!(
        disabled.scheduler.sched_ext_state.value,
        SchedExtState::Disabled
    );
    assert!(disabled.scheduler.sched_ext_state.certain);
    Ok(())
}
