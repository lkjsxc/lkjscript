//! Output-only independent oracles for complete raw-source summaries and canonical units.
//! Nest this module under selfhost_source_scalar_tests. The source compiler receives
//! only current LKJSRQ01 profile 2/mode 1 + the complete LKJSG002 profile 3
//! carrier. The positive seed retains its original published RevisionCore;
//! deliberate snapshot mutations use the shared raw fixture owner. Independent
//! comparison witnesses and units are built after the source-driven outputs and
//! are never appended to a request.
use super::*;
use crate::platform::change::WitnessBaseRead;
use crate::platform::compiler::selfhost_source_input::{
    self as source_input, ExportedSource, RawRevisionProvenance, SourceOutputMode,
};
use crate::platform::compiler::unit::{CompiledFieldSelector, CompiledSignature, CompiledText};
use crate::platform::compiler::{
    CompilationPayload, CompilationReceipt, CompilationUnit, CompilationUnitKey,
    CompiledInstruction as I, OptimizationPolicy, compile_unit,
};
use crate::platform::package_transport::source::PackageContainer;
use crate::platform::storage::object::{ObjectDomain, ObjectKey};
use crate::platform::witness::OwnerSummary;
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

type Check<T> = Result<T, String>;
type SummaryDimensions = [[u8; 32]; 7];

const UNIT_SOURCE: &str = include_str!("../../../../tests/fixtures/selfhost-source-units.lkjc");
const SUMMARY_FIELDS: [&str; 7] = [
    "interface",
    "implementation",
    "type",
    "effect",
    "capability",
    "relations",
    "validation-dependencies",
];

struct PublishedUnitFixture {
    source: KernelSnapshot,
    carrier: Vec<u8>,
}

static UNIT_FIXTURE: OnceLock<Check<PublishedUnitFixture>> = OnceLock::new();

fn published_fixture() -> Check<&'static PublishedUnitFixture> {
    UNIT_FIXTURE
        .get_or_init(|| {
            // Retain the same public authoring sequence used by author_only, so the
            // positive seed can export its actual history instead of recreating it
            // from a detached snapshot. No compiled unit or witness is a request input.
            let empty = empty_normalized_snapshot(b"owned-byte-buffer");
            let memory = crate::platform::MemProjectStore::new(None)
                .map_err(|error| format!("unit fixture store: {error:?}"))?;
            let repository = GraphRepository::create_memory(&memory, &empty, None)
                .map_err(|error| format!("unit fixture repository: {error:?}"))?
                .repository;
            let base = repository
                .view_current()
                .map_err(|error| format!("unit fixture base: {error:?}"))?
                .revision();
            let input = format!("request base={base}\n{UNIT_SOURCE}");
            let decoded =
                crate::platform::control::decode_compact_change("buffer", input.as_bytes())
                    .map_err(|error| format!("unit fixture proposal: {error:?}"))?;
            let plan = repository
                .prepare_authored_change(&decoded.semantic, decoded.options)
                .map_err(|error| format!("unit fixture plan: {error:?}"))?;
            repository
                .publish(&plan.publication)
                .map_err(|error| format!("unit fixture publication: {error:?}"))?;
            let view = repository
                .view_current()
                .map_err(|error| format!("unit fixture accepted view: {error:?}"))?;
            let source = view
                .reconstruct_full_oracle()
                .map_err(|error| format!("unit fixture readback: {error:?}"))?
                .value;
            let public_export = view
                .export_package_transport()
                .map_err(|error| format!("unit fixture export: {error:?}"))?;
            let container =
                PackageContainer::decode(&public_export.container, public_export.transport_digest)
                    .map_err(|error| format!("unit fixture container: {error:?}"))?;
            let exported = ExportedSource::from_container(&container)
                .map_err(|error| format!("unit fixture original source: {error:?}"))?;
            let raw = exported.as_raw();
            assert_eq!(raw.provenance.claimed_revision, view.revision().bytes());
            assert_eq!(raw.package, source.root.package_id.bytes());
            let semantic_state = kernel::semantic_state_digest(&source)
                .map_err(|error| format!("unit fixture state: {error:?}"))?;
            assert_eq!(raw.semantic_state, semantic_state.bytes());
            assert_eq!(raw.owners.len(), source.owners.len());
            assert_eq!(raw.types.len(), source.types.len());
            assert_eq!(raw.retirements.len(), source.retirements.len());
            let carrier = source_input::carrier(&raw)
                .map_err(|error| format!("unit fixture carrier: {error:?}"))?;
            let provenance = carrier_provenance(&carrier)?;
            assert_eq!(provenance.claimed_revision, raw.provenance.claimed_revision);
            assert_eq!(provenance.canonical_core, raw.provenance.canonical_core);
            eprintln!(
                "source unit seed revision={} semantic_state={} raw_core_sha256={} \
            export_sha256={} carrier_sha256={} provenance=exact-public-export",
                view.revision(),
                semantic_state,
                sha256(raw.provenance.canonical_core),
                sha256(&public_export.container),
                sha256(&carrier)
            );
            Ok(PublishedUnitFixture { source, carrier })
        })
        .as_ref()
        .map_err(Clone::clone)
}

fn fixture_units() -> Check<&'static KernelSnapshot> {
    Ok(&published_fixture()?.source)
}

fn unit_carrier(source: &KernelSnapshot) -> Vec<u8> {
    if let Some(Ok(fixture)) = UNIT_FIXTURE.get()
        && std::ptr::eq(source, &fixture.source)
    {
        return fixture.carrier.clone();
    }
    // Shared fixture-only history for deliberately changed snapshots. The exact
    // maintained compiler pointer instead selects the shared public-export path.
    graph::packet(source)
}

fn unit_request(carrier: &[u8], target: TargetId) -> Check<Vec<u8>> {
    source_input::request(carrier, target, SourceOutputMode::CanonicalArtifact)
        .map_err(|error| format!("canonical unit request: {error:?}"))
}

fn carrier_word(carrier: &[u8], at: usize) -> Check<usize> {
    let end = at
        .checked_add(4)
        .ok_or_else(|| "carrier word extent".to_owned())?;
    let bytes: [u8; 4] = carrier
        .get(at..end)
        .ok_or_else(|| "carrier header".to_owned())?
        .try_into()
        .map_err(|_| "carrier word".to_owned())?;
    usize::try_from(u32::from_le_bytes(bytes)).map_err(|_| "carrier word representation".to_owned())
}

fn carrier_provenance(carrier: &[u8]) -> Check<RawRevisionProvenance<'_>> {
    if carrier_word(carrier, 112)? != 3 {
        return Err("unit input is not source profile 3".to_owned());
    }
    let start = carrier_word(carrier, 116)?;
    let length = carrier_word(carrier, 120)?;
    let name_end = 128_usize
        .checked_add(carrier_word(carrier, 24)?)
        .ok_or_else(|| "carrier name extent".to_owned())?;
    if start != name_end || length < 32 {
        return Err("carrier provenance framing".to_owned());
    }
    let core_at = start
        .checked_add(32)
        .ok_or_else(|| "carrier revision extent".to_owned())?;
    let end = start
        .checked_add(length)
        .ok_or_else(|| "carrier provenance extent".to_owned())?;
    if carrier_word(carrier, 80)? != end {
        return Err("carrier provenance directory binding".to_owned());
    }
    let claim = carrier
        .get(start..core_at)
        .ok_or_else(|| "carrier revision missing".to_owned())?;
    let canonical_core = carrier
        .get(core_at..end)
        .ok_or_else(|| "carrier core missing".to_owned())?;
    let claimed_revision = claim
        .try_into()
        .map_err(|_| "carrier revision width".to_owned())?;
    Ok(RawRevisionProvenance {
        claimed_revision,
        canonical_core,
    })
}

