//! Raw-source ELF and canonical artifact acceptance controls.
//! Nest under selfhost_source_scalar_tests. The production entry receives only
//! the complete public source carrier and caller TargetId/output mode. C0 build,
//! link, normalized preparation and canonical source evaluation are independent
//! output oracles; none of their results are supplied to a self-produced stage.
use super::*;
use crate::platform::compiler::selfhost_source_input::{
    self as raw_source, ExportedSource, SourceOutputMode,
};
use crate::platform::compiler::{
    CompilationBinding, CompilationManifest, CompilationUnit, LoadedArtifact, OptimizationPolicy,
    build_clean, link_artifact, load_artifact,
};
use crate::platform::kernel::{
    EncodedOwnerKey, OwnerKind, PackageRevisionDigest, TypeObjectDigest,
};
use crate::platform::package_transport::{PackageRevision, source::PackageContainer};
use crate::platform::persistent_map::{MapWork, PersistentMap};
use crate::platform::storage::object::{ObjectDomain, ObjectKey};
use crate::platform::storage::page_store::ObjectPageReader;
use std::collections::BTreeSet;
use std::fmt::Debug;
use std::path::Path;
use std::time::{Duration, Instant};

#[path = "selfhost_source_artifact_support.rs"]
mod artifact_support;

#[path = "selfhost_source_stage_controls.rs"]
mod stage_controls;

#[path = "selfhost_source_bulk_tests.rs"]
mod bulk;

#[path = "native_phase_tests.rs"]
mod phase;
use artifact_support::{RepositoryCopy, run_filter, run_scalar};

type Check<T> = Result<T, String>;
const FIXTURE: &str = include_str!("../../../../tests/fixtures/selfhost-source-artifact.lkjc");
const FOCUSED_DEADLINE: Duration = Duration::from_secs(600);
const WHOLE_SOURCE_DEADLINE: Duration = Duration::from_secs(3600);

fn ensure(condition: bool, label: &str) -> Check<()> {
    if condition {
        Ok(())
    } else {
        Err(label.to_owned())
    }
}

fn equal<T: PartialEq + Debug>(actual: T, expected: T, label: &str) -> Check<()> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{label}: actual={actual:?}; expected={expected:?}"))
    }
}

fn equal_bytes(actual: &[u8], expected: &[u8], label: &str) -> Check<()> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "{label}: actual_bytes={} actual_sha256={} expected_bytes={} expected_sha256={}",
            actual.len(),
            sha256(actual),
            expected.len(),
            sha256(expected)
        ))
    }
}

