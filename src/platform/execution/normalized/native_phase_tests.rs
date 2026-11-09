//! Invocation-level native phase ownership, promotion and failure recovery.
//! Independent /proc maps and slab walks corroborate internal observations.
//! Internal compiler subpass reclamation and language Regions remain separate.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
use super::artifact_support::run_filter;
use super::{
    Check, FOCUSED_DEADLINE, SourceOutputMode, WHOLE_SOURCE_DEADLINE, bootstrap_compiler, ensure,
    equal, equal_bytes, fixture_case, function_named, host_oracle, native_image, sha256,
};
use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::FileExt;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::time::{Duration, Instant};
#[path = "native_phase_caller.rs"]
mod caller;
use caller::{Mode, word};
#[path = "native_scalar_boundary_tests.rs"]
mod scalar_boundaries;
#[path = "native_scalar_probe_tests.rs"]
mod scalar_probe;
const TEST_ADDRESS_SPACE: u64 = 32 * 1024 * 1024 * 1024;
const SLAB: u64 = 65_536;

#[derive(Clone, Copy)]
struct PhasePolicy {
    address_space: Option<u64>,
    deadline: Duration,
}

impl PhasePolicy {
    fn bounded(deadline: Duration) -> Self {
        Self {
            address_space: Some(TEST_ADDRESS_SPACE),
            deadline,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct AddressSpaceLimits {
    soft: Option<u64>,
    hard: Option<u64>,
}

fn address_space_argument(limit: Option<u64>) -> Check<String> {
    match limit {
        None => Ok("unlimited".to_owned()),
        // Linux reserves this rlim_t representation for RLIM_INFINITY.
        Some(u64::MAX) => {
            Err("finite phase address-space allowance exceeds the OS representation".to_owned())
        }
        Some(bytes) => Ok(bytes.to_string()),
    }
}

fn parse_address_space_limits(text: &str) -> Check<AddressSpaceLimits> {
    fn limit(value: Option<&str>) -> Check<Option<u64>> {
        match value.ok_or("phase address-space limit absent")? {
            "unlimited" => Ok(None),
            value => {
                ensure(
                    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()),
                    "phase address-space limit integer",
                )?;
                let bytes = value
                    .parse::<u64>()
                    .map_err(|_| "phase address-space limit extent")?;
                address_space_argument(Some(bytes))?;
                Ok(Some(bytes))
            }
        }
    }
    let mut rows = text
        .lines()
        .filter_map(|line| line.strip_prefix("Max address space"));
    let row = rows.next().ok_or("phase Max address space row absent")?;
    ensure(
        rows.next().is_none(),
        "duplicate phase Max address space row",
    )?;
    let mut fields = row.split_whitespace();
    let soft = limit(fields.next())?;
    let hard = limit(fields.next())?;
    ensure(
        fields.next() == Some("bytes") && fields.next().is_none(),
        "phase address-space limit units/framing",
    )?;
    Ok(AddressSpaceLimits { soft, hard })
}

fn process_address_space(pid: u32) -> Check<AddressSpaceLimits> {
    let text = fs::read_to_string(format!("/proc/{pid}/limits"))
        .map_err(|error| format!("phase process limits: {error}"))?;
    parse_address_space_limits(&text)
}

fn observe_address_space(
    label: &str,
    pid: u32,
    frame: u64,
    event: &str,
    selected: PhasePolicy,
) -> Check<()> {
    let observed = process_address_space(pid)?;
    equal(
        observed,
        AddressSpaceLimits {
            soft: selected.address_space,
            hard: selected.address_space,
        },
        "actual phase address-space allowance",
    )?;
    eprintln!(
        "NATIVE_PHASE_POLICY {}",
        serde_json::json!({
            "case": label,
            "event": event,
            "frame": frame,
            "pid": pid,
            "selected_address_space": address_space_argument(selected.address_space)?,
            "soft": address_space_argument(observed.soft)?,
            "hard": address_space_argument(observed.hard)?,
            "deadline_seconds": selected.deadline.as_secs().to_string(),
        })
    );
    Ok(())
}
struct Joined(Child);
impl Drop for Joined {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
#[derive(Clone)]
struct Mapping {
    start: u64,
    end: u64,
    anonymous_rw: bool,
}
struct Snapshot {
    maps: Vec<Mapping>,
    anonymous: u64,
    rss_kib: u64,
    vm_bytes: u64,
}

fn process_snapshot(pid: u32) -> Check<Snapshot> {
    let text =
        fs::read_to_string(format!("/proc/{pid}/maps")).map_err(|e| format!("phase maps: {e}"))?;
    let mut maps = Vec::new();
    let mut anonymous = 0u64;
    for line in text.lines() {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        let (start, end) = fields
            .first()
            .ok_or("maps extent absent")?
            .split_once('-')
            .ok_or("maps extent framing")?;
        let start = u64::from_str_radix(start, 16).map_err(|_| "maps start")?;
        let end = u64::from_str_radix(end, 16).map_err(|_| "maps end")?;
        ensure(start < end, "maps empty/inverted extent")?;
        let anonymous_rw = fields.get(1).copied() == Some("rw-p") && fields.len() == 5;
        if anonymous_rw {
            anonymous = anonymous
                .checked_add(end - start)
                .ok_or("maps total overflow")?;
        }
        maps.push(Mapping {
            start,
            end,
            anonymous_rw,
        });
    }
    fn metric(text: &str, key: &str) -> Check<u64> {
        let row = text
            .lines()
            .find(|line| line.starts_with(key))
            .ok_or("process memory metric absent")?;
        row.split_whitespace()
            .nth(1)
            .ok_or("process memory metric framing")?
            .parse::<u64>()
            .map_err(|_| "process memory metric integer".to_owned())
    }
    let status = fs::read_to_string(format!("/proc/{pid}/status"))
        .map_err(|e| format!("phase status: {e}"))?;
    let smaps = fs::read_to_string(format!("/proc/{pid}/smaps_rollup"))
        .map_err(|e| format!("phase smaps: {e}"))?;
    Ok(Snapshot {
        maps,
        anonymous,
        rss_kib: metric(&smaps, "Rss:")?,
        vm_bytes: metric(&status, "VmSize:")?
            .checked_mul(1024)
            .ok_or("VmSize byte extent")?,
    })
}

fn memory(file: &File, address: u64, bytes: usize) -> Check<Vec<u8>> {
    let mut result = vec![0; bytes];
    file.read_exact_at(&mut result, address)
        .map_err(|e| format!("phase memory at {address:#x}: {e}"))?;
    Ok(result)
}

#[derive(Default)]
struct Context {
    bytes: u64,
    slabs: u64,
    root: u64,
    ranges: Vec<(u64, u64)>,
}
impl Context {
    fn contains(&self, start: u64, bytes: u64) -> bool {
        start.checked_add(bytes).is_some_and(|end| {
            self.ranges
                .iter()
                .any(|(low, high)| *low + 64 <= start && end <= *high)
        })
    }
}

fn context(
    mem: &File,
    address: u64,
    bytes: &[u8],
    snapshot: &Snapshot,
    closing: bool,
) -> Check<Context> {
    let mut result = Context {
        root: word(bytes, 56)?,
        ..Context::default()
    };
    let mut head = word(bytes, 0)?;
    let mut seen = BTreeSet::new();
    while head != 0 {
        ensure(seen.insert(head), "native slab ownership cycle")?;
        // Head/previous encode the nonzero payload address, permitting a real
        // mapping at address zero without colliding with the empty-chain marker.
        let base = head.checked_sub(64).ok_or("native slab payload link")?;
        let header = memory(mem, base, 64)?;
        let extent = word(&header, 8)?;
        ensure(
            base.is_multiple_of(4096) && extent >= SLAB && extent.is_multiple_of(4096),
            "native slab physical extent/alignment",
        )?;
        let end = base.checked_add(extent).ok_or("native slab end overflow")?;
        ensure(
            word(&header, 16)? == address && word(&header, 24)? == head,
            "native slab owner/payload",
        )?;
        ensure(
            snapshot
                .maps
                .iter()
                .any(|r| r.anonymous_rw && r.start <= base && end <= r.end),
            "owned slab absent from independent process maps",
        )?;
        result.ranges.push((base, end));
        result.bytes = result
            .bytes
            .checked_add(extent)
            .ok_or("native slab byte total")?;
        result.slabs = result.slabs.checked_add(1).ok_or("native slab count")?;
        head = word(&header, 0)?;
    }
    ensure(
        word(bytes, 40)? == 0,
        "finite phase control unexpectedly saturated observations",
    )?;
    equal(
        word(bytes, 24)?,
        result.slabs,
        "slab count independently walked",
    )?;
    equal(
        word(bytes, 32)?,
        result.bytes,
        "mapped bytes independently walked",
    )?;
    if result.slabs == 0 {
        ensure(
            word(bytes, 8)? == 0 && word(bytes, 16)? == 0 && result.root == 0,
            "empty context retains allocation/root pointers",
        )?;
    } else if closing && word(bytes, 48)? == 1 {
        ensure(
            word(bytes, 8)? == 0 && word(bytes, 16)? == 0 && result.root == 0,
            "failed disposal retained exposed allocation/root pointers",
        )?;
    } else {
        ensure(word(bytes, 48)? == 0, "live output context is not open")?;
        ensure(
            result.contains(word(bytes, 8)?, 0),
            "live cursor outside owned payload",
        )?;
    }
    if result.root != 0 {
        ensure(
            result.contains(result.root, 24),
            "result record not output-owned",
        )?;
        let record = memory(mem, result.root, 24)?;
        let length = word(&record, 8)?;
        let data = word(&record, 16)?;
        ensure(
            length == 0 || result.contains(data, length),
            "surviving bytes not output-owned",
        )?;
    }
    Ok(result)
}

struct Frame {
    id: u64,
    status: u64,
    diagnostic: u64,
    base: u64,
    handles: Vec<u8>,
    image: Vec<u8>,
}
struct Observation {
    attempts: [u64; 2],
    owned: u64,
    snapshot: Snapshot,
}
#[derive(Debug, PartialEq, Eq)]
struct AllocationObservation {
    frame: u64,
    status: u64,
    diagnostic: u64,
    output: Vec<u8>,
    attempts: [u64; 2],
    attempt_saturation: [u64; 2],
    owned_mapped_bytes: u64,
    output_slabs: [u64; 2],
    output_bytes: [u64; 2],
    output_roots_live: [bool; 2],
}

fn observe(pid: u32, mem: &File, frame: &Frame, baseline: u64) -> Check<Observation> {
    let snapshot = process_snapshot(pid)?;
    let actual = memory(mem, frame.base, 320)?;
    equal_bytes(
        &actual,
        &frame.handles,
        "framed and independently read handles",
    )?;
    let mut owned = 0u64;
    let mut all_ranges = Vec::new();
    let mut attempts = [0; 2];
    for (index, offset) in [0usize, 160].into_iter().enumerate() {
        let handle = actual
            .get(offset..offset + 160)
            .ok_or("phase handle framing")?;
        let address = frame
            .base
            .checked_add(u64::try_from(offset).map_err(|_| "handle offset")?)
            .ok_or("handle address overflow")?;
        let scratch = context(mem, address, &handle[..64], &snapshot, false)?;
        let output = context(
            mem,
            address + 64,
            &handle[64..128],
            &snapshot,
            frame.id == 203 && offset == 160,
        )?;
        ensure(
            scratch.bytes == 0 && scratch.slabs == 0,
            "scratch survived completed native invocation",
        )?;
        ensure(
            word(handle, 152)? == 0,
            "finite allocation observations unexpectedly saturated",
        )?;
        attempts[index] = word(handle, 144)?;
        owned = owned
            .checked_add(output.bytes)
            .ok_or("output ownership byte extent")?;
        all_ranges.extend(output.ranges);
    }
    all_ranges.sort_unstable();
    ensure(
        all_ranges.windows(2).all(|p| p[0].1 <= p[1].0),
        "contexts share owned slab extents",
    )?;
    equal(
        snapshot.anonymous,
        baseline
            .checked_add(owned)
            .ok_or("anonymous extent overflow")?,
        "independent anonymous maps equal exactly retained output ownership",
    )?;
    Ok(Observation {
        attempts,
        owned,
        snapshot,
    })
}

fn policy(pid: u32, limit: Option<u64>) -> Check<()> {
    let limit = address_space_argument(limit)?;
    let output = Command::new("/usr/bin/prlimit")
        .env_clear()
        .args(["--pid", &pid.to_string(), &format!("--as={limit}:")])
        .output()
        .map_err(|e| format!("phase explicit process policy: {e}"))?;
    ensure(
        output.status.success() && output.stderr.is_empty(),
        &format!(
            "phase policy failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ),
    )
}

fn cpu_ticks(pid: u32) -> Check<u64> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat"))
        .map_err(|e| format!("phase CPU observation: {e}"))?;
    let (_, fields) = stat.rsplit_once(')').ok_or("process stat framing")?;
    let fields = fields.split_whitespace().collect::<Vec<_>>();
    let user = fields
        .get(11)
        .ok_or("process utime")?
        .parse::<u64>()
        .map_err(|_| "process utime integer")?;
    let system = fields
        .get(12)
        .ok_or("process stime")?
        .parse::<u64>()
        .map_err(|_| "process stime integer")?;
    user.checked_add(system)
        .ok_or_else(|| "process CPU tick extent".to_owned())
}
struct Driver {
    child: Joined,
    input: ChildStdin,
    output: File,
    memory: Option<File>,
    offset: u64,
    base: u64,
    start: Instant,
    deadline: Duration,
    mid_cancel: bool,
    cancelled: bool,
    allocation_free: bool,
    cancel_started: Instant,
    cancel_cpu_ticks: u64,
}
impl Driver {
    fn wait_bytes(&mut self, extent: u64) -> Check<()> {
        loop {
            if self
                .output
                .metadata()
                .map_err(|e| format!("phase capture metadata: {e}"))?
                .len()
                >= extent
            {
                return Ok(());
            }
            if let Some(status) = self
                .child
                .0
                .try_wait()
                .map_err(|e| format!("phase wait: {e}"))?
            {
                return Err(format!(
                    "native phase caller exited before frame completion: {status:?}"
                ));
            }
            if self.start.elapsed() >= self.deadline {
                return Err(format!(
                    "native phase explicit test deadline {:?} exceeded",
                    self.deadline
                ));
            }
            if self.mid_cancel && !self.cancelled {
                let mem = self
                    .memory
                    .as_ref()
                    .ok_or("phase cancellation memory absent")?;
                let live = word(&memory(mem, self.base + 160, 64)?, 32)?;
                let running = if self.allocation_free {
                    self.cancel_started.elapsed() >= Duration::from_millis(50)
                        && cpu_ticks(self.child.0.id())?
                            >= self
                                .cancel_cpu_ticks
                                .checked_add(2)
                                .ok_or("CPU tick boundary")?
                } else {
                    live >= 8 * 1024 * 1024
                };
                if running {
                    if self.allocation_free {
                        let handle = memory(mem, self.base + 160, 160)?;
                        ensure(
                            live == 0 && word(&handle, 0)? == 0 && word(&handle, 144)? == 0,
                            "allocation-free loop allocated scratch or bypassed exact attempts",
                        )?;
                    }
                    let snapshot = process_snapshot(self.child.0.id())?;
                    mem.write_all_at(&1u64.to_le_bytes(), self.base + 352)
                        .map_err(|e| format!("native mid-work cancellation flag: {e}"))?;
                    self.cancelled = true;
                    eprintln!(
                        "native phase cancellation live_scratch_bytes={live} anonymous_mapped_bytes={} rss_kib={}",
                        snapshot.anonymous, snapshot.rss_kib
                    );
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    fn frame(&mut self, maximum_output: usize) -> Check<Frame> {
        let header_end = self.offset.checked_add(360).ok_or("phase frame extent")?;
        self.wait_bytes(header_end)?;
        let header = memory(&self.output, self.offset, 360)?;
        let length = usize::try_from(word(&header, 24)?).map_err(|_| "phase output domain")?;
        ensure(
            length <= maximum_output,
            "phase output exceeds independently expected result",
        )?;
        let end = header_end
            .checked_add(u64::try_from(length).map_err(|_| "phase output extent")?)
            .ok_or("phase frame end")?;
        self.wait_bytes(end)?;
        let image = memory(&self.output, header_end, length)?;
        let base = word(&header, 32)?;
        if self.memory.is_none() {
            ensure(
                base != 0 && base.is_multiple_of(8),
                "phase handle address/alignment",
            )?;
            self.memory = Some(
                File::options()
                    .read(true)
                    .write(true)
                    .open(format!("/proc/{}/mem", self.child.0.id()))
                    .map_err(|e| format!("phase process memory: {e}"))?,
            );
            self.base = base;
        }
        equal(base, self.base, "caller handle address remains stable")?;
        self.offset = end;
        Ok(Frame {
            id: word(&header, 0)?,
            status: word(&header, 8)?,
            diagnostic: word(&header, 16)?,
            base,
            handles: header.get(40..360).ok_or("phase frame handles")?.to_vec(),
            image,
        })
    }
    fn advance(&mut self) -> Check<()> {
        self.input
            .write_all(&[1])
            .map_err(|e| format!("native phase handshake: {e}"))
    }
    fn finish(&mut self) -> Check<()> {
        loop {
            if let Some(status) = self
                .child
                .0
                .try_wait()
                .map_err(|e| format!("phase terminal: {e}"))?
            {
                ensure(
                    status.success(),
                    &format!("native phase terminal {status:?}"),
                )?;
                return equal(
                    self.output
                        .metadata()
                        .map_err(|e| format!("phase final capture: {e}"))?
                        .len(),
                    self.offset,
                    "all native output frames consumed",
                );
            }
            if self.start.elapsed() >= self.deadline {
                return Err("native phase terminal deadline".to_owned());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

struct CleanupFault {
    head: u64,
    extent: u64,
    slabs: u64,
    bytes: u64,
}

fn verify(
    label: &str,
    image: &[u8],
    request: &[u8],
    expected: &[u8],
    mode: Mode,
    deadline: Duration,
    alternate: Option<(&[u8], &[u8])>,
) -> Check<()> {
    verify_observed(
        label,
        image,
        request,
        expected,
        mode,
        PhasePolicy::bounded(deadline),
        alternate,
    )
    .map(|_| ())
}

fn verify_observed(
    label: &str,
    image: &[u8],
    request: &[u8],
    expected: &[u8],
    mode: Mode,
    phase_policy: PhasePolicy,
    alternate: Option<(&[u8], &[u8])>,
) -> Check<Vec<AllocationObservation>> {
    let address_space = address_space_argument(phase_policy.address_space)?;
    if phase_policy.address_space.is_none() {
        ensure(
            process_address_space(std::process::id())?.hard.is_none(),
            "unbounded phase allowance requires an unlimited inherited hard address-space limit",
        )?;
    }
    crate::release_container::inspect_static_elf_bytes(image)
        .map_err(|e| format!("{label}: production ELF: {e:?}"))?;
    let wrapper = caller::wrap(image, request, alternate.map(|(input, _)| input), mode)?;
    crate::release_container::inspect_static_elf_bytes(&wrapper)
        .map_err(|e| format!("{label}: appended caller ELF: {e:?}"))?;
    let directory = tempfile::tempdir().map_err(|e| format!("phase tempdir: {e}"))?;
    let binary = directory.path().join("caller");
    crate::platform::owned_output::publish_executable_create_new(
        &binary,
        &wrapper,
        None,
        "native phase control",
    )
    .map_err(|e| format!("phase caller publication: {e:?}"))?;
    let output_path = directory.path().join("frames");
    let error_path = directory.path().join("stderr");
    let output = File::create(&output_path).map_err(|e| format!("phase capture: {e}"))?;
    let errors = File::create(&error_path).map_err(|e| format!("phase stderr: {e}"))?;
    let start = Instant::now();
    let mut child = Joined(
        Command::new("/usr/bin/prlimit")
            .env_clear()
            .arg(format!("--as={address_space}"))
            .args(["--core=0", "--"])
            .arg(&binary)
            .current_dir(directory.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::from(output))
            .stderr(Stdio::from(errors))
            .spawn()
            .map_err(|e| format!("phase spawn: {e}"))?,
    );
    let input = child.0.stdin.take().ok_or("phase handshake input absent")?;
    let mut driver = Driver {
        child,
        input,
        output: File::open(&output_path).map_err(|e| format!("phase frame reader: {e}"))?,
        memory: None,
        offset: 0,
        base: 0,
        start,
        deadline: phase_policy.deadline,
        mid_cancel: false,
        cancelled: false,
        allocation_free: mode == Mode::AllocationFree,
        cancel_started: start,
        cancel_cpu_ticks: 0,
    };
    let ids: &[u64] = match mode {
        Mode::Compiler => &[100, 0, 1, 2, 3, 20, 4, 5, 106, 6, 107, 7, 108, 8, 9, 10, 11],
        Mode::Echo => &[
            100, 0, 40, 41, 42, 43, 44, 1, 2, 203, 3, 20, 200, 21, 201, 22, 202, 8, 9, 30, 31, 10,
            11,
        ],
        Mode::ObserverLeak => &[100, 0, 300],
        Mode::AllocationFree => &[100, 110, 7, 1, 10, 11],
        Mode::RuntimeReject => &[100, 24, 1, 10, 11],
        Mode::AllocationIdentity => &[100, 0, 1, 2, 3, 8, 9, 30, 31, 10, 11],
    };
    let mut baseline = 0;
    let mut allocations = Vec::new();
    let mut normal_attempts = 0;
    let mut cleanup_fault: Option<CleanupFault> = None;
    let mut preflight_handles: Option<Vec<u8>> = None;
    let maximum_output = expected
        .len()
        .max(alternate.map(|(_, output)| output.len()).unwrap_or(0));
    for id in ids {
        let frame = driver.frame(maximum_output)?;
        equal(frame.id, *id, "native phase frame order")?;
        let pid = driver.child.0.id();
        if *id == 100 {
            observe_address_space(label, pid, *id, "startup", phase_policy)?;
            baseline = process_snapshot(pid)?.anonymous;
        }
        let mem = driver
            .memory
            .as_ref()
            .ok_or("phase independent memory observer absent")?;
        if *id == 203 {
            let fault = cleanup_fault
                .take()
                .ok_or("cleanup fault was not installed")?;
            equal(
                frame.status,
                4,
                "actual munmap refusal surfaced as cleanup failure",
            )?;
            ensure(
                frame.image.is_empty(),
                "cleanup failure exposed an output root",
            )?;
            equal(
                word(&frame.handles, 224)?,
                fault.head,
                "failed munmap retains owned head",
            )?;
            equal(
                word(&frame.handles, 248)?,
                fault.slabs,
                "failed munmap retains slab count",
            )?;
            equal(
                word(&frame.handles, 256)?,
                fault.bytes,
                "failed munmap retains mapped bytes",
            )?;
            equal(
                word(&frame.handles, 272)?,
                1,
                "failed output context is closing",
            )?;
            equal(
                word(&frame.handles, 280)?,
                0,
                "failed output root is hidden",
            )?;
            equal(
                word(&frame.handles, 288)?,
                4,
                "handle records cleanup refusal",
            )?;
            let base = fault
                .head
                .checked_sub(64)
                .ok_or("cleanup fault payload link")?;
            equal(
                word(&memory(mem, base, 64)?, 8)?,
                0,
                "zero-extent fault reached disposer",
            )?;
            mem.write_all_at(&fault.extent.to_le_bytes(), base + 8)
                .map_err(|e| format!("restore native slab extent: {e}"))?;
            eprintln!(
                "native phase cleanup refusal case={label} fault=zero-length-munmap expected_errno=EINVAL completion=4 head={} restored_extent={} retry=next",
                fault.head, fault.extent
            );
        }
        let observation = observe(pid, mem, &frame, baseline);
        if *id == 300 {
            let error = observation
                .err()
                .ok_or("observer accepted forgotten unfreed mappings")?;
            ensure(
                error.contains("independent anonymous maps"),
                "observer control failed before mapping comparison",
            )?;
            eprintln!(
                "native phase observer control case={label} status=passed detected=forgotten-unmapped-slabs production_instructions=unchanged"
            );
            return Ok(allocations);
        }
        let observation = observation?;
        if mode == Mode::AllocationIdentity {
            // observe() already checked these handles against /proc/$pid/mem,
            // walked every slab, rejected saturation, and compared exact maps.
            allocations.push(AllocationObservation {
                frame: *id,
                status: frame.status,
                diagnostic: frame.diagnostic,
                output: frame.image.clone(),
                attempts: observation.attempts,
                attempt_saturation: [word(&frame.handles, 152)?, word(&frame.handles, 312)?],
                owned_mapped_bytes: observation.owned,
                output_slabs: [word(&frame.handles, 88)?, word(&frame.handles, 248)?],
                output_bytes: [word(&frame.handles, 96)?, word(&frame.handles, 256)?],
                output_roots_live: [
                    word(&frame.handles, 120)? != 0,
                    word(&frame.handles, 280)? != 0,
                ],
            });
        }
        if *id == 0 && mode == Mode::Echo {
            preflight_handles = Some(frame.handles.clone());
        }
        if (40..=44).contains(id) {
            equal(
                frame.status,
                5,
                "invalid caller numeric extent/alignment refusal",
            )?;
            ensure(frame.image.is_empty(), "invalid caller exposed a result")?;
            equal_bytes(
                &frame.handles,
                preflight_handles
                    .as_ref()
                    .ok_or("invalid caller baseline absent")?,
                "invalid caller preserved both handles and allocation observations exactly",
            )?;
        }
        if *id == 24 {
            equal(frame.status, 3, "valid pure program runtime trap recovered")?;
            equal(
                word(&frame.handles, 288)?,
                3,
                "runtime trap completion recorded in handle",
            )?;
            ensure(
                frame.image.is_empty(),
                "runtime trap exposed incomplete output",
            )?;
        }
        if [0, 1, 2, 8, 9, 31].contains(id) {
            equal(frame.status, 0, "native invocation completion")?;
            equal(frame.diagnostic, 0, "native successful compiler diagnostic")?;
            let wanted = if *id == 1 {
                alternate.map(|(_, output)| output).unwrap_or(expected)
            } else {
                expected
            };
            equal_bytes(
                &frame.image,
                wanted,
                "promoted output survives reclaimed scratch and later phases",
            )?;
        }
        if *id == 1 {
            normal_attempts = observation.attempts[1];
        }
        if *id == 4 {
            equal(frame.status, 0, "diagnostic invocation completion")?;
            equal(frame.diagnostic, 12, "raw compiler diagnostic")?;
            ensure(frame.image.is_empty(), "rejected compiler emitted output")?;
        }
        if [6, 21, 22].contains(id) {
            equal(frame.status, 2, "actual kernel allocation refusal")?;
            ensure(
                frame.image.is_empty() && observation.attempts[1] > 0,
                "allocation refusal did not reach allocator",
            )?;
            if *id == 22 {
                equal(
                    observation.attempts[1],
                    normal_attempts,
                    "promotion failure reached final output allocation",
                )?;
                ensure(
                    normal_attempts >= 2,
                    "promotion control did not allocate separate survival storage",
                )?;
            }
            policy(pid, phase_policy.address_space)?;
            observe_address_space(label, pid, *id, "restored", phase_policy)?;
        }
        if [7, 20].contains(id) {
            equal(frame.status, 1, "native cancellation completion")?;
            ensure(frame.image.is_empty(), "cancelled compiler emitted output")?;
            if *id == 7 {
                ensure(driver.cancelled, "mid-work cancellation not delivered")?;
                if mode == Mode::AllocationFree {
                    equal(
                        observation.attempts[1],
                        0,
                        "allocation-free cancellation attempt count",
                    )?;
                }
                driver.mid_cancel = false;
            }
        }
        if [3, 5, 10, 11, 30].contains(id) {
            equal(frame.status, 0, "native phase release")?;
            ensure(frame.image.is_empty(), "released output root survived")?;
        }
        if *id == 11 {
            equal(
                observation.owned,
                0,
                "all scratch and surviving output mappings released",
            )?;
        }
        eprintln!(
            "native phase case={label} frame={id} status={} diagnostic={} output_bytes={} output_sha256={} attempts={:?} owned_mapped_bytes={} anonymous_rw_bytes={} rss_kib={} vm_bytes={} elapsed_ms={}",
            frame.status,
            frame.diagnostic,
            frame.image.len(),
            sha256(&frame.image),
            observation.attempts,
            observation.owned,
            observation.snapshot.anonymous,
            observation.snapshot.rss_kib,
            observation.snapshot.vm_bytes,
            start.elapsed().as_millis()
        );
        match *id {
            2 if mode == Mode::Echo => {
                let head = word(&frame.handles, 224)?;
                ensure(head != 0, "cleanup fault requires live inner output")?;
                let base = head.checked_sub(64).ok_or("cleanup fault payload link")?;
                let extent = word(&memory(mem, base, 64)?, 8)?;
                cleanup_fault = Some(CleanupFault {
                    head,
                    extent,
                    slabs: word(&frame.handles, 248)?,
                    bytes: word(&frame.handles, 256)?,
                });
                mem.write_all_at(&0u64.to_le_bytes(), base + 8)
                    .map_err(|e| format!("inject zero extent for kernel munmap refusal: {e}"))?;
            }
            106 => policy(
                pid,
                Some(
                    observation
                        .snapshot
                        .vm_bytes
                        .checked_add(4 * SLAB)
                        .ok_or("compiler failure policy extent")?,
                ),
            )?,
            200 => policy(pid, Some(observation.snapshot.vm_bytes))?,
            201 => policy(
                pid,
                Some(
                    observation
                        .snapshot
                        .vm_bytes
                        .checked_add(2 * SLAB)
                        .ok_or("promotion failure policy extent")?,
                ),
            )?,
            107 => {
                driver.mid_cancel = true;
            }
            110 => {
                driver.mid_cancel = true;
                driver.cancel_started = Instant::now();
                driver.cancel_cpu_ticks = cpu_ticks(pid)?;
            }
            _ => {}
        }
        driver.advance()?;
    }
    driver.finish()?;
    ensure(
        fs::read(&error_path)
            .map_err(|e| format!("phase final diagnostics: {e}"))?
            .is_empty(),
        "native phase unexpected stderr",
    )?;
    eprintln!(
        "native phase verified case={label} frames={} pid={} compiler_sha256={} request_sha256={} oracle_sha256={} production_instructions=unchanged scratch_ownership=zero independent_maps=passed callee_saved_registers=preserved elapsed_ms={}",
        ids.len(),
        driver.child.0.id(),
        sha256(image),
        sha256(request),
        sha256(expected),
        start.elapsed().as_millis()
    );
    Ok(allocations)
}

/// Compare a matched pair whose only runtime source difference is Bind(0).
///
/// Each fixed-address ELF runs three invocations in its own long-lived process.
/// These are actual native allocator attempts and independently checked owner
/// mappings, not the evaluator's modeled cumulative allocation. The source
/// fixtures must cover static and already-bound callables with the same nonempty
/// Bind in both programs. Private callable pointer identity is separately checked
/// by the emitted-helper native control; the public ABI exposes promoted results.
pub(super) fn verify_empty_bind_identity_pair(
    baseline: &[u8],
    empty_bind: &[u8],
    input: &[u8],
    expected: &[u8],
) -> Check<()> {
    let ordinary = verify_observed(
        "source-callable-identity-baseline",
        baseline,
        input,
        expected,
        Mode::AllocationIdentity,
        PhasePolicy::bounded(FOCUSED_DEADLINE),
        None,
    )?;
    let bound = verify_observed(
        "source-callable-empty-bind-identity",
        empty_bind,
        input,
        expected,
        Mode::AllocationIdentity,
        PhasePolicy::bounded(FOCUSED_DEADLINE),
        None,
    )?;
    ensure(
        ordinary.len() == 11 && bound.len() == 11,
        "empty Bind allocation comparison missed invocation/release observations",
    )?;
    for frame in [0, 1, 8] {
        let invocation = ordinary
            .iter()
            .find(|observation| observation.frame == frame)
            .ok_or("empty Bind baseline invocation observation absent")?;
        let owner = usize::from(frame != 0);
        ensure(
            invocation.attempts[owner] > 0 && invocation.output_roots_live[owner],
            "empty Bind baseline did not reach observed allocation and output promotion",
        )?;
    }
    ensure(
        ordinary == bound,
        &format!(
            "empty Bind changed native allocation attempts or owned output mappings: baseline={ordinary:?}; empty_bind={bound:?}"
        ),
    )?;
    eprintln!(
        "native empty Bind pair verified baseline_sha256={} empty_bind_sha256={} input_sha256={} expected_sha256={} processes=2 invocations_per_process=3 observations_per_process={} allocator_attempts=equal saturation=absent independent_owner_maps=equal final_release=passed",
        sha256(baseline),
        sha256(empty_bind),
        sha256(input),
        sha256(expected),
        ordinary.len(),
    );
    Ok(())
}

fn echo_images() -> Check<(Vec<u8>, Vec<u8>)> {
    let case = fixture_case()?;
    let host = host_oracle(&case)?;
    let native = native_image(
        &host.program,
        function_named(&case.source, "artifact-controls", "binary-entry")?,
    )?;
    let source = run_filter(
        "native-phase/source-echo",
        bootstrap_compiler()?,
        &case.request("binary-main", SourceOutputMode::Elf)?,
        0,
        FOCUSED_DEADLINE,
    )?;
    Ok((native, source))
}

#[test]
fn native_elf_phase_reclaims_and_recovers_in_both_backends() -> Check<()> {
    let (host, source) = echo_images()?;
    // Explicit workload exceeds the two-slab allowance used to refuse final
    // payload promotion. Restore that child-only policy before the next attempt.
    let input = (0u8..=255).cycle().take(1_048_593).collect::<Vec<_>>();
    for (policy_label, address_space) in [("finite", Some(TEST_ADDRESS_SPACE)), ("unbounded", None)]
    {
        for (label, image) in [("Rust-echo", &host), ("source-echo", &source)] {
            verify_observed(
                &format!("{label}/{policy_label}"),
                image,
                &input,
                &input,
                Mode::Echo,
                PhasePolicy {
                    address_space,
                    deadline: FOCUSED_DEADLINE,
                },
                None,
            )?;
        }
    }
    Ok(())
}

#[test]
fn native_elf_phase_observer_rejects_forgotten_live_mappings() -> Check<()> {
    let case = fixture_case()?;
    let host = host_oracle(&case)?;
    let image = native_image(
        &host.program,
        function_named(&case.source, "artifact-controls", "binary-entry")?,
    )?;
    let input = b"independent observer must detect untracked surviving pages";
    verify(
        "forgotten-live-mapping",
        &image,
        input,
        input,
        Mode::ObserverLeak,
        FOCUSED_DEADLINE,
        None,
    )
}

const LOOP_FIXTURE: &str = r#"declarations.begin
(units
  (module create phase-controls
    (external create length (visibility private) (implementation core.bytes.length)
      (parameter create input (type Bytes)) (returns I64))
    (external create eq (visibility private) (implementation core.i64.equal)
      (parameter create left (type I64)) (parameter create right (type I64)) (returns Bool))
    (function create spin (visibility private) (effect pure)
      (parameter create input (type Bytes)) (returns (record (diagnostic I64) (image Bytes)))
      (body (call spin (local input))))
    (function create entry (visibility public) (effect pure)
      (parameter create input (type Bytes)) (returns (record (diagnostic I64) (image Bytes)))
      (body (if (call eq (call length (local input)) (i64 0))
        (call spin (local input))
        (record structural (field diagnostic (i64 0)) (field image (local input))))))
    (component create binary (visibility public)
      (port create run (type (function (Bytes) (record (diagnostic I64) (image Bytes)))) (function entry))))
  (target create binary-main (component phase-controls::binary) (runner command)
    (port phase-controls::binary::run)))
declarations.end"#;

#[test]
fn native_elf_phase_cancels_allocation_free_tail_loop_and_retries() -> Check<()> {
    let source = super::byte_buffer_tests::author_only(LOOP_FIXTURE)?;
    let case = super::case_from_snapshot(&source)?;
    let host = host_oracle(&case)?;
    let native = native_image(
        &host.program,
        function_named(&case.source, "phase-controls", "entry")?,
    )?;
    let generated = run_filter(
        "native-phase/source-loop",
        bootstrap_compiler()?,
        &case.request("binary-main", SourceOutputMode::Elf)?,
        0,
        FOCUSED_DEADLINE,
    )?;
    let success = b"resumed";
    for (label, image) in [
        ("Rust-allocation-free", &native),
        ("source-allocation-free", &generated),
    ] {
        verify(
            label,
            image,
            &[],
            success,
            Mode::AllocationFree,
            Duration::from_secs(10),
            Some((success, success)),
        )?;
    }
    Ok(())
}

#[test]
fn native_elf_phase_cancels_allocation_free_tail_invokes_and_retries() -> Check<()> {
    // Authored source mutations exercise both ordinary and zero-arity graph
    // transfer; FunctionValue has a static empty prefix in both cases.
    let one_argument = LOOP_FIXTURE.replace(
        "(call spin (local input))",
        "(invoke (function-value spin) (local input))",
    );
    let zero_arguments = LOOP_FIXTURE
        .replace(
            "spin (visibility private) (effect pure)\n      (parameter create input (type Bytes))",
            "spin (visibility private) (effect pure)",
        )
        .replace(
            "(call spin (local input))",
            "(invoke (function-value spin))",
        );
    for (label, source_text) in [
        ("Rust-indirect-allocation-free", one_argument),
        ("Rust-zero-arity-indirect-allocation-free", zero_arguments),
    ] {
        ensure(
            source_text.matches("(invoke (function-value spin)").count() == 2,
            "indirect-loop fixture did not replace both source calls",
        )?;
        let source = super::byte_buffer_tests::author_only(&source_text)?;
        let case = super::case_from_snapshot(&source)?;
        let host = host_oracle(&case)?;
        let spin = host
            .program
            .function(function_named(&case.source, "phase-controls", "spin")?)
            .ok_or("indirect-loop function absent")?;
        if label.contains("zero-arity") {
            let spin = &host.program.functions[spin.0 as usize];
            ensure(
                spin.parameter_count == 0,
                "zero-arity loop retained a parameter",
            )?;
            ensure(
                matches!(
                    &spin.body,
                    crate::platform::execution::normalized::prepare::NormalizedFunctionBody::Code(code)
                        if code.local_count == 0
                ),
                "zero-arity loop unexpectedly retains local slots",
            )?;
        }
        let native = native_image(
            &host.program,
            function_named(&case.source, "phase-controls", "entry")?,
        )?;
        let success = b"resumed";
        verify(
            label,
            &native,
            &[],
            success,
            Mode::AllocationFree,
            Duration::from_secs(10),
            Some((success, success)),
        )?;
    }
    Ok(())
}

const TASK_FIXTURE: &str = r#"declarations.begin
(units (module create phase-task
  (function create entry (visibility public) (effect (task))
    (parameter create input (type Bytes)) (returns (record (diagnostic I64) (image Bytes)))
    (body (record structural (field diagnostic (i64 0)) (field image (local input)))))))
declarations.end"#;

#[test]
fn native_elf_phase_does_not_export_task_empty_filter_as_pure() -> Check<()> {
    let source = super::byte_buffer_tests::author_only(TASK_FIXTURE)?;
    let case = super::case_from_snapshot(&source)?;
    let host = host_oracle(&case)?;
    let entry = function_named(&case.source, "phase-task", "entry")?;
    let index = host
        .program
        .function(entry)
        .ok_or("task fixture entry absent")?;
    let index = usize::try_from(index.0).map_err(|_| "task fixture function index extent")?;
    let function = host
        .program
        .functions
        .get(index)
        .ok_or("task fixture function descriptor absent")?;
    ensure(
        !matches!(
            function.effect,
            crate::platform::kernel::FunctionEffect::Pure
        ),
        "task fixture lost task identity",
    )?;
    ensure(
        function.task_requirements.is_empty(),
        "task exclusion fixture must have empty requirements",
    )?;
    let image = native_image(&host.program, entry)?;
    ensure(
        image.get(256..264) != Some(b"LKJPH001".as_slice()),
        "empty task filter exposed pure recovery ABI",
    )?;
    let input = b"one-shot task behavior remains available";
    equal_bytes(
        &run_filter(
            "native-phase/task-one-shot",
            &image,
            input,
            0,
            FOCUSED_DEADLINE,
        )?,
        input,
        "task descriptor exclusion preserves standalone filter behavior",
    )
}

const REJECTION_FIXTURE: &str = r#"declarations.begin
(units
  (module create phase-reject
    (external create length (visibility private) (implementation core.bytes.length)
      (parameter create input (type Bytes)) (returns I64))
    (external create eq (visibility private) (implementation core.i64.equal)
      (parameter create left (type I64)) (parameter create right (type I64)) (returns Bool))
    (external create get (visibility private) (implementation core.bytes.get)
      (parameter create input (type Bytes)) (parameter create index (type I64)) (returns I64))
    (function create entry (visibility public) (effect pure)
      (parameter create input (type Bytes)) (returns (record (diagnostic I64) (image Bytes)))
      (body (if (call eq (call length (local input)) (i64 0))
        (record structural (field diagnostic (call get (local input) (i64 -1))) (field image (local input)))
        (record structural (field diagnostic (i64 0)) (field image (local input))))))
    (component create binary (visibility public)
      (port create run (type (function (Bytes) (record (diagnostic I64) (image Bytes)))) (function entry))))
  (target create binary-main (component phase-reject::binary) (runner command)
    (port phase-reject::binary::run)))
declarations.end"#;

#[test]
fn native_elf_phase_recovers_runtime_trap_and_retries_in_both_backends() -> Check<()> {
    let source = super::byte_buffer_tests::author_only(REJECTION_FIXTURE)?;
    let case = super::case_from_snapshot(&source)?;
    let host = host_oracle(&case)?;
    let entry = function_named(&case.source, "phase-reject", "entry")?;
    let success = b"runtime trap recovery";
    for input in [&[][..], success.as_slice()] {
        let policy = super::NormalizedRunPolicy::foreground();
        let control = super::ExecutionControl::uncancelled();
        let arguments = vec![super::NormalizedValue::bytes(input)];
        let vm = super::NormalizedVm::for_test(&host.program, policy).invoke(
            entry,
            arguments.clone(),
            None,
            &control,
        );
        let reference =
            super::NormalizedReferenceInterpreter::new(&case.source, &host.program, policy)
                .invoke(entry, arguments, None, &control);
        if input.is_empty() {
            ensure(
                vm.is_err() && reference.is_err(),
                "independent evaluator accepted negative bytes index",
            )?;
        } else {
            let vm = vm.map_err(|e| format!("runtime retry VM: {e:?}"))?;
            let reference = reference.map_err(|e| format!("runtime retry reference: {e:?}"))?;
            equal(
                super::filter_value(&vm.0)?,
                (0, input),
                "runtime retry VM output",
            )?;
            equal(
                super::filter_value(&reference.0)?,
                (0, input),
                "runtime retry reference output",
            )?;
        }
    }
    let native = native_image(&host.program, entry)?;
    let generated = run_filter(
        "native-phase/source-reject",
        bootstrap_compiler()?,
        &case.request("binary-main", SourceOutputMode::Elf)?,
        0,
        FOCUSED_DEADLINE,
    )?;
    for (label, image) in [
        ("Rust-runtime-reject", &native),
        ("source-runtime-reject", &generated),
    ] {
        verify(
            label,
            image,
            &[],
            success,
            Mode::RuntimeReject,
            FOCUSED_DEADLINE,
            Some((success, success)),
        )?;
    }
    Ok(())
}

pub(super) fn verify_compiler(
    label: &str,
    image: &[u8],
    request: &[u8],
    expected: &[u8],
    elf_request: &[u8],
    elf_expected: &[u8],
) -> Check<()> {
    // Same complete authoritative request and independent oracle already bound
    // by maintained_fixed_point. One process emits artifact then ELF, rejects,
    // cancels, encounters actual mmap refusal, and emits another artifact while
    // the first artifact survives every invocation and recovery path.
    verify_compiler_with_address_space(
        label,
        image,
        request,
        expected,
        elf_request,
        elf_expected,
        Some(TEST_ADDRESS_SPACE),
    )
}

pub(super) fn verify_compiler_with_address_space(
    label: &str,
    image: &[u8],
    request: &[u8],
    expected: &[u8],
    elf_request: &[u8],
    elf_expected: &[u8],
    address_space: Option<u64>,
) -> Check<()> {
    verify_observed(
        label,
        image,
        request,
        expected,
        Mode::Compiler,
        PhasePolicy {
            address_space,
            deadline: WHOLE_SOURCE_DEADLINE,
        },
        Some((elf_request, elf_expected)),
    )
    .map(|_| ())
}

#[test]
fn native_elf_phase_address_space_policy_is_exact() -> Check<()> {
    equal(
        address_space_argument(None)?,
        "unlimited".to_owned(),
        "unbounded OS allowance",
    )?;
    for (bytes, text) in [
        (0, "0"),
        (1, "1"),
        (TEST_ADDRESS_SPACE, "34359738368"),
        (u64::MAX - 1, "18446744073709551614"),
    ] {
        equal(
            address_space_argument(Some(bytes))?,
            text.to_owned(),
            "finite OS allowance",
        )?;
        equal(
            parse_address_space_limits(&format!("Max address space {text} unlimited bytes\n"))?,
            AddressSpaceLimits {
                soft: Some(bytes),
                hard: None,
            },
            "finite soft allowance and distinct unlimited hard allowance",
        )?;
    }
    ensure(
        address_space_argument(Some(u64::MAX)).is_err(),
        "finite MAX became OS infinity",
    )?;
    equal(
        parse_address_space_limits("Max address space unlimited unlimited bytes\n")?,
        AddressSpaceLimits {
            soft: None,
            hard: None,
        },
        "explicit unlimited kernel limits",
    )?;
    for text in [
        "",
        "Max address space unlimited bytes\n",
        "Max address space 1 2 kB\n",
        "Max address space 1 2 bytes trailing\n",
        "Max address space 18446744073709551615 unlimited bytes\n",
        "Max address space 1 unlimited bytes\nMax address space 2 unlimited bytes\n",
    ] {
        ensure(
            parse_address_space_limits(text).is_err(),
            "invalid kernel limit row admitted",
        )?;
    }
    Ok(())
}

#[test]
fn native_elf_phase_compiler_recovers_with_unbounded_address_space() -> Check<()> {
    // The in-work cancellation oracle waits for 8 MiB of live scratch. Give it
    // observable compilation work while preserving the independent echo result.
    let body = "(body (record structural (field diagnostic (i64 0)) (field image (local input))))";
    ensure(
        super::FIXTURE.matches(body).count() == 1,
        "compiler phase fixture must replace exactly the binary-entry body",
    )?;
    let expanded = format!(
        "(body (sequence {}(record structural (field diagnostic (i64 0)) (field image (local input)))))",
        "(i64 0) ".repeat(4096),
    );
    let fixture = super::FIXTURE.replacen(body, &expanded, 1);
    let source = super::byte_buffer_tests::author_only(&fixture)?;
    let case = super::case_from_snapshot(&source)?;
    let compiler = bootstrap_compiler()?;
    let host = host_oracle(&case)?;
    let request = case.request("binary-main", SourceOutputMode::CanonicalArtifact)?;
    let elf_request = case.request("binary-main", SourceOutputMode::Elf)?;
    let elf = run_filter(
        "native-phase/unbounded-compiler-elf",
        compiler,
        &elf_request,
        0,
        FOCUSED_DEADLINE,
    )?;
    let input = b"compiled phase fixture echoes its independent input";
    equal_bytes(
        &run_filter(
            "native-phase/unbounded-compiler-behavior",
            &elf,
            input,
            0,
            FOCUSED_DEADLINE,
        )?,
        input,
        "unbounded compiler fixture behavior",
    )?;
    verify_compiler_with_address_space(
        "compiler-unbounded",
        compiler,
        &request,
        &host.bytes,
        &elf_request,
        &elf,
        None,
    )
}
