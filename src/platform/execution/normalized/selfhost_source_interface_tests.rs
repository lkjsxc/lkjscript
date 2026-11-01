//! Output-only interface oracles. Nest under selfhost_source_scalar_tests.
//! The probe consumes one complete profile-3 source/profile-2 mode-1 request.
//! Host selection, witness reconstruction, and interface construction occur only
//! after all four probe engines return. The normalized backend input below builds
//! a test probe executable; it never replaces the raw graph supplied to that probe.
use super::*;
use crate::platform::change::WitnessBaseRead;
use crate::platform::compiler::selfhost_source_input::{
    self as raw_source, ExportedSource, RawType, SourceOutputMode,
};
use crate::platform::kernel::{EncodedOwnerKey, PackageInterfaceDigest, TypeObjectDigest};
use crate::platform::package_interface::{
    PackageInterfaceBuild, PackageInterfaceOwner, PackageInterfaceOwnerDigest,
    PackageInterfaceSelection, build_package_interface, package_interface_digest,
    validate_package_interface,
};
use crate::platform::package_transport::source::PackageContainer;
use crate::platform::persistent_map::{MapContentDigest, MapRoot, PageDigest};
use crate::platform::storage::memory::MemoryPackedStore;
use crate::platform::storage::object::{ImmutableObjectStore, ObjectDomain, ObjectKey, StoreWork};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

type Check<T> = Result<T, String>;
const FIXTURE: &str = include_str!("../../../../tests/fixtures/selfhost-source-interface.lkjc");
const PROBE: &str =
    include_str!("../../../../tests/fixtures/selfhost-source-interface-probes.lkjc");

fn declaration_id(
    source: &KernelSnapshot,
    module_name: &str,
    name: &str,
) -> Check<DeclarationReference> {
    let module = source
        .owners
        .iter()
        .find_map(|(key, record)| match (key, record) {
            (OwnerKey::Module(id), OwnerRecord::Module(record))
                if record.name.as_str() == module_name =>
            {
                Some(*id)
            }
            _ => None,
        })
        .ok_or_else(|| format!("missing module {module_name}"))?;
    let declaration = source
        .owners
        .iter()
        .find_map(|(key, record)| match (key, record) {
            (OwnerKey::Declaration(id), OwnerRecord::Declaration(record))
                if record.module == module && record.name.as_str() == name =>
            {
                Some(*id)
            }
            _ => None,
        })
        .ok_or_else(|| format!("missing function {module_name}::{name}"))?;
    Ok(DeclarationReference {
        package: source.root.package_id,
        declaration,
    })
}

fn selected_target(source: &KernelSnapshot, name: &str) -> Check<TargetId> {
    source
        .owners
        .iter()
        .find_map(|(key, record)| match (key, record) {
            (OwnerKey::Target(id), OwnerRecord::Target(record)) if record.name.as_str() == name => {
                Some(*id)
            }
            _ => None,
        })
        .ok_or_else(|| format!("missing caller target {name}"))
}