fn function_named(
    source: &KernelSnapshot,
    module_name: &str,
    name: &str,
) -> Check<DeclarationReference> {
    let module = source
        .owners
        .iter()
        .find_map(|(key, owner)| match (key, owner) {
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
        .find_map(|(key, owner)| match (key, owner) {
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

fn target_named(source: &KernelSnapshot, name: &str) -> Check<TargetId> {
    source
        .owners
        .iter()
        .find_map(|(key, owner)| match (key, owner) {
            (OwnerKey::Target(id), OwnerRecord::Target(record)) if record.name.as_str() == name => {
                Some(*id)
            }
            _ => None,
        })
        .ok_or_else(|| format!("missing caller target {name}"))
}

fn function_body(source: &KernelSnapshot, module: &str, name: &str) -> Check<ExpressionId> {
    let reference = function_named(source, module, name)?;
    match source
        .owners
        .get(&OwnerKey::Declaration(reference.declaration))
    {
        Some(OwnerRecord::Declaration(record)) => match &record.payload {
            kernel::DeclarationPayload::Function(function) => Ok(function.body),
            _ => Err(format!("{module}::{name} is not a source function")),
        },
        _ => Err(format!("missing declaration {module}::{name}")),
    }
}

fn operation(source: &KernelSnapshot, expression: ExpressionId) -> Check<&E> {
    match source.owners.get(&OwnerKey::Expression(expression)) {
        Some(OwnerRecord::Expression(record)) => Ok(&record.operation),
        _ => Err(format!("missing expression {expression}")),
    }
}

fn set_operation(source: &mut KernelSnapshot, expression: ExpressionId, operation: E) -> Check<()> {
    match source.owners.get_mut(&OwnerKey::Expression(expression)) {
        Some(OwnerRecord::Expression(record)) => {
            record.operation = operation;
            Ok(())
        }
        _ => Err(format!("missing expression {expression}")),
    }
}

struct InputCase {
    repository: GraphRepository,
    source: KernelSnapshot,
    container: PackageContainer,
    revision: PackageRevision,
    revision_digest: PackageRevisionDigest,
    revision_bytes: Vec<u8>,
    canonical_core: Vec<u8>,
    carrier: Vec<u8>,
}

impl InputCase {
    fn from_repository(repository: GraphRepository) -> Check<Self> {
        let view = repository
            .view_current()
            .map_err(|error| format!("source view: {error:?}"))?;
        let source = view
            .reconstruct_full_oracle()
            .map_err(|error| format!("source readback: {error:?}"))?
            .value;
        let exported = view
            .export_package_transport()
            .map_err(|error| format!("public source export: {error:?}"))?;
        let container = PackageContainer::decode(&exported.container, exported.transport_digest)
            .map_err(|error| format!("public source framing: {error:?}"))?;
        let original = ExportedSource::from_container(&container)
            .map_err(|error| format!("original raw source inventory: {error:?}"))?;
        let raw = original.as_raw();
        equal(
            raw.owners.len(),
            source.owners.len(),
            "complete exported owner inventory",
        )?;
        equal(
            raw.types.len(),
            source.types.len(),
            "complete supplied type inventory",
        )?;
        equal(
            raw.retirements.len(),
            source.retirements.len(),
            "complete retirement inventory",
        )?;
        equal(
            raw.package,
            source.root.package_id.bytes(),
            "original package identity",
        )?;
        equal(
            raw.provenance.claimed_revision,
            exported
                .revision
                .revision
                .revision_id()
                .map_err(|error| format!("public revision: {error:?}"))?
                .bytes(),
            "original revision claim",
        )?;
        equal(
            raw.semantic_state,
            exported.revision.revision.semantic_state.bytes(),
            "original semantic state",
        )?;
        let canonical_core = raw.provenance.canonical_core.to_vec();
        let carrier =
            raw_source::carrier(&raw).map_err(|error| format!("raw source carrier: {error:?}"))?;
        eprintln!(
            "raw source input repository={} package={} revision={} semantic_state={} \
            owners={} supplied_types={} retirements={} carrier_sha256={} core_sha256={} source_profile=3",
            source.root.repository_id,
            source.root.package_id,
            crate::platform::semantic_id::encode_hex(&raw.provenance.claimed_revision),
            crate::platform::semantic_id::encode_hex(&raw.semantic_state),
            raw.owners.len(),
            raw.types.len(),
            raw.retirements.len(),
            sha256(&carrier),
            sha256(&canonical_core)
        );
        drop(original);
        drop(view);
        Ok(Self {
            repository,
            source,
            container,
            revision: exported.revision,
            revision_digest: exported.revision_digest,
            revision_bytes: exported.revision_bytes,
            canonical_core,
            carrier,
        })
    }

    fn request(&self, target: &str, mode: SourceOutputMode) -> Check<Vec<u8>> {
        raw_source::request(&self.carrier, target_named(&self.source, target)?, mode)
            .map_err(|error| format!("raw request: {error:?}"))
    }
}

fn case_from_snapshot(source: &KernelSnapshot) -> Check<InputCase> {
    let memory = crate::platform::MemProjectStore::new(None)
        .map_err(|error| format!("fixture memory store: {error:?}"))?;
    let repository = GraphRepository::create_memory(&memory, source, None)
        .map_err(|error| format!("fixture public repository creation: {error:?}"))?
        .repository;
    InputCase::from_repository(repository)
}

fn fixture_case() -> Check<InputCase> {
    case_from_snapshot(&byte_buffer_tests::author_only(FIXTURE)?)
}

fn native_image(program: &NormalizedProgram, entry: DeclarationReference) -> Check<Vec<u8>> {
    let index = program
        .function(entry)
        .ok_or_else(|| "native entry absent from program".to_owned())?
        .0;
    let index = usize::try_from(index).map_err(|_| "native function index extent".to_owned())?;
    crate::platform::compiler::elf_x86_64::compile_function(program, index)
        .map(|output| output.0)
        .map_err(|error| format!("C0 native emission: {error:?}"))
}

fn bootstrap_compiler() -> Check<&'static [u8]> {
    static IMAGE: OnceLock<Check<Vec<u8>>> = OnceLock::new();
    IMAGE
        .get_or_init(|| {
            let (source, program) = compiler();
            native_image(program, function_named(source, "source-elf", "compile")?)
        })
        .as_ref()
        .map(Vec::as_slice)
        .map_err(Clone::clone)
}

struct RawOutputs {
    elf: Vec<u8>,
    artifact: Vec<u8>,
}

fn raw_outputs(
    label: &str,
    compiler_image: &[u8],
    case: &InputCase,
    target: &str,
    deadline: Duration,
) -> Check<RawOutputs> {
    let elf_request = case.request(target, SourceOutputMode::Elf)?;
    let artifact_request = case.request(target, SourceOutputMode::CanonicalArtifact)?;
    // No C0 target build, normalization, inferred type table, custody witness or
    // owner selection is constructed before these complete raw requests execute.
    let elf = run_filter(
        &format!("{label}/mode0"),
        compiler_image,
        &elf_request,
        0,
        deadline,
    )?;
    let artifact = run_filter(
        &format!("{label}/mode1"),
        compiler_image,
        &artifact_request,
        0,
        deadline,
    )?;
    ensure(
        !elf.is_empty() && !artifact.is_empty(),
        "raw compiler emitted empty success",
    )?;
    crate::release_container::inspect_static_elf_bytes(&elf)
        .map_err(|error| format!("{label}: emitted ELF: {error:?}"))?;
    Ok(RawOutputs { elf, artifact })
}

struct HostOracle {
    bytes: Vec<u8>,
    loaded: LoadedArtifact,
    program: NormalizedProgram,
}

fn host_oracle(case: &InputCase) -> Check<HostOracle> {
    let build = build_clean(&case.repository, OptimizationPolicy::DeterministicBaseline)
        .map_err(|error| format!("independent C0 build: {error:?}"))?;
    let linked = link_artifact(&case.repository, build.manifest_digest, &[])
        .map_err(|error| format!("independent C0 link: {error:?}"))?;
    let bytes = linked.artifact.bytes;
    let loaded = load_artifact(&bytes)
        .map_err(|error| format!("independent C0 strict loader: {error:?}"))?;
    let program = NormalizedProgram::prepare(loaded.clone())
        .map_err(|error| format!("independent C0 preparation: {error:?}"))?;
    eprintln!(
        "raw source C0 output oracle compilation_manifest={} artifact_sha256={} bytes={} units_compiled={}",
        build.manifest_digest,
        sha256(&bytes),
        bytes.len(),
        build.units_compiled
    );
    Ok(HostOracle {
        bytes,
        loaded,
        program,
    })
}

fn strict_artifact(
    label: &str,
    bytes: &[u8],
    case: &InputCase,
    host: &HostOracle,
) -> Check<LoadedArtifact> {
    let loaded =
        load_artifact(bytes).map_err(|error| format!("{label}: strict load: {error:?}"))?;
    loaded
        .require_current_admission(|| Ok(()))
        .map_err(|error| format!("{label}: admission: {error:?}"))?;
    equal_bytes(bytes, &host.bytes, &format!("{label}: Ahost byte equality"))?;
    ensure(
        loaded.manifest == host.loaded.manifest,
        "strict artifact manifest differs from C0",
    )?;
    ensure(
        loaded.objects == host.loaded.objects,
        "strict artifact object bytes differ from C0",
    )?;
    equal(
        loaded.bundle_digest,
        host.loaded.bundle_digest,
        "strict artifact bundle identity",
    )?;
    equal(
        loaded
            .admitted_content_digest(|| Ok(()))
            .map_err(|error| format!("artifact content binding: {error:?}"))?,
        host.loaded
            .admitted_content_digest(|| Ok(()))
            .map_err(|error| format!("C0 content binding: {error:?}"))?,
        "strict artifact admitted-content identity",
    )?;
    equal(
        loaded.manifest.root_package,
        case.source.root.package_id,
        "artifact root package",
    )?;
    equal(
        loaded.manifest.packages.len(),
        1,
        "active source profile dependency closure",
    )?;
    let package = loaded
        .root_package()
        .ok_or_else(|| "strict root package absent".to_owned())?;
    let revision = case
        .revision
        .revision
        .revision_id()
        .map_err(|error| format!("revision ID: {error:?}"))?;
    equal(
        package.repository_id,
        case.source.root.repository_id,
        "artifact repository",
    )?;
    equal(
        package.package_revision,
        case.revision_digest,
        "artifact package revision",
    )?;
    equal(
        package.semantic_revision,
        revision,
        "artifact semantic revision",
    )?;
    equal(
        package.semantic_state,
        case.revision.revision.semantic_state,
        "artifact semantic state",
    )?;
    equal(
        package.interface,
        case.revision.interface,
        "source-derived public interface",
    )?;
    let revision_key =
        ObjectKey::from_digest(ObjectDomain::PackageRevision, case.revision_digest.bytes());
    let revision_bytes = loaded
        .objects
        .get(&revision_key)
        .ok_or_else(|| "artifact PackageRevision missing".to_owned())?;
    equal_bytes(
        revision_bytes,
        &case.revision_bytes,
        "exact original PackageRevision bytes",
    )?;
    equal(
        PackageRevision::decode(revision_bytes, case.revision_digest)
            .map_err(|error| format!("artifact PackageRevision: {error:?}"))?,
        case.revision.clone(),
        "exact revision core",
    )?;
    let manifest_key = ObjectKey::from_digest(
        ObjectDomain::CompilationManifest,
        package.compilation.bytes(),
    );
    let manifest = CompilationManifest::decode(
        loaded
            .objects
            .get(&manifest_key)
            .ok_or_else(|| "compilation manifest object absent".to_owned())?,
        package.compilation,
    )
    .map_err(|error| format!("compilation manifest: {error:?}"))?;
    equal(manifest.revision, revision, "compilation revision")?;
    equal(
        manifest.package_revision,
        case.revision_digest,
        "compilation PackageRevision",
    )?;
    equal(
        manifest.semantic_state,
        case.revision.revision.semantic_state,
        "compilation semantic state",
    )?;
    equal(
        manifest.package_interface,
        case.revision.interface,
        "compilation interface",
    )?;
    let expected = case
        .source
        .owners
        .iter()
        .filter_map(|(owner, record)| {
            matches!(
                record.kind(),
                OwnerKind::External
                    | OwnerKind::PureFunction
                    | OwnerKind::Component
                    | OwnerKind::Target
            )
            .then_some(*owner)
        })
        .collect::<BTreeSet<_>>();
    let mut rows = Vec::new();
    PersistentMap::from_root(manifest.units)
        .for_each(
            &ObjectPageReader::new(&loaded),
            &mut MapWork::default(),
            |key, value| {
                rows.push((key.to_vec(), value.to_vec()));
                Ok(())
            },
        )
        .map_err(|error| format!("unit-map full inventory: {error:?}"))?;
    let mut actual = BTreeSet::new();
    for (key, value) in rows {
        let owner =
            EncodedOwnerKey::decode(&key).map_err(|error| format!("unit owner key: {error:?}"))?;
        let binding = CompilationBinding::decode(&value, owner)
            .map_err(|error| format!("unit binding: {error:?}"))?;
        let object = ObjectKey::from_digest(ObjectDomain::CompilerUnit, binding.object.bytes());
        let image = loaded
            .objects
            .get(&object)
            .ok_or_else(|| "unit canonical image missing".to_owned())?;
        let unit = CompilationUnit::decode(image, object)
            .map_err(|error| format!("strict unit decode: {error:?}"))?;
        equal(unit.key, binding.key, "unit exact key")?;
        equal(unit.source.owner, owner, "unit exact owner")?;
        equal(
            unit.source.package,
            case.source.root.package_id,
            "unit exact package",
        )?;
        equal(unit.source.kind, binding.kind, "unit exact kind")?;
        ensure(actual.insert(owner), "duplicate compiled owner")?;
    }
    equal(
        actual,
        expected,
        "all compilation owners, including uncalled and unselected",
    )?;
    eprintln!(
        "raw source strict artifact case={label} sha256={} manifest={} bundle={} objects={} \
        units={} revision={} semantic_state={} interface={} source_carrier_sha256={}",
        sha256(bytes),
        loaded.manifest_digest,
        loaded.bundle_digest,
        loaded.objects.len(),
        manifest.units.entries(),
        revision,
        package.semantic_state,
        package.interface,
        sha256(&case.carrier)
    );
    Ok(loaded)
}

fn scalar_behavior(
    label: &str,
    case: &InputCase,
    host: &HostOracle,
    loaded: LoadedArtifact,
    elf: &[u8],
    (function, expected): (&str, i64),
    deadline: Duration,
) -> Check<()> {
    let entry = function_named(&case.source, "artifact-controls", function)?;
    let program = NormalizedProgram::prepare(loaded)
        .map_err(|error| format!("selfhost artifact preparation: {error:?}"))?;
    let policy = NormalizedRunPolicy::foreground();
    let control = ExecutionControl::uncancelled();
    let vm = NormalizedVm::for_test(&program, policy)
        .invoke(entry, Vec::new(), None, &control)
        .map_err(|error| format!("{label}: artifact VM: {error:?}"))?;
    let reference = NormalizedReferenceInterpreter::new(&case.source, &host.program, policy)
        .invoke(entry, Vec::new(), None, &control)
        .map_err(|error| format!("{label}: canonical reference: {error:?}"))?;
    equal(
        vm.0,
        NormalizedValue::I64(expected),
        &format!("{label}: artifact VM value"),
    )?;
    equal(
        reference.0,
        NormalizedValue::I64(expected),
        &format!("{label}: canonical reference value"),
    )?;
    equal(
        vm.1.live_call_frames_after,
        0,
        "artifact VM frames reclaimed",
    )?;
    equal(
        reference.1.live_call_frames_after,
        0,
        "reference frames reclaimed",
    )?;
    let native = native_image(&host.program, entry)?;
    run_scalar(&format!("{label}/C0-native"), &native, expected, deadline)?;
    run_scalar(
        &format!("{label}/raw-source-native"),
        elf,
        expected,
        deadline,
    )
}

fn filter_value(value: &NormalizedValue) -> Check<(i64, &[u8])> {
    let NormalizedValue::Record(NormalizedRecord::Structural { fields }) = value else {
        return Err("filter result is not structural record".to_owned());
    };
    let diagnostic = fields
        .iter()
        .find(|(name, _)| name.as_str() == "diagnostic")
        .ok_or_else(|| "filter diagnostic missing".to_owned())?;
    let image = fields
        .iter()
        .find(|(name, _)| name.as_str() == "image")
        .ok_or_else(|| "filter image missing".to_owned())?;
    match (&diagnostic.1, &image.1) {
        (NormalizedValue::I64(code), NormalizedValue::Bytes(image)) => Ok((*code, image.as_ref())),
        _ => Err("filter result field types".to_owned()),
    }
}

fn binary_behavior(
    case: &InputCase,
    host: &HostOracle,
    loaded: LoadedArtifact,
    elf: &[u8],
) -> Check<()> {
    let input: &[u8] = &[0, 1, 127, 128, 255, 0, 42];
    let entry = function_named(&case.source, "artifact-controls", "binary-entry")?;
    let program = NormalizedProgram::prepare(loaded)
        .map_err(|error| format!("binary artifact prepare: {error:?}"))?;
    let arguments = vec![NormalizedValue::bytes(input)];
    let policy = NormalizedRunPolicy::foreground();
    let control = ExecutionControl::uncancelled();
    let vm = NormalizedVm::for_test(&program, policy)
        .invoke(entry, arguments.clone(), None, &control)
        .map_err(|error| format!("binary artifact VM: {error:?}"))?;
    let reference = NormalizedReferenceInterpreter::new(&case.source, &host.program, policy)
        .invoke(entry, arguments, None, &control)
        .map_err(|error| format!("binary reference: {error:?}"))?;
    equal(filter_value(&vm.0)?, (0, input), "binary artifact VM")?;
    equal(
        filter_value(&reference.0)?,
        (0, input),
        "binary canonical reference",
    )?;
    equal(vm.1.live_call_frames_after, 0, "binary VM frames reclaimed")?;
    equal(
        reference.1.live_call_frames_after,
        0,
        "binary reference frames reclaimed",
    )?;
    let native = native_image(&host.program, entry)?;
    equal_bytes(
        &run_filter("binary/C0-native", &native, input, 0, FOCUSED_DEADLINE)?,
        input,
        "C0 binary behavior",
    )?;
    equal_bytes(
        &run_filter("binary/raw-native", elf, input, 0, FOCUSED_DEADLINE)?,
        input,
        "raw binary behavior",
    )
}

#[test]
fn selfhost_source_artifact_two_modes_match_c0_and_four_execution_engines() -> Check<()> {
    let case = fixture_case()?;
    let output = raw_outputs(
        "independent",
        bootstrap_compiler()?,
        &case,
        "scalar-main",
        FOCUSED_DEADLINE,
    )?;
    let host = host_oracle(&case)?;
    let loaded = strict_artifact("independent", &output.artifact, &case, &host)?;
    scalar_behavior(
        "independent",
        &case,
        &host,
        loaded,
        &output.elf,
        ("entry", 42),
        FOCUSED_DEADLINE,
    )
}

fn mutated_case(source: &KernelSnapshot) -> Check<InputCase> {
    let mut source = source.clone();
    let mut changed = 0usize;
    for owner in source.owners.values_mut() {
        if let OwnerRecord::Expression(record) = owner
            && matches!(record.operation, E::I64 { value: 41 })
        {
            record.operation = E::I64 { value: 42 };
            changed += 1;
        }
    }
    equal(changed, 1, "exact independent source behavior mutation")?;
    // A complete authoring proposal is accepted by a fresh public repository.
    // This intentional S' has a new revision; no accepted repository is edited.
    case_from_snapshot(&source)
}

#[test]
fn selfhost_source_artifact_target_selection_and_source_mutation_are_observable() -> Check<()> {
    let case = fixture_case()?;
    let compiler_image = bootstrap_compiler()?;
    let main = raw_outputs(
        "target/main",
        compiler_image,
        &case,
        "scalar-main",
        FOCUSED_DEADLINE,
    )?;
    let other = raw_outputs(
        "target/other",
        compiler_image,
        &case,
        "other-main",
        FOCUSED_DEADLINE,
    )?;
    let binary = raw_outputs(
        "target/binary",
        compiler_image,
        &case,
        "binary-main",
        FOCUSED_DEADLINE,
    )?;
    equal_bytes(
        &main.artifact,
        &other.artifact,
        "complete artifact independent of selected scalar target",
    )?;
    equal_bytes(
        &main.artifact,
        &binary.artifact,
        "complete artifact independent of selected binary target",
    )?;
    ensure(
        main.elf != other.elf && main.elf != binary.elf,
        "selected ELF did not change",
    )?;
    let host = host_oracle(&case)?;
    let loaded = strict_artifact("target/main", &main.artifact, &case, &host)?;
    scalar_behavior(
        "target/main",
        &case,
        &host,
        loaded,
        &main.elf,
        ("entry", 42),
        FOCUSED_DEADLINE,
    )?;
    let loaded = strict_artifact("target/other", &other.artifact, &case, &host)?;
    scalar_behavior(
        "target/other",
        &case,
        &host,
        loaded,
        &other.elf,
        ("other-entry", 11),
        FOCUSED_DEADLINE,
    )?;
    let loaded = strict_artifact("target/binary", &binary.artifact, &case, &host)?;
    binary_behavior(&case, &host, loaded, &binary.elf)?;
    let changed = mutated_case(&case.source)?;
    let output = raw_outputs(
        "source-mutated",
        compiler_image,
        &changed,
        "scalar-main",
        FOCUSED_DEADLINE,
    )?;
    ensure(
        case.carrier != changed.carrier
            && main.artifact != output.artifact
            && main.elf != output.elf,
        "source mutation failed to change committed input and generated outputs",
    )?;
    let changed_host = host_oracle(&changed)?;
    let loaded = strict_artifact("source-mutated", &output.artifact, &changed, &changed_host)?;
    scalar_behavior(
        "source-mutated",
        &changed,
        &changed_host,
        loaded,
        &output.elf,
        ("entry", 43),
        FOCUSED_DEADLINE,
    )
}

struct Rejection {
    label: &'static str,
    carrier: Vec<u8>,
    diagnostic: i64,
    e1_diagnostic: Option<i64>,
    invalid_source: Option<KernelSnapshot>,
}

fn admission_layer_result(name: &str, carrier: &[u8], diagnostic: i64) -> Check<()> {
    let module = super::super::probe_module(name);
    let (source, program) = super::super::probes(module);
    let entry = function_named(source, module, name)?;
    let arguments = vec![NormalizedValue::bytes(carrier)];
    let policy = NormalizedRunPolicy::foreground();
    let control = ExecutionControl::uncancelled();
    let vm = NormalizedVm::for_test(program, policy)
        .invoke(entry, arguments.clone(), None, &control)
        .map_err(|error| format!("{name}: admission VM: {error:?}"))?;
    let reference = NormalizedReferenceInterpreter::new(source, program, policy)
        .invoke(entry, arguments, None, &control)
        .map_err(|error| format!("{name}: admission reference: {error:?}"))?;
    let expected = if diagnostic == 0 {
        carrier
            .get(48..80)
            .ok_or_else(|| "admission carrier state extent".to_owned())?
    } else {
        &[][..]
    };
    equal(
        filter_value(&vm.0)?,
        (diagnostic, expected),
        &format!("{name}: VM layer result"),
    )?;
    equal(
        filter_value(&reference.0)?,
        (diagnostic, expected),
        &format!("{name}: reference layer result"),
    )?;
    equal(
        vm.1.live_call_frames_after,
        0,
        "admission VM frames reclaimed",
    )?;
    equal(
        reference.1.live_call_frames_after,
        0,
        "admission reference frames reclaimed",
    )?;
    eprintln!(
        "raw source rejection layer={name} diagnostic={diagnostic} carrier_sha256={} \
        carrier_bytes={} independent_engines=vm,canonical-reference supplied_to_compiler=false",
        sha256(carrier),
        carrier.len()
    );
    Ok(())
}

fn reject_admission_layers(
    label: &str,
    image: &[u8],
    fallback_target: TargetId,
    deadline: Duration,
) -> Check<()> {
    use super::super::{namespace, nominal_flow, scope, type_scope};
    for (name, source, prior, layer, diagnostic, oracle_code) in [
        (
            "self-binding-initializer",
            scope::artifact_rejection_fixture(),
            "read-graph",
            "read-syntax",
            15,
            "kernel_full_lexical_scope",
        ),
        (
            "duplicate-top-level-module",
            namespace::artifact_rejection_fixture()?,
            "read-syntax",
            "read-namespace",
            16,
            "kernel_full_namespace_duplicate",
        ),
        (
            "foreign-TypeParameter-context",
            type_scope::artifact_rejection_fixture()?,
            "read-namespace",
            "read-type-scope",
            17,
            "kernel_full_type_parameter_scope",
        ),
        (
            "expanding-nominal-self-slot",
            nominal_flow::artifact_rejection_fixture(),
            "read-type-scope",
            "read-nominal",
            18,
            "kernel_type_nominal_expansion",
        ),
    ] {
        // Existing complete fixture builders mutate canonical records and rehash
        // their owner directories. The single production writer binds all raw
        // objects, original supplied Types and synthetic fixture RevisionCore.
        let carrier = graph::packet(&source);
        admission_layer_result(prior, &carrier, 0)?;
        admission_layer_result(layer, &carrier, diagnostic)?;
        // Type/nominal-only fixtures need no target to demonstrate a prior E1
        // failure. A well-framed caller TargetId is still supplied; selection is
        // intentionally downstream of all four tested admission layers.
        let target = source
            .owners
            .keys()
            .find_map(|owner| match owner {
                OwnerKey::Target(id) => Some(*id),
                _ => None,
            })
            .unwrap_or(fallback_target);
        for mode in [SourceOutputMode::Elf, SourceOutputMode::CanonicalArtifact] {
            let request = raw_source::request(&carrier, target, mode)
                .map_err(|error| format!("layer-reaching request: {error:?}"))?;
            run_filter(
                &format!("{label}/{mode:?}/{name}"),
                image,
                &request,
                diagnostic,
                deadline,
            )?;
        }
        // The independent snapshot oracle's live-Type rule differs from raw
        // carrier admission, which checks unused descriptors too. Apply the
        // existing neutral oracle projection only after native rejection; it
        // never changes the raw request and supplies no evidence to a stage.
        let oracle = if diagnostic == 18 {
            nominal_flow::live_type_oracle(&source)
        } else {
            source.clone()
        };
        let errors = kernel::validate_full(&oracle)
            .err()
            .ok_or_else(|| format!("independent kernel accepted {name}"))?;
        ensure(
            errors.iter().any(|error| {
                error.code == oracle_code
                    && error.class == crate::platform::DiagnosticClass::Semantic
            }),
            &format!("{name}: independent kernel missed intended {oracle_code}: {errors:?}"),
        )?;
        ensure(
            errors.iter().all(|error| {
                error.code != "kernel_full_map_count"
                    && error.class != crate::platform::DiagnosticClass::Resource
            }),
            &format!(
                "{name}: independent oracle was stopped by framing/resource error: {errors:?}"
            ),
        )?;
        equal_bytes(
            &graph::packet(&source),
            &carrier,
            "oracle inspection preserved complete raw negative carrier",
        )?;
    }
    Ok(())
}

fn rejection_carriers(case: &InputCase) -> Check<Vec<Rejection>> {
    let original = ExportedSource::from_container(&case.container)
        .map_err(|error| format!("negative raw source: {error:?}"))?;
    let raw = original.as_raw();
    let mut cases = Vec::new();
    let mut claim = raw.provenance;
    claim.claimed_revision[0] ^= 1;
    cases.push(Rejection {
        label: "wrong-revision-claim",
        carrier: raw_source::carrier(&raw_source::RawSourceGraph {
            provenance: claim,
            ..raw
        })
        .map_err(|error| format!("claim carrier: {error:?}"))?,
        diagnostic: 12,
        e1_diagnostic: None,
        invalid_source: None,
    });
    let mut state = raw.semantic_state;
    state[0] ^= 1;
    cases.push(Rejection {
        label: "core-state-disagreement",
        carrier: raw_source::carrier(&raw_source::RawSourceGraph {
            semantic_state: state,
            ..raw
        })
        .map_err(|error| format!("state carrier: {error:?}"))?,
        diagnostic: 12,
        e1_diagnostic: None,
        invalid_source: None,
    });
    let mut profile = case.carrier.clone();
    profile
        .get_mut(112..116)
        .ok_or_else(|| "source profile extent".to_owned())?
        .copy_from_slice(&2u32.to_le_bytes());
    cases.push(Rejection {
        label: "obsolete-source-profile",
        carrier: profile,
        diagnostic: 12,
        e1_diagnostic: None,
        invalid_source: None,
    });
    cases.push(Rejection {
        label: "explicit-zero-proof-policy",
        carrier: raw_source::carrier_with_allowance(&raw, Some(0))
            .map_err(|error| format!("zero policy carrier: {error:?}"))?,
        diagnostic: 6,
        e1_diagnostic: None,
        invalid_source: None,
    });

    let absent = TypeObjectDigest::from_bytes([0xa5; 32]);
    ensure(
        !raw.types.iter().any(|row| row.digest == absent.bytes()),
        "chosen missing Type exists",
    )?;
    let text = case
        .source
        .types
        .iter()
        .find_map(|(digest, object)| {
            matches!(object.form, kernel::TypeForm::Text).then_some(*digest)
        })
        .ok_or_else(|| "fixture Text type absent".to_owned())?;
    for (label, form, diagnostic, e1_diagnostic) in [
        (
            "unused-Type-missing-child",
            kernel::TypeForm::List { item: absent },
            14,
            Some(14),
        ),
        (
            "unused-Type-unsupported-owned-cell",
            kernel::TypeForm::OwnedI64Cell,
            13,
            Some(13),
        ),
        (
            // This shape and endpoint closure are supported by E1; the active
            // compiler's closed List<I64> profile rejects it only at E2.
            "unused-Type-unsupported-text-list",
            kernel::TypeForm::List { item: text },
            13,
            Some(0),
        ),
    ] {
        let object =
            kernel::TypeObject::new(form).map_err(|error| format!("negative Type: {error:?}"))?;
        let (digest, image) = kernel::encode_type_object(&object)
            .map_err(|error| format!("negative Type encoding: {error:?}"))?;
        ensure(
            !raw.types.iter().any(|row| row.digest == digest.bytes()),
            "negative Type already supplied",
        )?;
        let mut types = raw.types.to_vec();
        types.push(raw_source::RawType {
            digest: digest.bytes(),
            bytes: &image,
        });
        types.sort_by_key(|row| row.digest);
        let carrier = raw_source::carrier(&raw_source::RawSourceGraph {
            types: &types,
            ..raw
        })
        .map_err(|error| format!("negative Type carrier: {error:?}"))?;
        equal_bytes(
            carrier
                .get(48..80)
                .ok_or_else(|| "negative source state extent".to_owned())?,
            case.carrier
                .get(48..80)
                .ok_or_else(|| "source state extent".to_owned())?,
            "unused supplied Type deliberately leaves semantic-state unchanged",
        )?;
        ensure(
            carrier != case.carrier,
            "unused supplied Type did not change whole carrier",
        )?;
        cases.push(Rejection {
            label,
            carrier,
            diagnostic,
            e1_diagnostic,
            invalid_source: None,
        });
    }

    let mut source = case.source.clone();
    let body = function_body(&source, "artifact-controls", "untaken")?;
    let false_arm = match operation(&source, body)? {
        E::If { when_false, .. } => *when_false,
        _ => return Err("uncalled/untaken fixture branch absent".to_owned()),
    };
    set_operation(&mut source, false_arm, E::Bool { value: false })?;
    cases.push(Rejection {
        label: "uncalled-untaken-wrong-type",
        carrier: graph::packet(&source),
        diagnostic: 19,
        e1_diagnostic: Some(0),
        invalid_source: Some(source),
    });

    let mut source = case.source.clone();
    let reference = function_named(&source, "artifact-controls", "size")?;
    let parameter = match source
        .owners
        .get(&OwnerKey::Declaration(reference.declaration))
    {
        Some(OwnerRecord::Declaration(record)) => match &record.payload {
            kernel::DeclarationPayload::External(external) => *external
                .parameters
                .first()
                .ok_or_else(|| "size parameter absent".to_owned())?,
            _ => return Err("size is not an external".to_owned()),
        },
        _ => return Err("size external missing".to_owned()),
    };
    match source.owners.get_mut(&OwnerKey::Parameter(parameter)) {
        Some(OwnerRecord::Parameter(record)) => record.use_mode = kernel::ParameterUse::Consume,
        _ => return Err("size parameter owner missing".to_owned()),
    }
    cases.push(Rejection {
        label: "borrow-intrinsic-custody-mutation",
        carrier: graph::packet(&source),
        diagnostic: 19,
        e1_diagnostic: Some(0),
        invalid_source: Some(source),
    });
    Ok(cases)
}

fn reject_requests(label: &str, image: &[u8], case: &InputCase, deadline: Duration) -> Check<()> {
    let target = target_named(&case.source, "scalar-main")?;
    for mode in [SourceOutputMode::Elf, SourceOutputMode::CanonicalArtifact] {
        let valid = raw_source::request(&case.carrier, target, mode)
            .map_err(|error| format!("negative request seed: {error:?}"))?;
        let mut obsolete = valid.clone();
        obsolete
            .get_mut(8..12)
            .ok_or_else(|| "request profile extent".to_owned())?
            .copy_from_slice(&1u32.to_le_bytes());
        let mut wrong_mode = valid.clone();
        *wrong_mode
            .get_mut(29)
            .ok_or_else(|| "request mode extent".to_owned())? = 2;
        let mut trailing = valid.clone();
        trailing.push(0);
        for (name, request) in [
            (
                "truncated-request",
                valid
                    .get(..31)
                    .ok_or_else(|| "request header missing".to_owned())?
                    .to_vec(),
            ),
            ("obsolete-request-profile", obsolete),
            ("unsupported-output-mode", wrong_mode),
            ("trailing-request-byte", trailing),
        ] {
            run_filter(
                &format!("{label}/{mode:?}/{name}"),
                image,
                &request,
                12,
                deadline,
            )?;
        }
    }
    for rejection in rejection_carriers(case)? {
        if let Some(diagnostic) = rejection.e1_diagnostic {
            // Independently establish the exact rejection layer. An unsupported
            // E1 shape is not a late E2 negative; neither result is supplied to
            // a compiler request as a certificate or prefilter.
            admission_layer_result("read-nominal", &rejection.carrier, diagnostic)?;
        }
        for mode in [SourceOutputMode::Elf, SourceOutputMode::CanonicalArtifact] {
            let request = raw_source::request(&rejection.carrier, target, mode)
                .map_err(|error| format!("negative request: {error:?}"))?;
            run_filter(
                &format!("{label}/{mode:?}/{}", rejection.label),
                image,
                &request,
                rejection.diagnostic,
                deadline,
            )?;
        }
        if let Some(source) = rejection.invalid_source {
            ensure(
                kernel::validate_full(&source).is_err(),
                "independent Rust semantic oracle accepted negative",
            )?;
        }
    }
    reject_admission_layers(label, image, target, deadline)
}

#[test]
fn selfhost_source_artifact_rejects_complete_source_and_request_negatives_in_both_modes()
-> Check<()> {
    let case = fixture_case()?;
    let image = bootstrap_compiler()?;
    reject_requests("negatives", image, &case, FOCUSED_DEADLINE)?;
    // Some(u64::MAX) remains a finite opt-in policy. This small workload fits it;
    // this does not assert that finite MAX and None are equivalent contracts.
    let original = ExportedSource::from_container(&case.container)
        .map_err(|error| format!("finite policy original source: {error:?}"))?;
    let finite = raw_source::carrier_with_allowance(&original.as_raw(), Some(u64::MAX))
        .map_err(|error| format!("finite maximum allowance: {error:?}"))?;
    ensure(
        finite != case.carrier,
        "finite maximum allowance collapsed to None framing",
    )?;
    for mode in [SourceOutputMode::Elf, SourceOutputMode::CanonicalArtifact] {
        let expected = run_filter(
            &format!("none/{mode:?}"),
            image,
            &case.request("scalar-main", mode)?,
            0,
            FOCUSED_DEADLINE,
        )?;
        let request =
            raw_source::request(&finite, target_named(&case.source, "scalar-main")?, mode)
                .map_err(|error| format!("finite maximum request: {error:?}"))?;
        let actual = run_filter(
            &format!("finite-max/{mode:?}"),
            image,
            &request,
            0,
            FOCUSED_DEADLINE,
        )?;
        equal_bytes(&actual, &expected, "finite sufficient allowance output")?;
    }
    Ok(())
}

fn same_source(actual: &InputCase, expected: &InputCase, label: &str) -> Check<()> {
    equal_bytes(
        &actual.carrier,
        &expected.carrier,
        &format!("{label}: complete raw carrier"),
    )?;
    equal_bytes(
        &actual.canonical_core,
        &expected.canonical_core,
        &format!("{label}: original RevisionCore"),
    )?;
    equal_bytes(
        &actual.revision_bytes,
        &expected.revision_bytes,
        &format!("{label}: PackageRevision"),
    )?;
    equal(
        actual.revision_digest,
        expected.revision_digest,
        &format!("{label}: PackageRevision ID"),
    )?;
    equal(
        &actual.revision,
        &expected.revision,
        &format!("{label}: exact public revision"),
    )?;
    equal(
        actual.source.owners.len(),
        expected.source.owners.len(),
        &format!("{label}: all owners"),
    )?;
    equal(
        actual.source.types.len(),
        expected.source.types.len(),
        &format!("{label}: all supplied Types"),
    )
}

fn compiler_diagnostic_mutation(source: &KernelSnapshot) -> Check<InputCase> {
    let mut changed = source.clone();
    let body = function_body(&changed, "source-elf", "failure")?;
    let diagnostic = match operation(&changed, body)? {
        E::Record { fields, .. } => fields
            .iter()
            .find_map(|field| match &field.selector {
                kernel::FieldSelector::Structural(name) if name.as_str() == "diagnostic" => {
                    Some(field.value)
                }
                _ => None,
            })
            .ok_or_else(|| "compiler failure diagnostic field absent".to_owned())?,
        _ => {
            return Err(
                "compiler failure body changed; refresh this explicit source mutation".to_owned(),
            );
        }
    };
    ensure(
        matches!(operation(&changed, diagnostic)?, E::Local { .. }),
        "compiler failure diagnostic was not the expected source Local",
    )?;
    set_operation(&mut changed, diagnostic, E::I64 { value: 77 })?;
    // Public creation admits this whole intentional S' proposal. Only S' receives
    // fresh test history; exact maintained S was copied byte-for-byte above.
    case_from_snapshot(&changed)
}

fn cheap_compiler_diagnostic_oracles(
    case: &InputCase,
    host: &HostOracle,
    expected: i64,
) -> Check<()> {
    let entry = function_named(&case.source, "source-elf", "compile")?;
    let request = vec![NormalizedValue::bytes(b"invalid request".as_slice())];
    let policy = NormalizedRunPolicy::foreground();
    let control = ExecutionControl::uncancelled();
    let vm = NormalizedVm::for_test(&host.program, policy)
        .invoke(entry, request.clone(), None, &control)
        .map_err(|error| format!("compiler mutation VM oracle: {error:?}"))?;
    let reference = NormalizedReferenceInterpreter::new(&case.source, &host.program, policy)
        .invoke(entry, request, None, &control)
        .map_err(|error| format!("compiler mutation reference oracle: {error:?}"))?;
    equal(
        filter_value(&vm.0)?,
        (expected, &[][..]),
        "compiler mutation independent VM diagnostic",
    )?;
    equal(
        filter_value(&reference.0)?,
        (expected, &[][..]),
        "compiler mutation canonical reference diagnostic",
    )?;
    equal(
        vm.1.live_call_frames_after,
        0,
        "compiler mutation VM frames reclaimed",
    )?;
    equal(
        reference.1.live_call_frames_after,
        0,
        "compiler mutation reference frames reclaimed",
    )
}

fn stage_behavior_controls(c2: &[u8], c3: &[u8]) -> Check<()> {
    let case = fixture_case()?;
    let changed = mutated_case(&case.source)?;
    let baseline2 = raw_outputs(
        "C2-independent",
        c2,
        &case,
        "scalar-main",
        WHOLE_SOURCE_DEADLINE,
    )?;
    let baseline3 = raw_outputs(
        "C3-independent",
        c3,
        &case,
        "scalar-main",
        WHOLE_SOURCE_DEADLINE,
    )?;
    let changed2 = raw_outputs(
        "C2-independent-mutated",
        c2,
        &changed,
        "scalar-main",
        WHOLE_SOURCE_DEADLINE,
    )?;
    let changed3 = raw_outputs(
        "C3-independent-mutated",
        c3,
        &changed,
        "scalar-main",
        WHOLE_SOURCE_DEADLINE,
    )?;
    equal_bytes(
        &baseline2.elf,
        &baseline3.elf,
        "C2/C3 independent program ELF",
    )?;
    equal_bytes(
        &baseline2.artifact,
        &baseline3.artifact,
        "C2/C3 independent program artifact",
    )?;
    equal_bytes(
        &changed2.elf,
        &changed3.elf,
        "C2/C3 mutated independent program ELF",
    )?;
    equal_bytes(
        &changed2.artifact,
        &changed3.artifact,
        "C2/C3 mutated independent program artifact",
    )?;
    ensure(
        baseline2.elf != changed2.elf && baseline2.artifact != changed2.artifact,
        "independent 42-to-43 source mutation did not alter generated output",
    )?;
    // Both raw-source generations have finished before independent target oracles.
    let host = host_oracle(&case)?;
    let changed_host = host_oracle(&changed)?;
    for (label, output) in [("C2", baseline2), ("C3", baseline3)] {
        let loaded = strict_artifact(
            &format!("{label}-independent"),
            &output.artifact,
            &case,
            &host,
        )?;
        scalar_behavior(
            &format!("{label}-independent"),
            &case,
            &host,
            loaded,
            &output.elf,
            ("entry", 42),
            WHOLE_SOURCE_DEADLINE,
        )?;
    }
    for (label, output) in [("C2", changed2), ("C3", changed3)] {
        let loaded = strict_artifact(
            &format!("{label}-mutated"),
            &output.artifact,
            &changed,
            &changed_host,
        )?;
        scalar_behavior(
            &format!("{label}-mutated"),
            &changed,
            &changed_host,
            loaded,
            &output.elf,
            ("entry", 43),
            WHOLE_SOURCE_DEADLINE,
        )?;
    }
    reject_requests("C2-layer-negatives", c2, &case, WHOLE_SOURCE_DEADLINE)?;
    reject_requests("C3-layer-negatives", c3, &case, WHOLE_SOURCE_DEADLINE)?;
    stage_controls::verify("C2", c2, "C3", c3)
}

fn maintained_fixed_point(case: &InputCase) -> Check<()> {
    let start = Instant::now();
    let elf_request = case.request("compile-source-elf", SourceOutputMode::Elf)?;
    let artifact_request =
        case.request("compile-source-elf", SourceOutputMode::CanonicalArtifact)?;
    // C0 is the sole bootstrap: these writes target the exact isolated repository
    // copy. Requests already bind the original S, core, revision, all owners and
    // all supplied Types. C2/C3 receive no derived C0 data, files or environment.
    let host = host_oracle(case)?;
    let c1 = native_image(
        &host.program,
        function_named(&case.source, "source-elf", "compile")?,
    )?;
    let c2 = run_filter("C2=C1(S,E)", &c1, &elf_request, 0, WHOLE_SOURCE_DEADLINE)?;
    let c3 = run_filter("C3=C2(S,E)", &c2, &elf_request, 0, WHOLE_SOURCE_DEADLINE)?;
    equal_bytes(
        &c2,
        &c3,
        "complete raw-source compiler native fixed point C2=C3",
    )?;
    let a1 = run_filter(
        "A1=C1(S,lkja)",
        &c1,
        &artifact_request,
        0,
        WHOLE_SOURCE_DEADLINE,
    )?;
    let a2 = run_filter(
        "A2=C2(S,lkja)",
        &c2,
        &artifact_request,
        0,
        WHOLE_SOURCE_DEADLINE,
    )?;
    let a3 = run_filter(
        "A3=C3(S,lkja)",
        &c3,
        &artifact_request,
        0,
        WHOLE_SOURCE_DEADLINE,
    )?;
    for (label, bytes) in [("A1", &a1), ("A2", &a2), ("A3", &a3)] {
        // Each generation independently re-enters the strict loader and complete
        // unit/provenance inventory. Equality with another generation is not a seal.
        strict_artifact(label, bytes, case, &host)?;
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        phase::verify_compiler(
            "C1-Rust",
            &c1,
            &artifact_request,
            &host.bytes,
            &elf_request,
            &c2,
        )?;
        phase::verify_compiler(
            "C2-source",
            &c2,
            &artifact_request,
            &host.bytes,
            &elf_request,
            &c3,
        )?;
    }
    cheap_compiler_diagnostic_oracles(case, &host, 12)?;
    stage_behavior_controls(&c2, &c3)?;

    let mutant = compiler_diagnostic_mutation(&case.source)?;
    ensure(
        mutant.carrier != case.carrier,
        "compiler S' raw source unchanged",
    )?;
    let mutant2 = raw_outputs(
        "C2-compiler-S-prime",
        &c2,
        &mutant,
        "compile-source-elf",
        WHOLE_SOURCE_DEADLINE,
    )?;
    let mutant3 = raw_outputs(
        "C3-compiler-S-prime",
        &c3,
        &mutant,
        "compile-source-elf",
        WHOLE_SOURCE_DEADLINE,
    )?;
    equal_bytes(
        &mutant2.elf,
        &mutant3.elf,
        "C2/C3 compiler-source mutation ELF",
    )?;
    equal_bytes(
        &mutant2.artifact,
        &mutant3.artifact,
        "C2/C3 compiler-source mutation artifact",
    )?;
    ensure(
        mutant2.elf != c2 && mutant2.artifact != a2,
        "compiler-source mutation did not affect outputs",
    )?;
    let mutant_host = host_oracle(&mutant)?;
    strict_artifact("S-prime-via-C2", &mutant2.artifact, &mutant, &mutant_host)?;
    strict_artifact("S-prime-via-C3", &mutant3.artifact, &mutant, &mutant_host)?;
    cheap_compiler_diagnostic_oracles(&mutant, &mutant_host, 77)?;
    for (label, image, diagnostic) in [
        ("original-C2", c2.as_slice(), 12),
        ("original-C3", c3.as_slice(), 12),
        ("mutant-via-C2", mutant2.elf.as_slice(), 77),
        ("mutant-via-C3", mutant3.elf.as_slice(), 77),
    ] {
        run_filter(
            &format!("compiler-source-behavior/{label}"),
            image,
            b"invalid request",
            diagnostic,
            WHOLE_SOURCE_DEADLINE,
        )?;
    }
    eprintln!(
        "raw source maintained fixed point inner_equations_and_behavior_checked=true original_recheck=pending source_carrier_sha256={} \
        elf_request_sha256={} artifact_request_sha256={} C1_sha256={} C2_sha256={} C3_sha256={} \
        Ahost_sha256={} A1_sha256={} A2_sha256={} A3_sha256={} \
        mutant_carrier_sha256={} mutant_elf_sha256={} mutant_artifact_sha256={} \
        independent_behavior=42,43 compiler_mutation_diagnostic=77 elapsed_ms={} \
        input_boundary=complete-raw-source request_profile=2 source_profile=3 environment=empty \
        full_P_self=false same_process_reclamation={}",
        sha256(&case.carrier),
        sha256(&elf_request),
        sha256(&artifact_request),
        sha256(&c1),
        sha256(&c2),
        sha256(&c3),
        sha256(&host.bytes),
        sha256(&a1),
        sha256(&a2),
        sha256(&a3),
        sha256(&mutant.carrier),
        sha256(&mutant2.elf),
        sha256(&mutant2.artifact),
        start.elapsed().as_millis(),
        cfg!(all(target_os = "linux", target_arch = "x86_64"))
    );
    Ok(())
}

#[test]
#[ignore = "explicit maintained compiler acceptance: raw C1/C2/C3, artifacts, behavior and native phases; run separately"]
fn selfhost_source_artifact_whole_maintained_raw_source_fixed_point() -> Check<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/selfhost/project");
    // Public opens can repair a damaged lock/catalog. Capture the healthy,
    // quiescent original before any such call and reject a repair before C0.
    let copy = RepositoryCopy::create(&root)?;
    let first_read = (|| {
        InputCase::from_repository(
            GraphRepository::open(&root)
                .map_err(|error| format!("maintained repository public open: {error:?}"))?,
        )
    })();
    let original = match (first_read, copy.verify_original(&root)) {
        (Ok(original), Ok(())) => original,
        (read, integrity) => {
            return Err(format!(
                "initial original read={} integrity={integrity:?}",
                match read {
                    Ok(_) => "passed".to_owned(),
                    Err(error) => error,
                }
            ));
        }
    };

    // Keep every ordinary inner failure as Result so original verification runs
    // on success, semantic failure and native cancellation/deadline paths alike.
    let outcome = (|| {
        let copied = InputCase::from_repository(
            GraphRepository::open(&copy.path)
                .map_err(|error| format!("isolated repository open: {error:?}"))?,
        )?;
        same_source(&copied, &original, "original/copy before C0")?;
        maintained_fixed_point(&copied)
    })();
    let unchanged_source = (|| {
        let after = InputCase::from_repository(
            GraphRepository::open(&root)
                .map_err(|error| format!("maintained repository final read: {error:?}"))?,
        )?;
        same_source(&after, &original, "original after fixed-point operation")
    })();
    // This is deliberately last: a final public read must not repair a catalog
    // after the byte/directory inventory was already declared unchanged.
    let unchanged_files = copy.verify_original(&root);
    // Preserve the original failure too if repository invariants also fail.
    match (outcome, unchanged_files, unchanged_source) {
        (Ok(()), Ok(()), Ok(())) => {
            eprintln!(
                "raw source maintained fixed_point_component_passed=true original_repository_unchanged=true \
                exact_source_carrier_sha256={} original_core_sha256={} package_revision={} \
                full_P_self=false same_process_reclamation={}",
                sha256(&original.carrier),
                sha256(&original.canonical_core),
                original.revision_digest,
                cfg!(all(target_os = "linux", target_arch = "x86_64"))
            );
            Ok(())
        }
        (outcome, files, source) => Err(format!(
            "maintained acceptance outcome={outcome:?}; original_file_inventory={files:?}; original_public_source={source:?}"
        )),
    }
}

/// Optional measurement export, not a fixed-point or semantic acceptance result.
/// The original repository is checked even when preparing the measurement fails.
#[test]
#[ignore = "explicit native source profiling export; requires an absent LKJSCRIPT_SELFHOST_NATIVE_PROFILE_DIR"]
fn selfhost_source_native_profile_export() -> Check<()> {
    use std::io::Write;
    let destination = std::env::var_os("LKJSCRIPT_SELFHOST_NATIVE_PROFILE_DIR")
        .ok_or_else(|| "profiling directory was not explicitly selected".to_owned())?;
    let destination = std::path::PathBuf::from(destination);
    std::fs::create_dir(&destination).map_err(|error| format!("profile directory: {error}"))?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/selfhost/project");
    let copy = RepositoryCopy::create(&root)?;
    let operation = (|| {
        let case = InputCase::from_repository(
            GraphRepository::open(&copy.path)
                .map_err(|error| format!("profile source: {error:?}"))?,
        )?;
        let host = host_oracle(&case)?;
        let image = native_image(
            &host.program,
            function_named(&case.source, "source-elf", "compile")?,
        )?;
        let request = case.request("compile-source-elf", SourceOutputMode::CanonicalArtifact)?;
        crate::platform::owned_output::publish_executable_create_new(
            &destination.join("compiler"),
            &image,
            None,
            "explicit source profiling compiler",
        )
        .map_err(|error| format!("profile executable: {error:?}"))?;
        for (name, bytes) in [
            ("artifact-request.bin", request.as_slice()),
            ("host.lkja", host.bytes.as_slice()),
        ] {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(destination.join(name))
                .map_err(|error| format!("profile {name}: {error}"))?;
            file.write_all(bytes)
                .map_err(|error| format!("profile {name}: {error}"))?;
        }
        let mut declarations = Vec::new();
        for (key, owner) in &case.source.owners {
            if let (OwnerKey::Declaration(id), OwnerRecord::Declaration(record)) = (key, owner) {
                let module = match case.source.owners.get(&OwnerKey::Module(record.module)) {
                    Some(OwnerRecord::Module(module)) => module.name.as_str(),
                    _ => return Err("profile declaration module absent".to_owned()),
                };
                declarations.push(serde_json::json!({"id": id.to_string(),
                    "name": format!("{module}::{}", record.name.as_str())}));
            }
        }
        let report = serde_json::json!({"schema":"lkjscript.native-source-profile-input.v1",
            "scope":"measurement inputs only; no execution or fixed-point claim",
            "compiler_sha256":sha256(&image), "compiler_bytes":image.len(),
            "request_sha256":sha256(&request), "request_bytes":request.len(),
            "carrier_sha256":sha256(&case.carrier), "host_artifact_sha256":sha256(&host.bytes),
            "declarations":declarations});
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination.join("inputs.json"))
            .map_err(|error| format!("profile inputs: {error}"))?;
        serde_json::to_writer_pretty(&mut file, &report)
            .map_err(|error| format!("profile receipt: {error}"))?;
        eprintln!(
            "native source profile compiler_sha256={} request_sha256={} input_bytes={} output_directory={}",
            sha256(&image),
            sha256(&request),
            request.len(),
            destination.display()
        );
        Ok(())
    })();
    let unchanged = copy.verify_original(&root);
    match (operation, unchanged) {
        (Ok(()), Ok(())) => Ok(()),
        (operation, unchanged) => Err(format!(
            "profile operation={operation:?}; original={unchanged:?}"
        )),
    }
}