fn unit_producer(summaries: bool) -> &'static NativeCompiler {
    static SUMMARIES: OnceLock<NativeCompiler> = OnceLock::new();
    static UNITS: OnceLock<NativeCompiler> = OnceLock::new();
    if summaries {
        SUMMARIES.get_or_init(|| {
            build_native_compiler(
                "source-unit",
                "inspect-summaries",
                "source-unit::inspect-summaries",
            )
        })
    } else {
        UNITS
            .get_or_init(|| build_native_compiler("source-unit", "compile", "source-unit::compile"))
    }
}

struct JoinedChild(Child);
impl Drop for JoinedChild {
    fn drop(&mut self) {
        // A test error or unwind cannot leave a compiler running against its temporary input.
        // wait also succeeds after try_wait has already reaped the child.
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn native_unit_output(
    summaries: bool,
    name: &str,
    request: &[u8],
    diagnostic: i64,
    deadline: Duration,
) -> Check<Vec<u8>> {
    let producer = unit_producer(summaries);
    crate::release_container::inspect_static_elf_bytes(&producer.image)
        .map_err(|error| format!("{name}: bootstrap ELF: {error:?}"))?;
    let directory = tempfile::tempdir().map_err(|error| format!("{name}: tempdir: {error}"))?;
    let executable = directory.path().join("source-unit-compiler");
    let input = directory.path().join("request.bin");
    let stdout_path = directory.path().join("output.bin");
    let stderr_path = directory.path().join("diagnostic.txt");
    crate::platform::owned_output::publish_executable_create_new(
        &executable,
        &producer.image,
        None,
        "source unit compiler test",
    )
    .map_err(|error| format!("{name}: publish bootstrap: {error:?}"))?;
    fs::write(&input, request).map_err(|error| format!("{name}: input: {error}"))?;
    let stdin = File::open(&input).map_err(|error| format!("{name}: stdin: {error}"))?;
    let stdout = File::create(&stdout_path).map_err(|error| format!("{name}: stdout: {error}"))?;
    let stderr = File::create(&stderr_path).map_err(|error| format!("{name}: stderr: {error}"))?;
    let (carrier_sha256, carrier_bytes) = match request.get(32..) {
        Some(carrier) => (sha256(carrier), carrier.len()),
        None => ("absent".to_owned(), 0),
    };
    eprintln!(
        "source unit request case={name} entry={} bootstrap=rust_elf compiler_carrier_sha256={} \
         executable_sha256={} carrier_sha256={} carrier_bytes={} request_sha256={} request_bytes={} \
         target=linux-x86_64 arguments=[] environment=empty stdin=regular-file \
         host_deadline_seconds={} capture_byte_quota=none",
        producer.entry,
        producer.source_carrier_sha256,
        producer.executable_sha256,
        carrier_sha256,
        carrier_bytes,
        sha256(request),
        request.len(),
        deadline.as_secs(),
    );
    let start = Instant::now();
    let mut child = JoinedChild(
        Command::new(&executable)
            .env_clear()
            .current_dir(directory.path())
            .stdin(Stdio::from(stdin))
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .spawn()
            .map_err(|error| format!("{name}: spawn: {error}"))?,
    );
    let status = loop {
        if let Some(status) = child
            .0
            .try_wait()
            .map_err(|error| format!("{name}: poll compiler: {error}"))?
        {
            break status;
        }
        if start.elapsed() >= deadline {
            // JoinedChild's Drop kills and joins before any temporary file is removed.
            return Err(format!(
                "{name}: {} exceeded caller-selected test deadline {deadline:?}",
                producer.entry
            ));
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    // Files allow simultaneous producer progress without a capture-size admission limit.
    // They are read only after the exact child has terminated and been joined.
    let output = fs::read(&stdout_path).map_err(|error| format!("{name}: output: {error}"))?;
    let errors = fs::read(&stderr_path).map_err(|error| format!("{name}: diagnostics: {error}"))?;
    eprintln!(
        "source unit response case={name} entry={} exit={:?} output_bytes={} output_sha256={} \
         stderr_sha256={} elapsed_ms={}",
        producer.entry,
        status.code(),
        output.len(),
        sha256(&output),
        sha256(&errors),
        start.elapsed().as_millis(),
    );
    let expected_exit = if diagnostic == 0 { 0 } else { 65 };
    if status.code() != Some(expected_exit) {
        return Err(format!(
            "{name}: {} exit={:?}, stderr={}",
            producer.entry,
            status.code(),
            String::from_utf8_lossy(&errors)
        ));
    }
    if diagnostic == 0 {
        if !errors.is_empty() {
            return Err(format!("{name}: successful compiler wrote diagnostics"));
        }
    } else if !output.is_empty() || errors != format!("compiler_rejected:{diagnostic}\n").as_bytes()
    {
        return Err(format!(
            "{name}: expected empty output and diagnostic {diagnostic}; output_bytes={} stderr={}",
            output.len(),
            String::from_utf8_lossy(&errors)
        ));
    }
    Ok(output)
}

struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Cursor<'a> {
    fn take(&mut self, count: usize) -> Check<&'a [u8]> {
        let end = self
            .at
            .checked_add(count)
            .ok_or_else(|| "output extent overflow".to_owned())?;
        let bytes = self
            .bytes
            .get(self.at..end)
            .ok_or_else(|| format!("truncated output at {} + {count}", self.at))?;
        self.at = end;
        Ok(bytes)
    }
    fn word(&mut self) -> Check<u64> {
        let bytes: [u8; 8] = self
            .take(8)?
            .try_into()
            .map_err(|_| "u64 width".to_owned())?;
        Ok(u64::from_le_bytes(bytes))
    }
    fn size(&mut self) -> Check<usize> {
        usize::try_from(self.word()?).map_err(|_| "output length exceeds usize".to_owned())
    }
    fn digest(&mut self) -> Check<[u8; 32]> {
        self.take(32)?
            .try_into()
            .map_err(|_| "digest width".to_owned())
    }
    fn finish(&self) -> Check<()> {
        if self.at != self.bytes.len() {
            return Err("trailing output bytes".to_owned());
        }
        Ok(())
    }
}

fn carrier_owners(source: &KernelSnapshot, carrier: &[u8]) -> Check<Vec<OwnerKey>> {
    let count = carrier_word(carrier, 12)?;
    let directory = carrier_word(carrier, 80)?;
    if count != source.owners.len() {
        return Err("fixture omitted source owners".to_owned());
    }
    if carrier_word(carrier, 16)? != source.types.len() {
        return Err("fixture omitted supplied types".to_owned());
    }
    let owners = source.owners.keys().copied().collect::<Vec<_>>();
    for (row, owner) in owners.iter().enumerate() {
        let offset = row
            .checked_mul(50)
            .and_then(|n| directory.checked_add(n))
            .ok_or_else(|| "carrier owner directory extent".to_owned())?;
        let end = offset
            .checked_add(17)
            .ok_or_else(|| "carrier owner key extent".to_owned())?;
        if carrier.get(offset..end) != Some(kernel::owner_map_key(*owner).as_slice()) {
            return Err(format!(
                "carrier source order differs at owner {row}: {owner:?}"
            ));
        }
    }
    Ok(owners)
}

fn dimensions(summary: &OwnerSummary) -> SummaryDimensions {
    [
        summary.semantic_interface.bytes(),
        summary.implementation.bytes(),
        summary.type_digest.bytes(),
        summary.effect.bytes(),
        summary.capability.bytes(),
        summary.relations.bytes(),
        summary.validation_dependencies.bytes(),
    ]
}

fn compilation_owner(record: &OwnerRecord) -> bool {
    matches!(
        record.kind(),
        kernel::OwnerKind::External
            | kernel::OwnerKind::PureFunction
            | kernel::OwnerKind::Component
            | kernel::OwnerKind::Target
    )
}

struct HostOracle {
    summaries: BTreeMap<OwnerKey, SummaryDimensions>,
    units: BTreeMap<OwnerKey, CompilationReceipt>,
}

fn host_oracle(source: &KernelSnapshot) -> Check<HostOracle> {
    // OUTPUT-ONLY oracle. This function is called only after both lkjscript outputs.
    let memory = crate::platform::MemProjectStore::new(None)
        .map_err(|error| format!("oracle store: {error:?}"))?;
    let repository = GraphRepository::create_memory(&memory, source, None)
        .map_err(|error| format!("oracle public admission: {error:?}"))?
        .repository;
    let view = repository
        .view_current()
        .map_err(|error| format!("oracle view: {error:?}"))?;
    let readback = view
        .reconstruct_full_oracle()
        .map_err(|error| format!("oracle readback: {error:?}"))?
        .value;
    if readback.owners != source.owners {
        return Err("oracle adoption changed an authored owner".to_owned());
    }
    let mut summaries = BTreeMap::new();
    let mut units = BTreeMap::new();
    for (owner, record) in &source.owners {
        let bound = view
            .read_owner_summary(*owner)
            .map_err(|error| format!("oracle summary {owner:?}: {error:?}"))?
            .value
            .ok_or_else(|| format!("missing oracle summary {owner:?}"))?;
        if bound.summary.owner != *owner || bound.summary.kind != record.kind() {
            return Err(format!("oracle summary binding changed {owner:?}"));
        }
        if bound.summary.test.is_some() {
            return Err(format!(
                "active unit fixture unexpectedly includes test summary {owner:?}"
            ));
        }
        summaries.insert(*owner, dimensions(&bound.summary));
        if compilation_owner(record) {
            let receipt = compile_unit(
                &view,
                &view,
                *owner,
                OptimizationPolicy::DeterministicBaseline,
            )
            .map_err(|error| format!("oracle unit {owner:?}: {error:?}"))?;
            units.insert(*owner, receipt);
        }
    }
    Ok(HostOracle { summaries, units })
}

#[derive(Debug)]
struct UnitImage {
    unit: CompilationUnit,
    object: ObjectKey,
    image: Vec<u8>,
}
struct UnitOutputs {
    summaries: BTreeMap<OwnerKey, SummaryDimensions>,
    units: BTreeMap<OwnerKey, UnitImage>,
    summary_bytes: Vec<u8>,
    unit_bytes: Vec<u8>,
}

fn compare_summaries(
    bytes: &[u8],
    owners: &[OwnerKey],
    oracle: &HostOracle,
) -> Check<BTreeMap<OwnerKey, SummaryDimensions>> {
    let mut cursor = Cursor { bytes, at: 0 };
    if cursor.take(8)? != b"LKJSUS01" {
        return Err("summary output magic".to_owned());
    }
    let count = cursor.size()?;
    if count != owners.len() {
        return Err(format!("summary owner count {count} != {}", owners.len()));
    }
    let expected_bytes = count
        .checked_mul(224)
        .and_then(|n| n.checked_add(16))
        .ok_or_else(|| "summary output extent overflow".to_owned())?;
    if bytes.len() != expected_bytes {
        return Err("summary output has missing or extra rows".to_owned());
    }
    let mut actual = BTreeMap::new();
    for owner in owners {
        let expected = oracle
            .summaries
            .get(owner)
            .ok_or_else(|| format!("missing summary oracle {owner:?}"))?;
        let mut values = [[0; 32]; 7];
        for dimension in 0..7 {
            values[dimension] = cursor.digest()?;
            if values[dimension] != expected[dimension] {
                return Err(format!(
                    "summary {owner:?} dimension={} expected={} actual={}",
                    SUMMARY_FIELDS[dimension],
                    crate::platform::semantic_id::encode_hex(&expected[dimension]),
                    crate::platform::semantic_id::encode_hex(&values[dimension])
                ));
            }
        }
        actual.insert(*owner, values);
    }
    cursor.finish()?;
    Ok(actual)
}

fn compare_units(
    bytes: &[u8],
    source: &KernelSnapshot,
    owners: &[OwnerKey],
    oracle: &HostOracle,
) -> Check<BTreeMap<OwnerKey, UnitImage>> {
    let mut cursor = Cursor { bytes, at: 0 };
    if cursor.take(8)? != b"LKJSUN01" {
        return Err("unit output magic".to_owned());
    }
    let count = cursor.size()?;
    let selected = owners
        .iter()
        .enumerate()
        .filter(|(_, owner)| source.owners.get(*owner).is_some_and(compilation_owner))
        .collect::<Vec<_>>();
    if count != selected.len() || count != oracle.units.len() {
        return Err(format!(
            "unit inventory count {count} differs from complete source {} / oracle {}",
            selected.len(),
            oracle.units.len()
        ));
    }
    let mut actual = BTreeMap::new();
    for (row, owner) in selected {
        let output_row = cursor.size()?;
        if output_row != row {
            return Err(format!("unit source row {output_row} != {row} ({owner:?})"));
        }
        let key = CompilationUnitKey::from_bytes(cursor.digest()?);
        let object = ObjectKey::from_digest(ObjectDomain::CompilerUnit, cursor.digest()?);
        let image_length = cursor.size()?;
        let image = cursor.take(image_length)?;
        let unit = CompilationUnit::decode(image, object)
            .map_err(|error| format!("strict unit {owner:?}: {error:?}"))?;
        let expected = oracle
            .units
            .get(owner)
            .ok_or_else(|| format!("missing unit oracle {owner:?}"))?;
        if key != unit.key
            || key != expected.key
            || object != expected.object
            || unit.source.owner != *owner
            || unit.source.package != source.root.package_id
        {
            return Err(format!(
                "unit identity/key/object binding differs for {owner:?}"
            ));
        }
        if image != expected.bytes.as_slice() {
            let first = image.iter().zip(&expected.bytes).position(|(a, b)| a != b);
            return Err(format!(
                "unit bytes differ for {owner:?}: first={first:?} actual_bytes={} expected_bytes={} actual_sha256={} expected_sha256={}",
                image.len(),
                expected.bytes.len(),
                sha256(image),
                sha256(&expected.bytes)
            ));
        }
        if unit != expected.unit {
            return Err(format!("decoded canonical unit differs for {owner:?}"));
        }
        if actual
            .insert(
                *owner,
                UnitImage {
                    unit,
                    object,
                    image: image.to_vec(),
                },
            )
            .is_some()
        {
            return Err(format!("duplicate emitted unit owner {owner:?}"));
        }
    }
    cursor.finish()?;
    Ok(actual)
}

fn compare_language_engines(name: &str, request: &[u8], expected: &[u8]) -> Check<()> {
    // Existing helper runs the VM and independent canonical reference interpreter.
    let value = super::super::invoke("source-unit", name, vec![NormalizedValue::bytes(request)]);
    if super::super::field(&value, "diagnostic") != &NormalizedValue::I64(0) {
        return Err(format!("language engines rejected source-unit::{name}"));
    }
    let NormalizedValue::Bytes(bytes) = super::super::field(&value, "image") else {
        return Err(format!("source-unit::{name} did not return Bytes"));
    };
    if bytes.as_ref() != expected {
        return Err(format!(
            "native and language-engine output differs for source-unit::{name}"
        ));
    }
    Ok(())
}

fn verify_carrier_units(
    name: &str,
    source: &KernelSnapshot,
    target: TargetId,
    carrier: &[u8],
    oracle_source: &KernelSnapshot,
    engines: bool,
    deadline: Duration,
) -> Check<UnitOutputs> {
    let owners = carrier_owners(source, carrier)?;
    let provenance = carrier_provenance(carrier)?;
    let request = unit_request(carrier, target)?;
    let summary_bytes = native_unit_output(true, name, &request, 0, deadline)?;
    let unit_bytes = native_unit_output(false, name, &request, 0, deadline)?;
    if engines {
        compare_language_engines("inspect-summaries", &request, &summary_bytes)?;
        compare_language_engines("compile", &request, &unit_bytes)?;
    }
    // A raw carrier may contain additional supplied TypeRecords that a public
    // repository disallows as unreachable. The explicit independent oracle can
    // omit only such extra descriptors, never owners or live type definitions.
    // Both source-driven outputs above consumed the original complete carrier.
    assert_eq!(source.owners, oracle_source.owners);
    assert_eq!(source.root, oracle_source.root);
    assert_eq!(source.retirements, oracle_source.retirements);
    assert!(
        oracle_source
            .types
            .iter()
            .all(|(id, ty)| source.types.get(id) == Some(ty))
    );
    let oracle = host_oracle(oracle_source)?;
    let summaries = compare_summaries(&summary_bytes, &owners, &oracle)?;
    let units = compare_units(&unit_bytes, source, &owners, &oracle)?;
    eprintln!(
        "source unit oracle case={name} owners={} types={} compilation_units={} summary_dimensions={} \
        carrier_sha256={} raw_core_sha256={} request_sha256={} summary_sha256={} unit_inventory_sha256={} \
        source_profile=3 request_profile=2 output_mode=1 oracle=public-memory-repository+compile_unit output_only=true",
        owners.len(),
        source.types.len(),
        units.len(),
        owners.len() * 7,
        sha256(carrier),
        sha256(provenance.canonical_core),
        sha256(&request),
        sha256(&summary_bytes),
        sha256(&unit_bytes)
    );
    Ok(UnitOutputs {
        summaries,
        units,
        summary_bytes,
        unit_bytes,
    })
}

fn verify_units(
    name: &str,
    source: &KernelSnapshot,
    target: TargetId,
    engines: bool,
    deadline: Duration,
) -> Check<UnitOutputs> {
    verify_carrier_units(
        name,
        source,
        target,
        &unit_carrier(source),
        source,
        engines,
        deadline,
    )
}

fn baseline_units() -> Check<&'static UnitOutputs> {
    static OUTPUT: OnceLock<Check<UnitOutputs>> = OnceLock::new();
    OUTPUT
        .get_or_init(|| {
            let source = fixture_units()?;
            verify_units(
                "unit-baseline",
                source,
                target_id(source, "scalar-main"),
                true,
                Duration::from_secs(60),
            )
        })
        .as_ref()
        .map_err(Clone::clone)
}

fn unit_named<'a>(
    output: &'a UnitOutputs,
    source: &KernelSnapshot,
    name: &str,
) -> Check<&'a UnitImage> {
    let owner = OwnerKey::Declaration(declaration(source, name));
    output
        .units
        .get(&owner)
        .ok_or_else(|| format!("unit absent for {name}"))
}