struct JoinedChild(Child);
impl Drop for JoinedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn native_output(
    label: &str,
    executable: &[u8],
    input: &[u8],
    diagnostic: i64,
    deadline: Duration,
) -> Check<Vec<u8>> {
    crate::release_container::inspect_static_elf_bytes(executable)
        .map_err(|error| format!("{label}: static ELF: {error:?}"))?;
    let directory = tempfile::tempdir().map_err(|error| format!("{label}: tempdir: {error}"))?;
    let binary = directory.path().join("interface-probe");
    let stdin_path = directory.path().join("input.bin");
    let stdout_path = directory.path().join("output.bin");
    let stderr_path = directory.path().join("diagnostic.txt");
    crate::platform::owned_output::publish_executable_create_new(
        &binary,
        executable,
        None,
        "source interface test",
    )
    .map_err(|error| format!("{label}: publish executable: {error:?}"))?;
    fs::write(&stdin_path, input).map_err(|error| format!("{label}: stdin: {error}"))?;
    let stdin = File::open(&stdin_path).map_err(|error| format!("{label}: stdin open: {error}"))?;
    let stdout = File::create(&stdout_path).map_err(|error| format!("{label}: stdout: {error}"))?;
    let stderr = File::create(&stderr_path).map_err(|error| format!("{label}: stderr: {error}"))?;
    let start = Instant::now();
    let mut child = JoinedChild(
        Command::new(&binary)
            .env_clear()
            .current_dir(directory.path())
            .stdin(Stdio::from(stdin))
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .spawn()
            .map_err(|error| format!("{label}: spawn: {error}"))?,
    );
    let status = loop {
        if let Some(status) = child
            .0
            .try_wait()
            .map_err(|error| format!("{label}: wait: {error}"))?
        {
            break status;
        }
        if start.elapsed() >= deadline {
            return Err(format!(
                "{label}: caller-selected test deadline {deadline:?} exceeded"
            ));
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let output = fs::read(&stdout_path).map_err(|error| format!("{label}: output: {error}"))?;
    let errors =
        fs::read(&stderr_path).map_err(|error| format!("{label}: diagnostics: {error}"))?;
    if status.code() != Some(if diagnostic == 0 { 0 } else { 65 }) {
        return Err(format!(
            "{label}: exit={:?}; stderr={}",
            status.code(),
            String::from_utf8_lossy(&errors)
        ));
    }
    if diagnostic == 0 {
        if !errors.is_empty() {
            return Err(format!("{label}: successful filter wrote diagnostics"));
        }
    } else if !output.is_empty() || errors != format!("compiler_rejected:{diagnostic}\n").as_bytes()
    {
        return Err(format!("{label}: noncanonical rejection output"));
    }
    eprintln!(
        "source interface native case={label} executable_sha256={} input_sha256={} input_bytes={} \
        output_sha256={} output_bytes={} elapsed_ms={} host_deadline_seconds={} environment=empty \
        arguments=[] io=regular-files capture_byte_quota=none",
        sha256(executable),
        sha256(input),
        input.len(),
        sha256(&output),
        output.len(),
        start.elapsed().as_millis(),
        deadline.as_secs()
    );
    Ok(output)
}

struct ProbeProgram {
    source: KernelSnapshot,
    program: NormalizedProgram,
    entry: DeclarationReference,
    rust_image: Vec<u8>,
    selfhost_image: Vec<u8>,
}

fn build_probe() -> Check<ProbeProgram> {
    let changes =
        proposals::updates(PROBE).map_err(|error| format!("probe proposal: {error:?}"))?;
    let memory = crate::platform::MemProjectStore::new(None)
        .map_err(|error| format!("probe store: {error:?}"))?;
    let repository = GraphRepository::create_memory(&memory, &compiler().0, None)
        .map_err(|error| format!("probe repository: {error:?}"))?
        .repository;
    let source = publish_proposals(&repository, changes);
    let program = prepare_from_repository(&repository);
    let entry = declaration_id(&source, "source-interface-probe", "inspect-interface")?;
    let index = program
        .function(entry)
        .ok_or_else(|| "probe entry not compiled".to_owned())?
        .0 as usize;
    let rust_image = crate::platform::compiler::elf_x86_64::compile_function(&program, index)
        .map_err(|error| format!("Rust probe backend: {error:?}"))?
        .0;

    // Bootstrap only: compile the accepted probe graph using the maintained
    // normalized selfhost backend. Every subsequent probe invocation receives
    // raw source. This test does not claim raw-source self-compilation or C2=C3.
    let normalized = crate::platform::compiler::selfhost_input::export(&program, index)
        .map_err(|error| format!("probe backend input: {error:?}"))?;
    let (backend_source, backend_program) = compiler();
    let backend_entry = declaration_id(backend_source, "elf", "compile-elf")?;
    let backend_index = backend_program
        .function(backend_entry)
        .ok_or_else(|| "maintained backend entry absent".to_owned())?
        .0 as usize;
    let backend_image =
        crate::platform::compiler::elf_x86_64::compile_function(backend_program, backend_index)
            .map_err(|error| format!("backend bootstrap: {error:?}"))?
            .0;
    let selfhost_image = native_output(
        "probe-bootstrap-normalized-backend",
        &backend_image,
        &normalized,
        0,
        Duration::from_secs(600),
    )?;
    crate::release_container::inspect_static_elf_bytes(&selfhost_image)
        .map_err(|error| format!("selfhost probe ELF: {error:?}"))?;
    eprintln!(
        "source interface probe construction=public-proposal-and-normalized-bootstrap \
        normalized_sha256={} backend_sha256={} rust_probe_sha256={} selfhost_probe_sha256={} \
        raw_source_fixed_point=false",
        sha256(&normalized),
        sha256(&backend_image),
        sha256(&rust_image),
        sha256(&selfhost_image)
    );
    Ok(ProbeProgram {
        source,
        program,
        entry,
        rust_image,
        selfhost_image,
    })
}

fn probe_program() -> Check<&'static ProbeProgram> {
    static PROBE_PROGRAM: OnceLock<Check<ProbeProgram>> = OnceLock::new();
    PROBE_PROGRAM
        .get_or_init(build_probe)
        .as_ref()
        .map_err(Clone::clone)
}

fn filter_result(value: &NormalizedValue) -> Check<(i64, &[u8])> {
    let NormalizedValue::Record(NormalizedRecord::Structural { fields }) = value else {
        return Err("probe result is not a structural record".to_owned());
    };
    let diagnostic = fields
        .iter()
        .find(|(name, _)| name.as_str() == "diagnostic")
        .ok_or_else(|| "probe diagnostic absent".to_owned())?;
    let image = fields
        .iter()
        .find(|(name, _)| name.as_str() == "image")
        .ok_or_else(|| "probe image absent".to_owned())?;
    match (&diagnostic.1, &image.1) {
        (NormalizedValue::I64(code), NormalizedValue::Bytes(bytes)) => Ok((*code, bytes.as_ref())),
        _ => Err("probe result has wrong field types".to_owned()),
    }
}

fn four_engines(
    label: &str,
    request: &[u8],
    diagnostic: i64,
    deadline: Duration,
) -> Check<Vec<u8>> {
    let probe = probe_program()?;
    let arguments = vec![NormalizedValue::bytes(request)];
    let policy = NormalizedRunPolicy::foreground();
    let control = ExecutionControl::uncancelled();
    let vm = NormalizedVm::for_test(&probe.program, policy)
        .invoke(probe.entry, arguments.clone(), None, &control)
        .map_err(|error| format!("{label}: VM: {error:?}"))?;
    let reference = NormalizedReferenceInterpreter::new(&probe.source, &probe.program, policy)
        .invoke(probe.entry, arguments, None, &control)
        .map_err(|error| format!("{label}: reference: {error:?}"))?;
    assert_eq!(vm.0, reference.0, "{label}: VM/reference disagreement");
    assert_eq!(vm.1.live_call_frames_after, 0);
    assert_eq!(reference.1.live_call_frames_after, 0);
    let (code, expected) = filter_result(&vm.0)?;
    assert_eq!(code, diagnostic, "{label}: source admission result");
    if diagnostic != 0 {
        assert!(
            expected.is_empty(),
            "{label}: failed projection leaked output"
        );
    }
    for (name, image) in [
        ("rust-native", &probe.rust_image),
        ("selfhost-native", &probe.selfhost_image),
    ] {
        let output = native_output(
            &format!("{label}/{name}"),
            image,
            request,
            diagnostic,
            deadline,
        )?;
        assert_eq!(output, expected, "{label}: {name} output differs");
    }
    let carrier = request
        .get(32..)
        .ok_or_else(|| "test request has no carrier".to_owned())?;
    eprintln!(
        "source interface four-engine case={label} carrier_sha256={} carrier_bytes={} \
        request_sha256={} request_profile=2 requested_mode={} output_sha256={} output_bytes={} \
        vm_modeled_allocation={} reference_modeled_allocation={}",
        sha256(carrier),
        carrier.len(),
        sha256(request),
        request.get(29).copied().map(u64::from).unwrap_or(u64::MAX),
        sha256(expected),
        expected.len(),
        vm.1.allocated_bytes,
        reference.1.allocated_bytes
    );
    Ok(expected.to_vec())
}

struct InputCase {
    source: KernelSnapshot,
    container: PackageContainer,
    target: TargetId,
    carrier: Vec<u8>,
    request: Vec<u8>,
}

fn from_repository(repository: &GraphRepository, target_name: &str) -> Check<InputCase> {
    let source = repository
        .view_current()
        .map_err(|error| format!("source view: {error:?}"))?
        .reconstruct_full_oracle()
        .map_err(|error| format!("source readback: {error:?}"))?
        .value;
    let target = selected_target(&source, target_name)?;
    let exported = repository
        .export_package_transport()
        .map_err(|error| format!("public source export: {error:?}"))?;
    let container = PackageContainer::decode(&exported.container, exported.transport_digest)
        .map_err(|error| format!("public export framing: {error:?}"))?;
    let original = ExportedSource::from_container(&container)
        .map_err(|error| format!("raw source view: {error:?}"))?;
    let raw = original.as_raw();
    if raw.owners.len() != source.owners.len() {
        return Err("export omitted original owners".to_owned());
    }
    let carrier = raw_source::carrier(&raw).map_err(|error| format!("carrier: {error:?}"))?;
    let request = raw_source::request(&carrier, target, SourceOutputMode::CanonicalArtifact)
        .map_err(|error| format!("request: {error:?}"))?;
    eprintln!(
        "source interface source repository={} package={} revision={} semantic_state={} \
        source_owners={} supplied_types={} raw_carrier_sha256={} request_sha256={}",
        source.root.repository_id,
        source.root.package_id,
        crate::platform::semantic_id::encode_hex(&raw.provenance.claimed_revision),
        crate::platform::semantic_id::encode_hex(&raw.semantic_state),
        raw.owners.len(),
        raw.types.len(),
        sha256(&carrier),
        sha256(&request)
    );
    Ok(InputCase {
        source,
        container,
        target,
        carrier,
        request,
    })
}

fn authored_case(proposal: &str) -> Check<InputCase> {
    let source = byte_buffer_tests::author_only(proposal)?;
    case_from_snapshot(&source)
}

fn case_from_snapshot(source: &KernelSnapshot) -> Check<InputCase> {
    let memory = crate::platform::MemProjectStore::new(None)
        .map_err(|error| format!("fixture store: {error:?}"))?;
    let repository = GraphRepository::create_memory(&memory, source, None)
        .map_err(|error| format!("fixture publication: {error:?}"))?
        .repository;
    from_repository(&repository, "scalar-main")
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
            .ok_or_else(|| "truncated interface probe output".to_owned())?;
        self.at = end;
        Ok(bytes)
    }
    fn word(&mut self) -> Check<u64> {
        Ok(u64::from_le_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| "u64 width".to_owned())?,
        ))
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
        if self.at == self.bytes.len() {
            Ok(())
        } else {
            Err("trailing interface probe bytes".to_owned())
        }
    }
}

struct InterfaceOutput {
    root: MapRoot,
    digest: PackageInterfaceDigest,
    owners: BTreeMap<OwnerKey, PackageInterfaceOwnerDigest>,
    types: BTreeSet<TypeObjectDigest>,
    objects: BTreeMap<ObjectKey, Vec<u8>>,
}

fn decode_output(bytes: &[u8]) -> Check<InterfaceOutput> {
    let mut c = Cursor { bytes, at: 0 };
    if c.take(8)? != b"LKJSIF01" {
        return Err("interface probe magic".to_owned());
    }
    let page = PageDigest::from_bytes(c.digest()?);
    let count = c.word()?;
    let content = MapContentDigest::from_bytes(c.digest()?);
    let root = MapRoot::from_parts(page, count, content);
    let digest = PackageInterfaceDigest::from_bytes(c.digest()?);
    let owner_bytes = c.size()?;
    let type_bytes = c.size()?;
    let object_bytes = c.size()?;
    if owner_bytes % 49 != 0 || type_bytes % 32 != 0 {
        return Err("partial output directory row".to_owned());
    }
    if u64::try_from(owner_bytes / 49).map_err(|_| "owner count conversion".to_owned())? != count {
        return Err("map root count differs from directory".to_owned());
    }
    let mut owners = BTreeMap::new();
    let mut previous_owner = None;
    for row in c.take(owner_bytes)?.as_chunks::<49>().0 {
        let owner = EncodedOwnerKey::decode(&row[..17])
            .map_err(|error| format!("owner directory: {error:?}"))?;
        let encoded: [u8; 17] = row[..17]
            .try_into()
            .map_err(|_| "owner key width".to_owned())?;
        if previous_owner.is_some_and(|previous| previous >= encoded) {
            return Err("owner directory order".to_owned());
        }
        previous_owner = Some(encoded);
        let digest = PackageInterfaceOwnerDigest::from_bytes(
            row[17..]
                .try_into()
                .map_err(|_| "owner digest width".to_owned())?,
        );
        if owners.insert(owner, digest).is_some() {
            return Err("duplicate owner directory entry".to_owned());
        }
    }
    let mut types = BTreeSet::new();
    let mut previous_type = None;
    for row in c.take(type_bytes)?.as_chunks::<32>().0 {
        let bytes = *row;
        if previous_type.is_some_and(|previous| previous >= bytes) {
            return Err("type directory order".to_owned());
        }
        previous_type = Some(bytes);
        if !types.insert(TypeObjectDigest::from_bytes(bytes)) {
            return Err("duplicate Type".to_owned());
        }
    }
    let mut frames = Cursor {
        bytes: c.take(object_bytes)?,
        at: 0,
    };
    let mut objects = BTreeMap::new();
    while frames.at < frames.bytes.len() {
        let domain = match frames.word()? {
            2 => ObjectDomain::Type,
            5 => ObjectDomain::MapPage,
            23 => ObjectDomain::PackageInterface,
            tag => return Err(format!("unexpected generated object domain {tag}")),
        };
        let key = ObjectKey::from_digest(domain, frames.digest()?);
        let length = frames.size()?;
        let image = frames.take(length)?;
        key.verify(image)
            .map_err(|error| format!("generated object hash: {error:?}"))?;
        if objects.insert(key, image.to_vec()).is_some() {
            return Err("duplicate generated ObjectKey".to_owned());
        }
    }
    frames.finish()?;
    c.finish()?;
    Ok(InterfaceOutput {
        root,
        digest,
        owners,
        types,
        objects,
    })
}