fn code(image: &UnitImage) -> Check<&crate::platform::compiler::CompiledCode> {
    match &image.unit.payload {
        CompilationPayload::Function { code, .. } => Ok(code),
        _ => Err("expected function unit".to_owned()),
    }
}

fn signature(image: &UnitImage) -> Check<&CompiledSignature> {
    match &image.unit.payload {
        CompilationPayload::Function { signature, .. }
        | CompilationPayload::External { signature, .. } => Ok(signature),
        _ => Err("expected callable unit".to_owned()),
    }
}

fn external<'a>(source: &'a KernelSnapshot, name: &str) -> Check<&'a kernel::ExternalDeclaration> {
    match source
        .owners
        .get(&OwnerKey::Declaration(declaration(source, name)))
    {
        Some(OwnerRecord::Declaration(record)) => match &record.payload {
            kernel::DeclarationPayload::External(value) => Ok(value),
            _ => Err(format!("{name}: not an external")),
        },
        _ => Err(format!("{name}: missing declaration")),
    }
}

fn table_type(image: &UnitImage, index: u32) -> Check<TypeObjectDigest> {
    image
        .unit
        .tables
        .types
        .get(index as usize)
        .copied()
        .ok_or_else(|| "type relocation index".to_owned())
}

fn root_bindings(source: &KernelSnapshot, name: &str) -> Check<(Vec<BindingId>, ExpressionId)> {
    match &expression(source, fixture_function(source, name).body).operation {
        E::Let { bindings, body } => Ok((bindings.clone(), *body)),
        _ => Err(format!("{name}: expected root Let")),
    }
}