struct HostInterface {
    owners: BTreeMap<OwnerKey, PackageInterfaceOwner>,
    types: BTreeMap<TypeObjectDigest, Vec<u8>>,
    build: PackageInterfaceBuild,
}

fn host_oracle(source: &KernelSnapshot) -> Check<HostInterface> {
    // Called after the four outputs. Public readback/witness binds every original
    // canonical owner, but no host-projected object is a compiler input.
    let memory = crate::platform::MemProjectStore::new(None)
        .map_err(|error| format!("oracle store: {error:?}"))?;
    let repository = GraphRepository::create_memory(&memory, source, None)
        .map_err(|error| format!("oracle source admission: {error:?}"))?
        .repository;
    let view = repository
        .view_current()
        .map_err(|error| format!("oracle view: {error:?}"))?;
    let readback = view
        .reconstruct_full_oracle()
        .map_err(|error| format!("oracle full readback: {error:?}"))?
        .value;
    if readback.owners != source.owners {
        return Err("oracle readback changed owners".to_owned());
    }
    let selection = PackageInterfaceSelection::from_records(source.root.package_id, &source.owners)
        .map_err(|error| format!("oracle public selection: {error:?}"))?;
    let mut owners = BTreeMap::new();
    let mut pending = Vec::new();
    for (owner, canonical) in &source.owners {
        let summary = view
            .read_owner_summary(*owner)
            .map_err(|error| format!("oracle summary: {error:?}"))?
            .value
            .ok_or_else(|| format!("oracle summary absent for {owner:?}"))?;
        if let Some(projected) =
            PackageInterfaceOwner::project(canonical, &summary.summary, &selection)
                .map_err(|error| format!("oracle projection: {error:?}"))?
        {
            pending.extend(projected.type_roots());
            owners.insert(*owner, projected);
        }
    }
    let mut types = BTreeMap::new();
    while let Some(digest) = pending.pop() {
        if types.contains_key(&digest) {
            continue;
        }
        let object = source
            .types
            .get(&digest)
            .ok_or_else(|| format!("oracle missing Type {digest:?}"))?;
        pending.extend(object.child_types());
        let (encoded, bytes) = kernel::encode_type_object(object)
            .map_err(|error| format!("oracle Type encoding: {error:?}"))?;
        if encoded != digest {
            return Err("oracle Type digest differs".to_owned());
        }
        types.insert(digest, bytes);
    }
    let build = build_package_interface(&owners, &types)
        .map_err(|error| format!("oracle interface build: {error:?}"))?;
    Ok(HostInterface {
        owners,
        types,
        build,
    })
}

fn compare_strict(source: &KernelSnapshot, bytes: &[u8]) -> Check<InterfaceOutput> {
    let output = decode_output(bytes)?;
    let oracle = host_oracle(source)?;
    assert_eq!(
        output.root, oracle.build.root,
        "exact physical map root/count/content"
    );
    assert_eq!(
        output.digest,
        package_interface_digest(source.root.package_id, oracle.build.root.content_root())
            .map_err(|error| format!("oracle interface digest: {error:?}"))?
    );
    assert_eq!(
        output.objects, oracle.build.objects,
        "complete canonical generated object closure"
    );
    assert_eq!(
        output.types,
        oracle.types.keys().copied().collect::<BTreeSet<_>>()
    );
    assert_eq!(output.owners.len(), oracle.owners.len());
    for (owner, expected) in &oracle.owners {
        let (digest, _) = expected
            .encode()
            .map_err(|error| format!("oracle owner encoding: {error:?}"))?;
        assert_eq!(output.owners.get(owner), Some(&digest));
        let image = output
            .objects
            .get(&ObjectKey::from_digest(
                ObjectDomain::PackageInterface,
                digest.bytes(),
            ))
            .ok_or_else(|| "emitted interface owner object absent".to_owned())?;
        let decoded = PackageInterfaceOwner::decode(image, *owner, digest)
            .map_err(|error| format!("strict interface owner decoder: {error:?}"))?;
        assert_eq!(&decoded, expected);
    }
    let mut store = MemoryPackedStore::default();
    let mut work = StoreWork::default();
    for (key, image) in &output.objects {
        store
            .stage(*key, image, &mut work)
            .map_err(|error| format!("strict interface stage: {error:?}"))?;
        if key.domain == ObjectDomain::Type {
            let digest = TypeObjectDigest::from_bytes(key.digest.bytes());
            let decoded = kernel::decode_type_object(image, digest)
                .map_err(|error| format!("strict original Type: {error:?}"))?;
            assert_eq!(source.types.get(&digest), Some(&decoded));
        }
    }
    let validated =
        validate_package_interface(source.root.package_id, output.root, &store, &mut work)
            .map_err(|error| format!("strict complete interface validation: {error:?}"))?;
    assert_eq!(validated.owners, oracle.owners);
    assert_eq!(
        validated
            .type_objects
            .keys()
            .copied()
            .collect::<BTreeSet<_>>(),
        output.types
    );
    assert_eq!(
        validated.reachable_objects,
        output.objects.keys().copied().collect::<BTreeSet<_>>(),
        "no missing/orphan closure objects"
    );
    Ok(output)
}