fn binding_mut(
    source: &mut KernelSnapshot,
    binding: BindingId,
) -> Check<&mut kernel::BindingRecord> {
    match source.owners.get_mut(&OwnerKey::Binding(binding)) {
        Some(OwnerRecord::Binding(record)) => Ok(record),
        _ => Err("fixture binding absent".to_owned()),
    }
}

fn captured_pair(source: &KernelSnapshot) -> Check<(Vec<BindingId>, ExpressionId)> {
    let (bindings, _) = root_bindings(source, "fields")?;
    let value = bindings
        .iter()
        .find_map(|id| match source.owners.get(&OwnerKey::Binding(*id)) {
            Some(OwnerRecord::Binding(record)) if record.name.as_str() == "pair" => record.value,
            _ => None,
        })
        .ok_or_else(|| "authored pair binding absent".to_owned())?;
    match &expression(source, value).operation {
        E::Let { bindings, body }
            if matches!(expression(source, *body).operation, E::Record { .. }) =>
        {
            assert_eq!(bindings.len(), 2);
            Ok((bindings.clone(), value))
        }
        _ => Err("record initializer was not lowered to ordered captures".to_owned()),
    }
}

fn captured_literals(source: &KernelSnapshot, bindings: &[BindingId]) -> Check<Vec<i64>> {
    bindings
        .iter()
        .map(|id| {
            let Some(OwnerRecord::Binding(record)) = source.owners.get(&OwnerKey::Binding(*id))
            else {
                return Err("record capture binding absent".to_owned());
            };
            let value = record
                .value
                .ok_or_else(|| "record capture initializer absent".to_owned())?;
            match expression(source, value).operation {
                E::I64 { value } => Ok(value),
                _ => Err("record capture is not an independent literal".to_owned()),
            }
        })
        .collect()
}

fn literal_operands(image: &UnitImage) -> Check<Vec<i64>> {
    Ok(code(image)?
        .instructions
        .iter()
        .filter_map(|instruction| match instruction {
            I::I64(value) => Some(*value),
            _ => None,
        })
        .collect())
}

#[test]
fn selfhost_source_unit_summaries_and_complete_units_match_independent_oracles() -> Check<()> {
    let source = fixture_units()?;
    let output = baseline_units()?;
    assert_eq!(output.summaries.len(), source.owners.len());
    assert_eq!(
        output.units.len(),
        source
            .owners
            .values()
            .filter(|owner| compilation_owner(owner))
            .count()
    );
    for name in [
        "literals",
        "identity",
        "text-identity",
        "static-identity",
        "annotated",
        "generic-size",
        "unused-branch",
        "even",
        "odd",
        "owned-return",
        "unused-dispose",
        "binary-entry",
    ] {
        assert!(
            output
                .units
                .contains_key(&OwnerKey::Declaration(declaration(source, name))),
            "uncalled owner {name}"
        );
    }
    assert!(
        output
            .units
            .contains_key(&OwnerKey::Target(target_id(source, "unit-binary")))
    );
    // The caller's selection authorizes an endpoint check, never an inventory filter.
    let other = verify_units(
        "unit-other-target",
        source,
        target_id(source, "unit-binary"),
        false,
        Duration::from_secs(60),
    )?;
    assert_eq!(output.summary_bytes, other.summary_bytes);
    assert_eq!(output.unit_bytes, other.unit_bytes);
    Ok(())
}

#[test]
fn selfhost_source_unit_keeps_generic_signatures_static_text_and_authored_relocation_order()
-> Check<()> {
    let source = fixture_units()?;
    let output = baseline_units()?;
    let integer = scalar_type(source, kernel::TypeForm::I64);
    for name in ["list-get", "list-size"] {
        let raw = external(source, name)?;
        let image = unit_named(output, source, name)?;
        let compiled = signature(image)?;
        assert_eq!(compiled.type_parameters, raw.type_parameters);
        assert_eq!(
            compiled.type_parameter_constraints,
            vec![kernel::TypeParameterConstraints::None]
        );
        assert_eq!(compiled.parameters.len(), raw.parameters.len());
        for (parameter, raw_id) in compiled.parameters.iter().zip(&raw.parameters) {
            let Some(OwnerRecord::Parameter(raw_parameter)) =
                source.owners.get(&OwnerKey::Parameter(*raw_id))
            else {
                return Err("generic parameter owner missing".to_owned());
            };
            assert_eq!(parameter.parameter, *raw_id);
            assert_eq!(parameter.name, raw_parameter.name);
            assert_eq!(table_type(image, parameter.ty)?, raw_parameter.ty);
            assert_eq!(parameter.use_mode, kernel::ParameterUse::Unrestricted);
            assert_eq!(parameter.resource_requirement, None);
        }
        assert_eq!(table_type(image, compiled.result.ty)?, raw.result);
        if name == "list-get" {
            assert_ne!(
                raw.result, integer,
                "generic result must retain T, not I64 specialization"
            );
            assert!(matches!(
                source.types.get(&raw.result).map(|value| &value.form),
                Some(kernel::TypeForm::TypeParameter { .. })
            ));
        }
    }
    let generic = unit_named(output, source, "generic")?;
    let list_get = declaration_reference(source, "list-get");
    let call = code(generic)?
        .instructions
        .iter()
        .find_map(|instruction| match instruction {
            I::Call {
                function,
                type_arguments,
                ..
            } if generic.unit.tables.declarations.get(*function as usize) == Some(&list_get) => {
                Some(type_arguments)
            }
            _ => None,
        })
        .ok_or_else(|| "generic call absent".to_owned())?;
    assert_eq!(call.len(), 1);
    assert_eq!(table_type(generic, call[0])?, integer);

    let literals = unit_named(output, source, "literals")?;
    assert_eq!(
        literals.unit.tables.texts,
        vec![CompiledText::Inline("same".to_owned())]
    );
    assert_eq!(
        code(literals)?.instructions,
        vec![
            I::Text(0),
            I::Drop,
            I::StaticText(0),
            I::Drop,
            I::I64(7),
            I::Return
        ]
    );
    let text = scalar_type(source, kernel::TypeForm::Text);
    let static_text = scalar_type(source, kernel::TypeForm::StaticText);
    assert_ne!(
        text, static_text,
        "the two canonical type identities remain distinct"
    );
    for (name, ty) in [("text-identity", text), ("static-identity", static_text)] {
        let image = unit_named(output, source, name)?;
        let signature = signature(image)?;
        assert_eq!(image.unit.tables.types, vec![ty]);
        assert_eq!(table_type(image, signature.parameters[0].ty)?, ty);
        assert_eq!(table_type(image, signature.result.ty)?, ty);
    }
    let fields = unit_named(output, source, "fields")?;
    // Authoring retains the source evaluation order in generated Let captures;
    // canonical Record selectors and their relocation table are sorted by name.
    let (captures, _) = captured_pair(source)?;
    assert_eq!(captured_literals(source, &captures)?, [10, 20]);
    assert_eq!(literal_operands(fields)?, [10, 20]);
    assert_eq!(
        fields
            .unit
            .tables
            .structural_names
            .iter()
            .map(kernel::Name::as_str)
            .collect::<Vec<_>>(),
        ["alpha", "zed"]
    );
    assert!(code(fields)?.instructions.iter().any(|instruction| matches!(instruction,
        I::Record { nominal_type: None, fields, .. } if fields == &[CompiledFieldSelector::Structural(0), CompiledFieldSelector::Structural(1)])));
    for (name, other) in [("even", "odd"), ("odd", "even")] {
        let image = unit_named(output, source, name)?;
        assert!(
            image
                .unit
                .tables
                .declarations
                .contains(&declaration_reference(source, other))
        );
        assert!(
            code(image)?
                .instructions
                .iter()
                .any(|instruction| matches!(instruction, I::JumpIfFalse(_)))
        );
    }
    Ok(())
}

#[test]
fn selfhost_source_unit_owned_let_cleanup_uses_forward_unit_stores_after_transfer() -> Check<()> {
    let source = fixture_units()?;
    let output = baseline_units()?;
    let owned = code(unit_named(output, source, "owned-return")?)?;
    assert_eq!((owned.parameter_count, owned.local_count), (0, 3));
    assert!(
        owned.instructions.ends_with(&[
            I::LoadLocal {
                local: 1,
                use_mode: kernel::ParameterUse::Consume
            },
            I::Unit,
            I::StoreLocal(0),
            I::Unit,
            I::StoreLocal(1),
            I::Unit,
            I::StoreLocal(2),
            I::Return,
        ]),
        "canonical cleanup must include moved first, returned second, and unused spare in forward order"
    );
    let count = code(unit_named(output, source, "buffer-count")?)?;
    assert!(count.instructions.contains(&I::LoadLocal {
        local: 1,
        use_mode: kernel::ParameterUse::Borrow
    }));
    assert!(count.instructions.ends_with(&[
        I::Unit,
        I::StoreLocal(0),
        I::Unit,
        I::StoreLocal(1),
        I::Return
    ]));
    assert_eq!(
        signature(unit_named(output, source, "size")?)?.parameters[0].use_mode,
        kernel::ParameterUse::Borrow
    );
    assert_eq!(
        signature(unit_named(output, source, "freeze")?)?.parameters[0].use_mode,
        kernel::ParameterUse::Consume
    );
    Ok(())
}