#[test]
fn selfhost_source_interface_matches_full_objects_and_strict_validation_across_four_engines()
-> Check<()> {
    let case = authored_case(FIXTURE)?;
    let bytes = four_engines(
        "complete-public-interface",
        &case.request,
        0,
        Duration::from_secs(60),
    )?;
    let output = compare_strict(&case.source, &bytes)?;
    assert!(!output.owners.is_empty());
    assert!(
        output.types.len() >= 7,
        "fixture must exercise compound and generic Types"
    );
    // The parser/strict loader must reject a framing mutation independently.
    let mut trailing = bytes;
    trailing.push(0);
    assert!(decode_output(&trailing).is_err());
    Ok(())
}

#[test]
fn selfhost_source_interface_public_signature_and_private_body_mutations_have_independent_results()
-> Check<()> {
    let case = authored_case(FIXTURE)?;
    let before = four_engines(
        "interface-before-mutation",
        &case.request,
        0,
        Duration::from_secs(60),
    )?;
    let baseline = compare_strict(&case.source, &before)?;
    // Keep existing identities while authoring one different private literal;
    // the new snapshot becomes authoritative only through public repository creation.
    let mut private_source = case.source.clone();
    let hidden = declaration_id(&private_source, "interface-controls", "hidden")?;
    let body = match private_source
        .owners
        .get(&OwnerKey::Declaration(hidden.declaration))
    {
        Some(OwnerRecord::Declaration(record)) => match &record.payload {
            kernel::DeclarationPayload::Function(function) => function.body,
            _ => return Err("private fixture is not a function".to_owned()),
        },
        _ => return Err("private fixture declaration absent".to_owned()),
    };
    match private_source.owners.get_mut(&OwnerKey::Expression(body)) {
        Some(OwnerRecord::Expression(record)) => record.operation = E::I64 { value: 31 },
        _ => return Err("private fixture body absent".to_owned()),
    }
    let private = case_from_snapshot(&private_source)?;
    let bytes = four_engines(
        "interface-private-body",
        &private.request,
        0,
        Duration::from_secs(60),
    )?;
    compare_strict(&private.source, &bytes)?;
    assert_eq!(
        bytes, before,
        "private implementation must not change interface"
    );
    assert_ne!(sha256(&private.carrier), sha256(&case.carrier));

    // Rename one public generic parameter while preserving its stable typed ID
    // through a supported public rename proposal on the existing source graph.
    let memory = crate::platform::MemProjectStore::new(None)
        .map_err(|error| format!("rename store: {error:?}"))?;
    let repository = GraphRepository::create_memory(&memory, &case.source, None)
        .map_err(|error| format!("rename repository: {error:?}"))?
        .repository;
    let original = declaration_id(&case.source, "interface-controls", "list-get")?;
    let parameter = case
        .source
        .owners
        .iter()
        .find_map(|(owner, record)| match (owner, record) {
            (OwnerKey::Parameter(id), OwnerRecord::Parameter(record))
                if record.parent == kernel::ParameterParent::Function(original.declaration)
                    && record.name.as_str() == "items" =>
            {
                Some(*id)
            }
            _ => None,
        })
        .ok_or_else(|| "generic parameter absent".to_owned())?;
    let accepted = repository
        .view_current()
        .map_err(|error| format!("rename base: {error:?}"))?
        .revision();
    let change =
        format!("request base={accepted}\nrename.owner owner={parameter} name=renamed-items\n");
    let decoded = crate::platform::control::decode_compact_change(
        "interface parameter rename",
        change.as_bytes(),
    )
    .map_err(|error| format!("rename decode: {error:?}"))?;
    let plan = repository
        .prepare_authored_change(&decoded.semantic, decoded.options)
        .map_err(|error| format!("rename plan: {error:?}"))?;
    repository
        .publish(&plan.publication)
        .map_err(|error| format!("rename public apply: {error:?}"))?;
    let renamed = from_repository(&repository, "scalar-main")?;
    assert!(
        renamed
            .source
            .owners
            .contains_key(&OwnerKey::Parameter(parameter))
    );
    let bytes = four_engines(
        "interface-public-parameter-name",
        &renamed.request,
        0,
        Duration::from_secs(60),
    )?;
    let public = compare_strict(&renamed.source, &bytes)?;
    assert_ne!(public.digest, baseline.digest);
    assert_ne!(public.root, baseline.root);
    assert_eq!(public.types, baseline.types);
    Ok(())
}