#[test]
fn selfhost_source_unit_mutations_distinguish_keys_payloads_and_presentation() -> Check<()> {
    let source = fixture_units()?;
    let baseline = baseline_units()?;
    let deadline = Duration::from_secs(60);

    let mut changed = source.clone();
    let literal = match &expression(&changed, fixture_function(&changed, "literals").body).operation
    {
        E::Sequence { items } => *items.last().ok_or_else(|| "literal sequence".to_owned())?,
        _ => return Err("literals is not Sequence".to_owned()),
    };
    expression_mut(&mut changed, literal).operation = E::I64 { value: 9 };
    let output = verify_units(
        "unit-private-body-mutation",
        &changed,
        target_id(&changed, "scalar-main"),
        false,
        deadline,
    )?;
    assert_ne!(
        unit_named(baseline, source, "literals")?.unit.key,
        unit_named(&output, &changed, "literals")?.unit.key
    );
    assert_eq!(
        unit_named(baseline, source, "owned-return")?.image,
        unit_named(&output, &changed, "owned-return")?.image
    );

    let mut changed = source.clone();
    let parameter = fixture_function(&changed, "identity").parameters[0];
    let Some(OwnerRecord::Parameter(record)) =
        changed.owners.get_mut(&OwnerKey::Parameter(parameter))
    else {
        return Err("identity parameter absent".to_owned());
    };
    record.name =
        kernel::Name::new("renamed-value").map_err(|error| format!("rename: {error:?}"))?;
    let output = verify_units(
        "unit-parameter-rename",
        &changed,
        target_id(&changed, "scalar-main"),
        false,
        deadline,
    )?;
    let before = unit_named(baseline, source, "identity")?;
    let after = unit_named(&output, &changed, "identity")?;
    assert_eq!(
        before.unit.key, after.unit.key,
        "presentation-only name is excluded from the key"
    );
    assert_ne!(
        before.image, after.image,
        "canonical signature parameter names are serialized"
    );
    assert_ne!(
        before.object, after.object,
        "the object digest binds the serialized name"
    );
    assert_eq!(
        baseline.summaries, output.summaries,
        "all seven executable dimensions exclude this parameter name"
    );

    let mut changed = source.clone();
    let boolean = scalar_type(&changed, kernel::TypeForm::Bool);
    let parameter = fixture_function(&changed, "identity").parameters[0];
    let Some(OwnerRecord::Parameter(record)) =
        changed.owners.get_mut(&OwnerKey::Parameter(parameter))
    else {
        return Err("identity parameter absent".to_owned());
    };
    record.ty = boolean;
    let owner = OwnerKey::Declaration(declaration(&changed, "identity"));
    let Some(OwnerRecord::Declaration(record)) = changed.owners.get_mut(&owner) else {
        return Err("identity absent".to_owned());
    };
    let kernel::DeclarationPayload::Function(function) = &mut record.payload else {
        return Err("identity function absent".to_owned());
    };
    function.result.ty = boolean;
    let output = verify_units(
        "unit-public-signature-mutation",
        &changed,
        target_id(&changed, "scalar-main"),
        false,
        deadline,
    )?;
    assert_ne!(
        before.unit.key,
        unit_named(&output, &changed, "identity")?.unit.key
    );
    assert_ne!(baseline.summaries[&owner][0], output.summaries[&owner][0]);

    let mut changed = source.clone();
    let (bindings, _) = root_bindings(&changed, "annotated")?;
    binding_mut(&mut changed, bindings[0])?.declared_type = None;
    let output = verify_units(
        "unit-binding-annotation-mutation",
        &changed,
        target_id(&changed, "scalar-main"),
        false,
        deadline,
    )?;
    let before = unit_named(baseline, source, "annotated")?;
    let after = unit_named(&output, &changed, "annotated")?;
    assert_ne!(before.unit.key, after.unit.key);
    assert_eq!(before.unit.tables, after.unit.tables);
    assert_eq!(before.unit.payload, after.unit.payload);

    let mut changed = source.clone();
    let (captures, initializer) = captured_pair(&changed)?;
    assert_eq!(captured_literals(&changed, &captures)?, [10, 20]);
    let E::Let { bindings, .. } = &mut expression_mut(&mut changed, initializer).operation else {
        return Err("canonical record-order capture absent".to_owned());
    };
    // Reorder evaluation, not the canonical selector layout. Both literals are
    // independent, so this remains valid source with exactly the same owners.
    bindings.swap(0, 1);
    let output = verify_units(
        "unit-structural-field-order-mutation",
        &changed,
        target_id(&changed, "scalar-main"),
        false,
        deadline,
    )?;
    let after = unit_named(&output, &changed, "fields")?;
    assert_eq!(
        literal_operands(unit_named(baseline, source, "fields")?)?,
        [10, 20]
    );
    assert_eq!(literal_operands(after)?, [20, 10]);
    assert_ne!(
        unit_named(baseline, source, "fields")?.unit.payload,
        after.unit.payload
    );
    assert_eq!(
        after
            .unit
            .tables
            .structural_names
            .iter()
            .map(kernel::Name::as_str)
            .collect::<Vec<_>>(),
        ["alpha", "zed"]
    );
    assert_ne!(
        unit_named(baseline, source, "fields")?.unit.key,
        after.unit.key
    );
    Ok(())
}

#[test]
fn selfhost_source_unit_admits_unused_descriptors_without_including_them_in_unit_bytes() -> Check<()>
{
    let source = fixture_units()?;
    let baseline = baseline_units()?;
    let mut changed = source.clone();
    let descriptor = kernel::TypeObject::new(kernel::TypeForm::StructuralRecord {
        fields: vec![kernel::StructuralTypeField {
            name: kernel::Name::new("unused-marker")
                .map_err(|error| format!("unused name: {error:?}"))?,
            ty: scalar_type(source, kernel::TypeForm::I64),
        }],
    })
    .map_err(|error| format!("unused type: {error:?}"))?;
    let digest = kernel::encode_type_object(&descriptor)
        .map_err(|error| format!("unused type encode: {error:?}"))?
        .0;
    assert!(changed.types.insert(digest, descriptor).is_none());
    let before_carrier = unit_carrier(source);
    // Unused supplied Type records do not change semantic-state or revision.
    // Preserve the exact real published core while extending only that inventory.
    let state = kernel::semantic_state_digest(source)
        .map_err(|error| format!("unused type original state: {error:?}"))?;
    let after_carrier = super::super::source_fixture::raw_claim_packet(
        &changed,
        state,
        carrier_provenance(&before_carrier)?,
    )
    .map_err(|error| format!("unused type raw carrier: {error:?}"))?;
    assert_ne!(sha256(&before_carrier), sha256(&after_carrier));
    assert_eq!(
        &before_carrier[48..80],
        &after_carrier[48..80],
        "semantic-state alone does not bind unused supplied types"
    );
    let before = carrier_provenance(&before_carrier)?;
    let after = carrier_provenance(&after_carrier)?;
    assert_eq!(before.claimed_revision, after.claimed_revision);
    assert_eq!(before.canonical_core, after.canonical_core);
    let output = verify_carrier_units(
        "unit-valid-unused-type",
        &changed,
        target_id(&changed, "scalar-main"),
        &after_carrier,
        source,
        false,
        Duration::from_secs(60),
    )?;
    assert_eq!(baseline.summary_bytes, output.summary_bytes);
    assert_eq!(baseline.unit_bytes, output.unit_bytes);
    Ok(())
}

fn reject_semantic(
    name: &str,
    source: &KernelSnapshot,
    diagnostic: i64,
    host_rejects: bool,
) -> Check<()> {
    reject_with_e1(name, source, 0, diagnostic, host_rejects)
}