#[test]
fn selfhost_source_interface_checks_request_mode_allowance_and_unused_type_endpoints() -> Check<()>
{
    let case = authored_case(FIXTURE)?;
    let original = ExportedSource::from_container(&case.container)
        .map_err(|error| format!("original raw view: {error:?}"))?;
    let raw = original.as_raw();
    let wrong_mode = raw_source::request(&case.carrier, case.target, SourceOutputMode::Elf)
        .map_err(|error| format!("wrong-mode request: {error:?}"))?;
    four_engines(
        "interface-mode0-is-not-interface-probe",
        &wrong_mode,
        12,
        Duration::from_secs(60),
    )?;
    let mut old_profile = case.request.clone();
    old_profile
        .get_mut(8..12)
        .ok_or_else(|| "request profile extent".to_owned())?
        .copy_from_slice(&1_u32.to_le_bytes());
    four_engines(
        "interface-rejects-obsolete-request-profile",
        &old_profile,
        12,
        Duration::from_secs(60),
    )?;
    let finite = raw_source::carrier_with_allowance(&raw, Some(0))
        .map_err(|error| format!("finite raw policy: {error:?}"))?;
    let finite = raw_source::request(&finite, case.target, SourceOutputMode::CanonicalArtifact)
        .map_err(|error| format!("finite request: {error:?}"))?;
    four_engines(
        "interface-explicit-zero-proof-policy",
        &finite,
        6,
        Duration::from_secs(60),
    )?;

    // Hash-consistent unused descriptor: no Rust source semantic admission or
    // repair runs between inserting this row and the four compiler invocations.
    let absent = TypeObjectDigest::from_bytes([0xa5; 32]);
    assert!(!raw.types.iter().any(|row| row.digest == absent.bytes()));
    let malformed = kernel::TypeObject::new(kernel::TypeForm::List { item: absent })
        .map_err(|error| format!("unused descriptor: {error:?}"))?;
    let (hash, image) = kernel::encode_type_object(&malformed)
        .map_err(|error| format!("unused descriptor encoding: {error:?}"))?;
    let mut types = raw.types.to_vec();
    types.push(RawType {
        digest: hash.bytes(),
        bytes: &image,
    });
    types.sort_by_key(|row| row.digest);
    let changed = raw_source::RawSourceGraph {
        types: &types,
        ..raw
    };
    let carrier = raw_source::carrier(&changed)
        .map_err(|error| format!("unused descriptor carrier: {error:?}"))?;
    assert_ne!(sha256(&carrier), sha256(&case.carrier));
    assert_eq!(carrier.get(48..80), case.carrier.get(48..80));
    let request = raw_source::request(&carrier, case.target, SourceOutputMode::CanonicalArtifact)
        .map_err(|error| format!("unused descriptor request: {error:?}"))?;
    four_engines(
        "interface-unused-Type-missing-child",
        &request,
        14,
        Duration::from_secs(60),
    )?;
    Ok(())
}

#[test]
#[ignore = "explicit costly whole maintained source interface inventory across four engines"]
fn selfhost_source_interface_inventories_whole_maintained_project_against_rust() -> Check<()> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/selfhost/project");
    let repository = GraphRepository::open(&root)
        .map_err(|error| format!("maintained repository: {error:?}"))?;
    let case = from_repository(&repository, "compile-source-elf")?;
    let bytes = four_engines(
        "maintained-complete-interface",
        &case.request,
        0,
        Duration::from_secs(3600),
    )?;
    let output = compare_strict(&case.source, &bytes)?;
    eprintln!(
        "source interface maintained owners={} relevant_types={} generated_objects={} interface={} root={:?}",
        output.owners.len(),
        output.types.len(),
        output.objects.len(),
        output.digest,
        output.root
    );
    Ok(())
}