fn reject_with_e1(
    name: &str,
    source: &KernelSnapshot,
    e1_diagnostic: i64,
    diagnostic: i64,
    host_rejects: bool,
) -> Check<()> {
    let carrier = graph::packet(source);
    // Bind the expected rejecting layer explicitly. A supported E1 shape must
    // reach E2; a shape unsupported by E1 must not be claimed as E2 coverage.
    let (code, state) = super::super::result("read-nominal", &carrier);
    assert_eq!(code, e1_diagnostic, "{name}: unexpected E1 diagnostic");
    let expected_state: &[u8] = if e1_diagnostic == 0 {
        &carrier[48..80]
    } else {
        &[]
    };
    assert_eq!(state.as_slice(), expected_state);
    let request = unit_request(&carrier, target_id(source, "scalar-main"))?;
    for summaries in [true, false] {
        assert!(
            native_unit_output(
                summaries,
                name,
                &request,
                diagnostic,
                Duration::from_secs(60)
            )?
            .is_empty()
        );
    }
    if host_rejects {
        assert!(
            kernel::validate_full(source).is_err(),
            "host validator accepted {name}"
        );
    }
    Ok(())
}

#[test]
fn selfhost_source_unit_rejects_uncalled_untaken_generic_and_custody_errors() -> Check<()> {
    let source = fixture_units()?;
    let mut changed = source.clone();
    let false_arm =
        match &expression(&changed, fixture_function(&changed, "unused-branch").body).operation {
            E::If { when_false, .. } => *when_false,
            _ => return Err("unused branch absent".to_owned()),
        };
    expression_mut(&mut changed, false_arm).operation = E::Bool { value: false };
    reject_semantic("unit-uncalled-untaken-wrong-type", &changed, 19, true)?;

    let mut changed = source.clone();
    let body = fixture_function(&changed, "generic-size").body;
    let E::Call { type_arguments, .. } = &mut expression_mut(&mut changed, body).operation else {
        return Err("generic-size call absent".to_owned());
    };
    type_arguments.clear();
    reject_semantic(
        "unit-uncalled-generic-missing-substitution",
        &changed,
        19,
        true,
    )?;

    let mut changed = source.clone();
    let boolean = scalar_type(&changed, kernel::TypeForm::Bool);
    let body = fixture_function(&changed, "generic-size").body;
    let E::Call { type_arguments, .. } = &mut expression_mut(&mut changed, body).operation else {
        return Err("generic-size call absent".to_owned());
    };
    type_arguments[0] = boolean;
    reject_semantic(
        "unit-uncalled-generic-outside-closed-i64-profile",
        &changed,
        13,
        true,
    )?;

    // Keep the supported I64 substitution and mutate one operand instead. This
    // must reach semantic incompatibility (19), not unsupported-profile (13).
    let mut changed = source.clone();
    let body = fixture_function(&changed, "generic-size").body;
    let list = match &expression(&changed, body).operation {
        E::Call { arguments, .. } => arguments[0],
        _ => return Err("generic-size call absent".to_owned()),
    };
    let item = match &expression(&changed, list).operation {
        E::List { items, .. } => items[0],
        _ => return Err("generic-size list operand absent".to_owned()),
    };
    expression_mut(&mut changed, item).operation = E::Bool { value: true };
    reject_semantic(
        "unit-uncalled-generic-wrong-element-type",
        &changed,
        19,
        true,
    )?;

    let mut changed = source.clone();
    let other_port = kernel::PortReference {
        package: changed.root.package_id,
        port: port(&changed, "main"),
    };
    let target = OwnerKey::Target(target_id(&changed, "unit-binary"));
    let Some(OwnerRecord::Target(record)) = changed.owners.get_mut(&target) else {
        return Err("unselected target absent".to_owned());
    };
    record.port = Some(other_port);
    reject_semantic(
        "unit-unselected-target-crosses-components",
        &changed,
        19,
        true,
    )?;

    let mut changed = source.clone();
    let parameter = external(&changed, "size")?.parameters[0];
    let Some(OwnerRecord::Parameter(record)) =
        changed.owners.get_mut(&OwnerKey::Parameter(parameter))
    else {
        return Err("size parameter absent".to_owned());
    };
    record.use_mode = kernel::ParameterUse::Consume;
    reject_semantic("unit-intrinsic-borrow-mode-mutation", &changed, 19, true)?;

    let mut changed = source.clone();
    let (bindings, body) = root_bindings(&changed, "unused-dispose")?;
    let last = match &expression(&changed, body).operation {
        E::Sequence { items } => *items
            .last()
            .ok_or_else(|| "dispose sequence empty".to_owned())?,
        _ => return Err("dispose sequence absent".to_owned()),
    };
    let argument = match &expression(&changed, last).operation {
        E::Call { arguments, .. } => arguments[0],
        _ => return Err("dispose call absent".to_owned()),
    };
    expression_mut(&mut changed, argument).operation = E::Local {
        value: kernel::LocalValueReference::LexicalBinding(bindings[0]),
    };
    reject_semantic("unit-uncalled-double-consume", &changed, 19, true)?;
    assert!(
        !kernel::memory_reference::accepts(&changed),
        "independent custody oracle accepted double consume"
    );

    let mut changed = source.clone();
    let (bindings, _) = root_bindings(&changed, "owned-return")?;
    binding_mut(&mut changed, bindings[0])?.declared_type = None;
    reject_semantic("unit-unannotated-owned-binding", &changed, 19, true)?;

    let mut changed = source.clone();
    let object = kernel::TypeObject::new(kernel::TypeForm::F64)
        .map_err(|error| format!("unused F64: {error:?}"))?;
    let digest = kernel::encode_type_object(&object)
        .map_err(|error| format!("unused F64 encoding: {error:?}"))?
        .0;
    assert!(changed.types.insert(digest, object).is_none());
    reject_with_e1("unit-unused-unsupported-f64-shape", &changed, 13, 13, false)?;

    // List<Text> has supported E1 framing and endpoint closure, but lies outside
    // the active E2 List<I64> profile. Keep this separate from the F64 shape case.
    let mut changed = source.clone();
    let text = changed
        .types
        .iter()
        .find_map(|(digest, object)| {
            matches!(object.form, kernel::TypeForm::Text).then_some(*digest)
        })
        .ok_or_else(|| "fixture Text type absent".to_owned())?;
    let object = kernel::TypeObject::new(kernel::TypeForm::List { item: text })
        .map_err(|error| format!("unused List<Text>: {error:?}"))?;
    let digest = kernel::encode_type_object(&object)
        .map_err(|error| format!("unused List<Text> encoding: {error:?}"))?
        .0;
    assert!(changed.types.insert(digest, object).is_none());
    reject_semantic("unit-unused-unsupported-text-list", &changed, 13, false)?;
    Ok(())
}

#[test]
fn selfhost_source_unit_preserves_request_and_explicit_allowance_refusals() -> Check<()> {
    let source = fixture_units()?;
    let carrier = unit_carrier(source);
    let target = target_id(source, "scalar-main");
    let request = unit_request(&carrier, target)?;
    assert_eq!(&request[..8], b"LKJSRQ01");
    assert_eq!(&request[8..12], &2_u32.to_le_bytes());
    assert_eq!(request[29], 1);
    assert_eq!(&request[30..32], &[0, 0]);
    assert_eq!(&request[32..], carrier.as_slice());
    assert_eq!(
        &carrier[100..112],
        &[0; 12],
        "ordinary proof policy is None"
    );
    let provenance_at = carrier_word(&carrier, 116)?;
    let mut cases = Vec::new();
    cases.push(("unit-request-truncated", request[..31].to_vec(), 12));
    let mut wrong = request.clone();
    wrong[8..12].copy_from_slice(&1_u32.to_le_bytes());
    cases.push(("unit-request-predecessor-profile", wrong, 12));
    let mut wrong = request.clone();
    wrong[12] = 2;
    cases.push(("unit-request-wrong-owner-domain", wrong, 12));
    let mut wrong = request.clone();
    wrong[13..29].fill(0);
    cases.push(("unit-request-zero-target", wrong, 12));
    let mode_zero = source_input::request(&carrier, target, SourceOutputMode::Elf)
        .map_err(|error| format!("unit mode zero control: {error:?}"))?;
    cases.push(("unit-probe-requires-canonical-mode", mode_zero, 12));
    let mut wrong = request.clone();
    wrong[29] = 2;
    cases.push(("unit-request-unknown-mode", wrong, 12));
    for (at, name) in [
        (30, "unit-request-first-reserved-byte"),
        (31, "unit-request-second-reserved-byte"),
    ] {
        let mut wrong = request.clone();
        wrong[at] = 1;
        cases.push((name, wrong, 12));
    }
    let mut wrong = request.clone();
    wrong[32 + 112..32 + 116].copy_from_slice(&0_u32.to_le_bytes());
    cases.push(("unit-source-predecessor-profile", wrong, 12));
    let mut wrong = request.clone();
    wrong[32 + 124] = 1;
    cases.push(("unit-source-reserved-byte", wrong, 12));
    let mut wrong = request.clone();
    wrong[32 + provenance_at] ^= 1;
    cases.push(("unit-source-wrong-revision-claim", wrong, 12));
    let mut wrong = request.clone();
    wrong[32 + 120..32 + 124].copy_from_slice(&u32::MAX.to_le_bytes());
    cases.push(("unit-source-provenance-extent", wrong, 12));
    let mut wrong = request.clone();
    wrong.push(0);
    cases.push(("unit-request-trailing-byte", wrong, 12));
    let mut bounded = carrier.clone();
    graph::set_allowance(&mut bounded, Some(0));
    cases.push((
        "unit-source-explicit-zero-allowance",
        unit_request(&bounded, target)?,
        6,
    ));
    let missing = TargetId::migrate(b"unit-request-missing-target", 0);
    assert!(!source.owners.contains_key(&OwnerKey::Target(missing)));
    cases.push((
        "unit-request-missing-selected-target",
        unit_request(&carrier, missing)?,
        19,
    ));
    cases.push((
        "unit-source-admission-before-target-lookup",
        unit_request(&bounded, missing)?,
        6,
    ));
    for (name, request, diagnostic) in cases {
        for summaries in [true, false] {
            assert!(
                native_unit_output(
                    summaries,
                    name,
                    &request,
                    diagnostic,
                    Duration::from_secs(60)
                )?
                .is_empty()
            );
        }
    }
    // u64::MAX remains tag 1 and its literal finite value. For this finite small
    // input it permits exactly the same complete result as the ordinary None path.
    graph::set_allowance(&mut bounded, Some(u64::MAX));
    assert_eq!(&bounded[100..104], &1_u32.to_le_bytes());
    assert_eq!(&bounded[104..112], &u64::MAX.to_le_bytes());
    let request = unit_request(&bounded, target)?;
    let baseline = baseline_units()?;
    for (summaries, expected) in [
        (true, &baseline.summary_bytes),
        (false, &baseline.unit_bytes),
    ] {
        assert_eq!(
            native_unit_output(
                summaries,
                "unit-source-finite-maximum-allowance",
                &request,
                0,
                Duration::from_secs(60)
            )?,
            *expected
        );
    }
    Ok(())
}

#[test]
#[ignore = "whole maintained owner/type inventory; run explicitly under the shared target lease"]
fn selfhost_source_units_inventory_every_maintained_compilation_owner() -> Check<()> {
    let source = &compiler().0;
    let selected = source
        .owners
        .iter()
        .find_map(|(owner, _)| match owner {
            OwnerKey::Target(target) => Some(*target),
            _ => None,
        })
        .ok_or_else(|| "maintained compiler has no target".to_owned())?;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/selfhost/project");
    let repository = GraphRepository::open(&root)
        .map_err(|error| format!("maintained repository: {error:?}"))?;
    let current = repository
        .current()
        .map_err(|error| format!("maintained authority: {error:?}"))?;
    let view = repository
        .view_current()
        .map_err(|error| format!("maintained view: {error:?}"))?;
    assert_eq!(
        view.revision(),
        current.head.revision,
        "authority moved before exact export"
    );
    let public_export = view
        .export_package_transport()
        .map_err(|error| format!("maintained export: {error:?}"))?;
    let container =
        PackageContainer::decode(&public_export.container, public_export.transport_digest)
            .map_err(|error| format!("maintained container: {error:?}"))?;
    let exported = ExportedSource::from_container(&container)
        .map_err(|error| format!("maintained original source: {error:?}"))?;
    let raw = exported.as_raw();
    assert_eq!(
        raw.provenance.claimed_revision,
        current.head.revision.bytes()
    );
    assert_eq!(raw.owners.len(), source.owners.len());
    assert_eq!(
        raw.types.len(),
        source.types.len(),
        "complete supplied type inventory is required"
    );
    assert_eq!(raw.retirements.len(), source.retirements.len());
    let carrier =
        source_input::carrier(&raw).map_err(|error| format!("maintained carrier: {error:?}"))?;
    assert_eq!(
        carrier,
        graph::packet(source),
        "shared maintained path must preserve the exact exported input"
    );
    let provenance = carrier_provenance(&carrier)?;
    assert_eq!(provenance.claimed_revision, raw.provenance.claimed_revision);
    assert_eq!(provenance.canonical_core, raw.provenance.canonical_core);
    let semantic_state = kernel::semantic_state_digest(source)
        .map_err(|error| format!("maintained source state: {error:?}"))?;
    assert_eq!(
        semantic_state, current.accepted.semantic_state,
        "compiler snapshot is not the current authority"
    );
    let head = fs::read(root.join("HEAD")).map_err(|error| format!("maintained HEAD: {error}"))?;
    // This is an explicit test wall-clock policy, not a compiler admission ceiling.
    // Do not overlap it with other large native workloads or timed acceptance controls.
    let output = verify_carrier_units(
        "maintained-whole-owner-units",
        source,
        selected,
        &carrier,
        source,
        false,
        Duration::from_secs(3600),
    )?;
    assert!(!output.units.is_empty());
    assert_eq!(output.summaries.len(), source.owners.len());
    let after = repository
        .current()
        .map_err(|error| format!("maintained final authority: {error:?}"))?;
    assert_eq!(
        current.head.revision, after.head.revision,
        "authority changed during the unit inventory control"
    );
    assert_eq!(
        head,
        fs::read(root.join("HEAD")).map_err(|error| format!("maintained final HEAD: {error}"))?
    );
    eprintln!(
        "source unit maintained inventory revision={} owner_count={} type_count={} unit_count={} semantic_state={} \
        head_sha256={} export_sha256={} raw_core_sha256={} carrier_sha256={} summaries_sha256={} units_sha256={} \
        source_profile=3 request_profile=2 output_mode=1 complete_raw_source=true host_ir_input=false artifact_fixed_point=false",
        current.head.revision,
        source.owners.len(),
        source.types.len(),
        output.units.len(),
        current.accepted.semantic_state,
        sha256(&head),
        sha256(&public_export.container),
        sha256(raw.provenance.canonical_core),
        sha256(&carrier),
        sha256(&output.summary_bytes),
        sha256(&output.unit_bytes)
    );
    Ok(())
}
