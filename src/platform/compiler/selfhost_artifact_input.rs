//! Test-only semantic unit projection for Tier 1 artifact emission.
//! This supplies fields and canonical pre-tail instructions, never serialized units,
//! derived unit keys, envelope bytes, or executable templates.
use super::unit::{
    CompilationPayload, CompilationUnit, CompiledCode, CompiledFieldSelector,
    CompiledInstruction as I, CompiledPortImplementation, CompiledSignature, CompiledText,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{FunctionEffect, OwnerKey, OwnerKind, ParameterUse, ResultRelation};
use crate::platform::package::RunnerKind;
use crate::platform::storage::object::ObjectKey;

pub(crate) const ARTIFACT_PROFILE: u32 = 7;
const HEADER_BYTES: usize = 160;
const UNIT_DIRECTORY_ROW_BYTES: usize = 16;
const SOURCE_OBJECT_ROW_BYTES: usize = 56;
const METADATA_BYTES: usize = 312;
const RUNTIME_OWNER_ROW_BYTES: usize = 56;
const UNIT_HEADER_BYTES: usize = 272;
const UNIT_TABLE_BYTES: usize = 160;
const MINIMUM_PROJECTED_UNIT_BYTES: u64 = 432;
const SIGNATURE_BYTES: usize = 40;
const PARAMETER_BYTES: usize = 48;
const CODE_BYTES: usize = 32;
const INSTRUCTION_BYTES: usize = 48;

fn unsupported(what: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Source,
        "compiler_artifact_input_unsupported",
        format!("artifact unit projection does not support {what}"),
    )
}

fn admit_bundle_counts(units: usize, objects: usize) -> Result<(), Diagnostic> {
    if units == 0 {
        return Err(unsupported("unit count"));
    }
    if objects == 0 {
        return Err(unsupported("source-object count"));
    }
    let objects_at = projection_extent(HEADER_BYTES, units, UNIT_DIRECTORY_ROW_BYTES)?;
    projection_extent(objects_at, objects, SOURCE_OBJECT_ROW_BYTES)?;
    Ok(())
}

/// Every extent must fit the host index and the unsigned 64-bit wire representation.
/// This proves arithmetic before reservation; it is independent of caller policy.
fn projection_extent(start: usize, count: usize, stride: usize) -> Result<usize, Diagnostic> {
    let end = count
        .checked_mul(stride)
        .and_then(|size| start.checked_add(size))
        .ok_or_else(|| unsupported("projection extent arithmetic"))?;
    u64::try_from(end).map_err(|_| unsupported("unsigned 64-bit projection extent"))?;
    Ok(end)
}

struct Output {
    bytes: Vec<u8>,
    measured_length: usize,
}

/// The same emitter measures and fills. Measurement has no output allocation, and
/// the fill pass cannot grow beyond the exact length admitted before its allocation.
#[derive(Default)]
struct Projection {
    length: usize,
    output: Option<Output>,
}

impl Projection {
    fn writing(measured_length: usize) -> Result<Self, Diagnostic> {
        projection_extent(0, measured_length, 1)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(measured_length)
            .map_err(|_| unsupported("host allocation for projection"))?;
        Ok(Self {
            length: 0,
            output: Some(Output {
                bytes,
                measured_length,
            }),
        })
    }

    fn finish(self) -> Result<Vec<u8>, Diagnostic> {
        let output = self
            .output
            .ok_or_else(|| unsupported("output from measurement-only projection"))?;
        if self.length != output.measured_length || output.bytes.len() != self.length {
            return Err(unsupported("projection measurement agreement"));
        }
        Ok(output.bytes)
    }

    fn reserve(&mut self, size: usize) -> Result<usize, Diagnostic> {
        let at = self.length;
        let end = projection_extent(at, size, 1)?;
        if let Some(output) = &mut self.output {
            if end > output.measured_length {
                return Err(unsupported("projection exceeded its measured extent"));
            }
            // The complete capacity was reserved after admission, before this pass.
            output.bytes.resize(end, 0);
        }
        self.length = end;
        Ok(at)
    }

    fn rows(&mut self, count: usize, stride: usize) -> Result<usize, Diagnostic> {
        self.reserve(projection_extent(0, count, stride)?)
    }

    fn put(&mut self, at: usize, bytes: &[u8]) -> Result<(), Diagnostic> {
        let end = projection_extent(at, bytes.len(), 1)?;
        if end > self.length {
            return Err(unsupported("projection field outside reserved storage"));
        }
        if let Some(output) = &mut self.output {
            output
                .bytes
                .get_mut(at..end)
                .ok_or_else(|| unsupported("projection field outside reserved storage"))?
                .copy_from_slice(bytes);
        }
        Ok(())
    }

    fn append(&mut self, bytes: &[u8]) -> Result<usize, Diagnostic> {
        let at = self.reserve(bytes.len())?;
        self.put(at, bytes)?;
        Ok(at)
    }

    fn tag(&mut self, at: usize, value: impl TryInto<u32>) -> Result<(), Diagnostic> {
        let value = value
            .try_into()
            .map_err(|_| unsupported("unsigned 32-bit projection tag"))?;
        self.put(at, &value.to_le_bytes())
    }

    fn word(&mut self, at: usize, value: impl TryInto<u64>) -> Result<(), Diagnostic> {
        let value = value
            .try_into()
            .map_err(|_| unsupported("unsigned 64-bit projection field"))?;
        self.put(at, &value.to_le_bytes())
    }

    fn field(
        &mut self,
        base: usize,
        offset: usize,
        value: impl TryInto<u64>,
    ) -> Result<(), Diagnostic> {
        self.word(projection_extent(base, offset, 1)?, value)
    }

    fn relative(&mut self, at: usize, origin: usize, offset: usize) -> Result<(), Diagnostic> {
        let offset = offset
            .checked_sub(origin)
            .ok_or_else(|| unsupported("projection pointer before its unit origin"))?;
        self.word(at, offset)
    }

    fn span(&mut self, at: usize, origin: usize, bytes: &[u8]) -> Result<(), Diagnostic> {
        let offset = self.append(bytes)?;
        self.relative(at, origin, offset)?;
        self.field(at, 8, bytes.len())
    }

    fn fixed<const WIDTH: usize>(
        &mut self,
        at: usize,
        origin: usize,
        values: impl ExactSizeIterator<Item = Result<[u8; WIDTH], Diagnostic>>,
    ) -> Result<(), Diagnostic> {
        let count = values.len();
        let offset = self.rows(count, WIDTH)?;
        self.relative(at, origin, offset)?;
        self.field(at, 8, count)?;
        for (i, value) in values.enumerate() {
            self.put(projection_extent(offset, i, WIDTH)?, &value?)?;
        }
        Ok(())
    }

    fn numbers(&mut self, at: usize, origin: usize, numbers: &[u32]) -> Result<(), Diagnostic> {
        self.fixed(
            at,
            origin,
            numbers
                .iter()
                .map(|value| Ok(u64::from(*value).to_le_bytes())),
        )
    }

    fn strings<'a>(
        &mut self,
        at: usize,
        origin: usize,
        values: impl ExactSizeIterator<Item = Result<&'a str, Diagnostic>>,
    ) -> Result<(), Diagnostic> {
        let count = values.len();
        let offset = self.rows(count, 16)?;
        self.relative(at, origin, offset)?;
        self.field(at, 8, count)?;
        for (i, value) in values.enumerate() {
            self.span(projection_extent(offset, i, 16)?, origin, value?.as_bytes())?;
        }
        Ok(())
    }

    fn signature(
        &mut self,
        origin: usize,
        signature: &CompiledSignature,
    ) -> Result<usize, Diagnostic> {
        if !signature.implementation_parameters.is_empty()
            || !signature.requirement_parameters.is_empty()
            || !signature.effect_parameters.is_empty()
            || signature.effect != FunctionEffect::Pure
            || !signature.read_scope_parameters.is_empty()
            || !signature.read_scope_sources.is_empty()
            || !signature.task_requirements.is_empty()
            || signature.result.relation != ResultRelation::Value
            || signature.type_parameter_constraints.len() != signature.type_parameters.len()
            || signature
                .type_parameter_constraints
                .iter()
                .any(|constraint| {
                    *constraint != crate::platform::kernel::TypeParameterConstraints::None
                })
        {
            return Err(unsupported(
                "signature witnesses, effects, regions, or constraints",
            ));
        }
        let at = self.reserve(SIGNATURE_BYTES)?;
        self.fixed(
            at,
            origin,
            signature.type_parameters.iter().map(|id| Ok(id.bytes())),
        )?;
        let parameters = self.rows(signature.parameters.len(), PARAMETER_BYTES)?;
        self.relative(projection_extent(at, 16, 1)?, origin, parameters)?;
        self.field(at, 24, signature.parameters.len())?;
        self.field(at, 32, signature.result.ty)?;
        for (i, parameter) in signature.parameters.iter().enumerate() {
            if parameter.resource_requirement.is_some() {
                return Err(unsupported("parameter resources"));
            }
            let p = projection_extent(parameters, i, PARAMETER_BYTES)?;
            self.put(p, &parameter.parameter.bytes())?;
            self.span(
                projection_extent(p, 16, 1)?,
                origin,
                parameter.name.as_str().as_bytes(),
            )?;
            self.field(p, 32, parameter.ty)?;
            self.field(p, 40, parameter_use(parameter.use_mode))?;
        }
        Ok(at)
    }

    fn code(&mut self, origin: usize, code: &CompiledCode) -> Result<usize, Diagnostic> {
        if code.parameter_count > code.local_count || code.instructions.is_empty() {
            return Err(unsupported("code parameter or instruction count"));
        }
        let at = self.reserve(CODE_BYTES)?;
        self.word(at, code.parameter_count)?;
        self.field(at, 8, code.local_count)?;
        let nodes = self.rows(code.instructions.len(), INSTRUCTION_BYTES)?;
        self.relative(projection_extent(at, 16, 1)?, origin, nodes)?;
        self.field(at, 24, code.instructions.len())?;
        for (i, instruction) in code.instructions.iter().enumerate() {
            let n = projection_extent(nodes, i, INSTRUCTION_BYTES)?;
            let (tag, a, b): (u64, u64, u64) = match instruction {
                I::Unit => (2, 0, 0),
                I::Bool(value) => (3, u64::from(*value), 0),
                I::I64(value) => {
                    self.put(projection_extent(n, 24, 1)?, &value.to_le_bytes())?;
                    (4, 0, 0)
                }
                I::Text(index) => (5, u64::from(*index), 0),
                I::StaticText(index) => (6, u64::from(*index), 0),
                I::LoadLocal { local, use_mode } => {
                    (7, u64::from(*local), parameter_use(*use_mode))
                }
                I::StoreLocal(local) => (8, u64::from(*local), 0),
                I::Drop => (9, 0, 0),
                I::JumpIfFalse(target) => (10, u64::from(*target), 0),
                I::Jump(target) => (11, u64::from(*target), 0),
                I::Call {
                    requirement_arguments,
                    effect_arguments,
                    function,
                    type_arguments,
                    arguments,
                    read_scope_arguments,
                } if requirement_arguments.is_empty()
                    && effect_arguments.is_empty()
                    && read_scope_arguments.is_empty() =>
                {
                    self.numbers(projection_extent(n, 32, 1)?, origin, type_arguments)?;
                    (12, u64::from(*function), u64::from(*arguments))
                }
                I::Record {
                    nominal_type: None,
                    type_arguments,
                    fields,
                } if type_arguments.is_empty() => {
                    self.fixed(
                        projection_extent(n, 32, 1)?,
                        origin,
                        fields.iter().map(|field| match field {
                            CompiledFieldSelector::Structural(index) => {
                                Ok(u64::from(*index).to_le_bytes())
                            }
                            _ => Err(unsupported("nominal record field")),
                        }),
                    )?;
                    (15, 0, 0)
                }
                I::Field(CompiledFieldSelector::Structural(index)) => (17, u64::from(*index), 0),
                I::List { item_type, items } => (18, u64::from(*item_type), u64::from(*items)),
                I::Return => (27, 0, 0),
                _ => {
                    return Err(unsupported(&format!(
                        "canonical instruction {instruction:?}"
                    )));
                }
            };
            self.word(n, tag)?;
            self.field(n, 8, a)?;
            self.field(n, 16, b)?;
        }
        Ok(at)
    }
}

fn parameter_use(value: ParameterUse) -> u64 {
    match value {
        ParameterUse::Unrestricted => 0,
        ParameterUse::Borrow => 1,
        ParameterUse::Consume => 2,
    }
}

fn project_unit(p: &mut Projection, unit: &CompilationUnit) -> Result<(), Diagnostic> {
    let source = &unit.source;
    if source.test.is_some()
        || (
            unit.contract_version,
            unit.graph_contract_version,
            unit.bytecode_contract_version,
        ) != (1, 1, 1)
    {
        return Err(unsupported("test source or foreign contract"));
    }
    let owner: u32 = match source.owner {
        OwnerKey::Declaration(_) => 1,
        OwnerKey::Target(_) => 11,
        _ => return Err(unsupported("unit owner")),
    };
    let kind: u32 = match source.kind {
        OwnerKind::External => 4,
        OwnerKind::PureFunction => 5,
        OwnerKind::Component => 8,
        OwnerKind::Target => 19,
        _ => return Err(unsupported("unit kind")),
    };
    let origin = p.reserve(UNIT_HEADER_BYTES)?;
    p.tag(origin, owner)?;
    p.tag(projection_extent(origin, 4, 1)?, kind)?;
    p.tag(
        projection_extent(origin, 8, 1)?,
        u32::from(source.kind.tag()),
    )?;
    p.put(projection_extent(origin, 16, 1)?, &source.package.bytes())?;
    p.put(projection_extent(origin, 32, 1)?, &source.owner.bytes())?;
    for (i, digest) in [
        source.semantic_interface,
        source.implementation,
        source.type_digest,
        source.effect,
        source.capability,
        source.validation_dependencies,
    ]
    .iter()
    .enumerate()
    {
        let at = projection_extent(projection_extent(origin, 48, 1)?, i, 32)?;
        p.put(at, &digest.bytes())?;
    }
    let tables = p.reserve(UNIT_TABLE_BYTES)?;
    p.relative(projection_extent(origin, 240, 1)?, origin, tables)?;
    macro_rules! references {
        ($index:expr, $values:expr, $field:ident) => {{
            let values = &$values;
            p.fixed(
                projection_extent(tables, $index, 16)?,
                origin,
                values.iter().map(|value| {
                    let mut bytes = [0; 32];
                    bytes[..16].copy_from_slice(&value.package.bytes());
                    bytes[16..].copy_from_slice(&value.$field.bytes());
                    Ok(bytes)
                }),
            )?;
        }};
    }
    references!(0, unit.tables.declarations, declaration);
    references!(1, unit.tables.fields, field);
    references!(2, unit.tables.cases, case);
    references!(3, unit.tables.requirements, requirement);
    references!(4, unit.tables.operations, operation);
    references!(5, unit.tables.ports, port);
    p.fixed(
        projection_extent(tables, 6, 16)?,
        origin,
        unit.tables.types.iter().map(|ty| Ok(ty.bytes())),
    )?;
    p.strings(
        projection_extent(tables, 7, 16)?,
        origin,
        unit.tables
            .structural_names
            .iter()
            .map(|name| Ok(name.as_str())),
    )?;
    if !unit.tables.read_view_fields.is_empty() {
        return Err(unsupported("read-view fields"));
    }
    p.strings(
        projection_extent(tables, 9, 16)?,
        origin,
        unit.tables.texts.iter().map(|text| match text {
            CompiledText::Inline(text) => Ok(text.as_str()),
            CompiledText::Blob { .. } => Err(unsupported("blob-backed text")),
        }),
    )?;
    let (tag, payload): (u32, usize) = match &unit.payload {
        CompilationPayload::External {
            signature,
            implementation,
        } => {
            let signature = p.signature(origin, signature)?;
            p.relative(projection_extent(origin, 248, 1)?, origin, signature)?;
            let at = p.reserve(16)?;
            p.span(at, origin, implementation.as_str().as_bytes())?;
            (6, at)
        }
        CompilationPayload::Function { signature, code } => {
            if usize::try_from(code.parameter_count).ok() != Some(signature.parameters.len()) {
                return Err(unsupported("code/signature parameter agreement"));
            }
            let signature = p.signature(origin, signature)?;
            p.relative(projection_extent(origin, 248, 1)?, origin, signature)?;
            (7, p.code(origin, code)?)
        }
        CompilationPayload::Component {
            requirements,
            ports,
        } if requirements.is_empty() => {
            let at = p.reserve(16)?;
            let offset = p.rows(ports.len(), 32)?;
            p.relative(at, origin, offset)?;
            p.field(at, 8, ports.len())?;
            for (i, port) in ports.iter().enumerate() {
                let CompiledPortImplementation::Function(function) = port.implementation else {
                    return Err(unsupported("expression port"));
                };
                let row = projection_extent(offset, i, 32)?;
                p.word(row, port.port)?;
                p.field(row, 8, port.function_type)?;
                p.field(row, 24, function)?;
            }
            (9, at)
        }
        CompilationPayload::Target {
            component,
            port,
            routes,
            runner: RunnerKind::Command,
        } if routes.is_empty() => {
            let at = p.reserve(32)?;
            p.word(at, *component)?;
            p.field(at, 8, u64::from(port.is_some()))?;
            p.field(at, 16, port.unwrap_or(0))?;
            (11, at)
        }
        _ => return Err(unsupported("unit payload")),
    };
    p.tag(projection_extent(origin, 256, 1)?, tag)?;
    p.relative(projection_extent(origin, 264, 1)?, origin, payload)?;
    Ok(())
}

/// This projection does not change canonical-unit admission or its semantic indices.
/// All such indices arrive in their current typed u32 representation and are widened
/// losslessly on wire; pointer and sequence extents have independent u64 fields.
pub(crate) fn unit(unit: &CompilationUnit) -> Result<Vec<u8>, Diagnostic> {
    let mut measured = Projection::default();
    project_unit(&mut measured, unit)?;
    let mut output = Projection::writing(measured.length)?;
    project_unit(&mut output, unit)?;
    output.finish()
}

struct SourcePages<'a> {
    artifact: &'a super::LoadedArtifact,
    read:
        std::cell::RefCell<std::collections::BTreeSet<crate::platform::persistent_map::PageDigest>>,
}
impl crate::platform::persistent_map::PageStore for SourcePages<'_> {
    fn read_page(
        &self,
        digest: crate::platform::persistent_map::PageDigest,
        maximum_bytes: usize,
    ) -> Result<Option<Vec<u8>>, crate::platform::persistent_map::MapError> {
        self.read.borrow_mut().insert(digest);
        crate::platform::persistent_map::PageStore::read_page(
            &crate::platform::storage::page_store::ObjectPageReader::new(self.artifact),
            digest,
            maximum_bytes,
        )
    }
    fn write_page(
        &mut self,
        _digest: crate::platform::persistent_map::PageDigest,
        _bytes: &[u8],
    ) -> Result<crate::platform::persistent_map::PageWrite, crate::platform::persistent_map::MapError>
    {
        Err(crate::platform::persistent_map::MapError {
            class: crate::platform::persistent_map::MapErrorClass::Store,
            code: "selfhost_projection_read_only",
            message: "artifact projection has no write authority".to_owned(),
        })
    }
}

fn metadata(p: &mut Projection, package: &super::ArtifactPackage) -> Result<usize, Diagnostic> {
    let base = p.reserve(METADATA_BYTES)?;
    let owners = p.rows(package.runtime_owners.len(), RUNTIME_OWNER_ROW_BYTES)?;
    p.put(base, &package.repository_id.bytes())?;
    p.put(projection_extent(base, 16, 1)?, &package.package.bytes())?;
    for (i, digest) in [
        package.package_revision.bytes(),
        package.semantic_revision.bytes(),
        package.semantic_state.bytes(),
        package.interface.bytes(),
    ]
    .iter()
    .enumerate()
    {
        let at = projection_extent(projection_extent(base, 32, 1)?, i, 32)?;
        p.put(at, digest)?;
    }
    for (i, root) in [package.interface_owners, package.reference_owners]
        .iter()
        .enumerate()
    {
        let at = projection_extent(projection_extent(base, 160, 1)?, i, 72)?;
        p.put(at, &root.page().bytes())?;
        p.field(at, 32, root.entries())?;
        p.put(projection_extent(at, 40, 1)?, &root.content().bytes())?;
    }
    p.field(base, 304, package.runtime_owners.len())?;
    for (i, owner) in package.runtime_owners.iter().enumerate() {
        let at = projection_extent(owners, i, RUNTIME_OWNER_ROW_BYTES)?;
        let ordinal = crate::platform::kernel::EncodedOwnerKey::new(owner.owner).bytes()[0]
            .checked_sub(1)
            .ok_or_else(|| unsupported("runtime-owner ordinal"))?;
        let kind = owner
            .kind
            .tag()
            .checked_sub(1)
            .ok_or_else(|| unsupported("runtime-owner kind"))?;
        p.tag(at, u32::from(ordinal))?;
        p.tag(projection_extent(at, 4, 1)?, u32::from(kind))?;
        p.put(projection_extent(at, 8, 1)?, &owner.owner.bytes())?;
        p.put(projection_extent(at, 24, 1)?, &owner.object.bytes())?;
    }
    Ok(base)
}

fn project_bundle(
    p: &mut Projection,
    package: &super::ArtifactPackage,
    units: &[CompilationUnit],
    source: &[(ObjectKey, &[u8])],
    allowance: Option<u64>,
) -> Result<(), Diagnostic> {
    admit_bundle_counts(units.len(), source.len())?;
    p.reserve(HEADER_BYTES)?;
    let directory = p.rows(units.len(), UNIT_DIRECTORY_ROW_BYTES)?;
    let rows = p.rows(source.len(), SOURCE_OBJECT_ROW_BYTES)?;
    let meta = metadata(p, package)?;
    let metadata_length = p.length - meta;
    for (i, compiled) in units.iter().enumerate() {
        let at = projection_extent(directory, i, UNIT_DIRECTORY_ROW_BYTES)?;
        let start = p.length;
        project_unit(p, compiled)?;
        p.word(at, start)?;
        p.field(at, 8, p.length - start)?;
    }
    for (i, (key, bytes)) in source.iter().enumerate() {
        let at = projection_extent(rows, i, SOURCE_OBJECT_ROW_BYTES)?;
        p.tag(at, u32::from(key.domain.tag()))?;
        p.span(projection_extent(at, 8, 1)?, 0, bytes)?;
        p.put(projection_extent(at, 24, 1)?, &key.digest.bytes())?;
    }
    p.put(0, b"LKJCI001")?;
    p.tag(8, ARTIFACT_PROFILE)?;
    p.tag(12, u32::from(allowance.is_some()))?;
    p.word(16, p.length)?;
    p.word(24, units.len())?;
    p.word(32, allowance.unwrap_or(0))?;
    p.word(72, directory)?;
    p.word(80, units.len())?;
    p.word(88, rows)?;
    p.word(96, source.len())?;
    p.word(104, meta)?;
    p.tag(112, 1_u32)?;
    p.word(120, metadata_length)?;
    Ok(())
}

/// Current profile-7 projection of one host-admitted package closure.
/// The compiler validates framing, unit layout and object identity. Source semantic
/// admission and correspondence to canonical unit IR remain a Tier 1 host boundary.
pub(crate) fn bundle(artifact: &super::LoadedArtifact) -> Result<Vec<u8>, Diagnostic> {
    bundle_with_allowance(artifact, None)
}

/// The embedded policy governs the consumer's deterministic proof-admission
/// estimate, not the prior host artifact admission, decoding, map reads or sorting.
/// After that preprocessing, a shared allocation-free output measurement precedes
/// exact finite policy admission and allocation/filling of the projected carrier.
pub(crate) fn bundle_with_allowance(
    artifact: &super::LoadedArtifact,
    allowance: Option<u64>,
) -> Result<Vec<u8>, Diagnostic> {
    use crate::platform::persistent_map::{MapWork, PersistentMap};
    use crate::platform::storage::object::ObjectDomain;
    artifact.require_current_admission(|| Ok(()))?;
    if artifact.manifest.packages.len() != 1 {
        return Err(unsupported("multiple artifact packages"));
    }
    let package = &artifact.manifest.packages[0];
    let pages = SourcePages {
        artifact,
        read: std::cell::RefCell::default(),
    };
    for root in [package.interface_owners, package.reference_owners] {
        PersistentMap::from_root(root)
            .verify(&pages, &mut MapWork::default())
            .map_err(|error| unsupported(&error.to_string()))?;
    }
    let source_pages = pages.read.into_inner();
    let mut units = Vec::new();
    let mut source = Vec::new();
    for (key, bytes) in &artifact.objects {
        let include_source = match key.domain {
            ObjectDomain::CompilerUnit => {
                let unit = CompilationUnit::decode(bytes, *key)?;
                units
                    .try_reserve(1)
                    .map_err(|_| unsupported("host allocation for unit inventory"))?;
                units.push(unit);
                false
            }
            ObjectDomain::CompilationManifest => false,
            ObjectDomain::MapPage => source_pages.contains(
                &crate::platform::persistent_map::PageDigest::from_bytes(key.digest.bytes()),
            ),
            ObjectDomain::Owner
            | ObjectDomain::Type
            | ObjectDomain::Blob
            | ObjectDomain::PackageRevision
            | ObjectDomain::PackageInterface => true,
            _ => return Err(unsupported("non-source artifact object")),
        };
        if include_source {
            source
                .try_reserve(1)
                .map_err(|_| unsupported("host allocation for source-object inventory"))?;
            source.push((*key, bytes.as_slice()));
        }
    }
    // Owner identities must be unique below, so an unstable in-place sort has
    // the same accepted order without an auxiliary allocation.
    units.sort_unstable_by_key(|unit| unit.source.owner);
    if units
        .windows(2)
        .any(|pair| pair[0].source.owner == pair[1].source.owner)
    {
        return Err(unsupported("unit identity order"));
    }
    let mut measured = Projection::default();
    project_bundle(&mut measured, package, &units, &source, allowance)?;
    admit_allowance(
        u64::try_from(measured.length).map_err(|_| unsupported("projection length"))?,
        u64::try_from(units.len()).map_err(|_| unsupported("unit count"))?,
        u64::try_from(source.len()).map_err(|_| unsupported("source-object count"))?,
        allowance,
    )?;
    let mut output = Projection::writing(measured.length)?;
    project_bundle(&mut output, package, &units, &source, allowance)?;
    let mut bytes = output.finish()?;
    let mut digest = blake3::Hasher::new();
    digest.update(&bytes[..40]);
    digest.update(&bytes[72..]);
    bytes[40..72].copy_from_slice(digest.finalize().as_bytes());
    Ok(bytes)
}

/// Necessary layout bounds hold independently of policy. An unmetered operation
/// never computes a potentially overflowing estimate. For finite A, compare
/// A >= 64 * (64 + 2L + U^2 + O) by division and subtraction, never by saturation.
fn admit_allowance(
    length: u64,
    units: u64,
    objects: u64,
    allowance: Option<u64>,
) -> Result<(), Diagnostic> {
    if units == 0
        || objects == 0
        || units > length / MINIMUM_PROJECTED_UNIT_BYTES
        || objects > length / SOURCE_OBJECT_ROW_BYTES as u64
    {
        return Err(unsupported("artifact allowance layout bounds"));
    }
    let Some(allowance) = allowance else {
        return Ok(());
    };
    let selected = || unsupported("selected artifact allowance");
    let mut remaining = (allowance / 64).checked_sub(64).ok_or_else(selected)?;
    if length > remaining / 2 {
        return Err(selected());
    }
    // The preceding division proves this product and subtraction exact.
    remaining -= length * 2;
    remaining = remaining.checked_sub(objects).ok_or_else(selected)?;
    if units > remaining / units {
        return Err(selected());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::manifest::CompilationManifestDigest;
    use super::super::unit::{
        CompilationSource, CompilationTables, CompilationUnitKey, CompiledParameter,
        OptimizationPolicy,
    };
    use super::*;
    use crate::platform::kernel::{
        CallableResult, Name, OwnerObjectDigest, PackageId, PackageInterfaceDigest,
        PackageRevisionDigest, SemanticStateDigest,
    };
    use crate::platform::persistent_map::{MapContentDigest, MapRoot, PageDigest};
    use crate::platform::semantic_id::{RepositoryId, RevisionId};
    use crate::platform::storage::object::ObjectDomain;
    use crate::platform::witness::SemanticDigest;

    fn filled(
        mut emit: impl FnMut(&mut Projection) -> Result<(), Diagnostic>,
    ) -> Result<Vec<u8>, Diagnostic> {
        let mut measurement = Projection::default();
        emit(&mut measurement)?;
        assert!(measurement.output.is_none());
        let mut output = Projection::writing(measurement.length)?;
        emit(&mut output)?;
        output.finish()
    }

    fn append_words(bytes: &mut Vec<u8>, words: &[u64]) {
        for word in words {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
    }

    fn read_word(bytes: &[u8], at: usize) -> u64 {
        let mut word = [0; 8];
        word.copy_from_slice(&bytes[at..at + 8]);
        u64::from_le_bytes(word)
    }

    fn signature() -> CompiledSignature {
        CompiledSignature {
            implementation_parameters: vec![],
            requirement_parameters: vec![],
            effect_parameters: vec![],
            effect: FunctionEffect::Pure,
            type_parameters: vec![],
            type_parameter_constraints: vec![],
            parameters: vec![],
            result: CallableResult {
                ty: 0,
                relation: ResultRelation::Value,
            },
            read_scope_parameters: vec![],
            read_scope_sources: vec![],
            task_requirements: vec![],
        }
    }

    fn parameter(serial: usize, name: &str) -> Result<CompiledParameter, Diagnostic> {
        Ok(CompiledParameter {
            parameter: format!("param_{serial:032x}")
                .parse()
                .map_err(|_| unsupported("test parameter identity"))?,
            name: Name::new(name)?,
            ty: 0,
            use_mode: ParameterUse::Unrestricted,
            resource_requirement: None,
        })
    }

    // These fixtures exercise projection bytes, not an artifact admission seal.
    // The full public producer tests separately start from admitted graph artifacts.
    fn component(serial: usize) -> Result<CompilationUnit, Diagnostic> {
        let digest = SemanticDigest::from_bytes([1; 32]);
        let source = CompilationSource {
            package: PackageId::migrate(b"profile-seven-layout", 0),
            owner: OwnerKey::Declaration(
                format!("decl_{serial:032x}")
                    .parse()
                    .map_err(|_| unsupported("test declaration identity"))?,
            ),
            kind: OwnerKind::Component,
            semantic_interface: digest,
            implementation: digest,
            type_digest: digest,
            effect: digest,
            capability: digest,
            test: None,
            validation_dependencies: digest,
        };
        Ok(CompilationUnit {
            contract_version: 1,
            graph_contract_version: 1,
            bytecode_contract_version: 1,
            key: CompilationUnitKey::derive(&source, OptimizationPolicy::DeterministicBaseline)?,
            source,
            optimization: OptimizationPolicy::DeterministicBaseline,
            tables: CompilationTables {
                declarations: vec![],
                fields: vec![],
                cases: vec![],
                requirements: vec![],
                operations: vec![],
                ports: vec![],
                types: vec![],
                structural_names: vec![],
                read_view_fields: vec![],
                texts: vec![],
            },
            payload: CompilationPayload::Component {
                requirements: vec![],
                ports: vec![],
            },
        })
    }

    fn package(owner: OwnerKey) -> super::super::ArtifactPackage {
        // Deliberately wide logical roots test metadata width without allocating
        // a corresponding map. This is not a claim that these roots are admitted.
        let root = MapRoot::from_parts(
            PageDigest::from_bytes([8; 32]),
            u64::MAX,
            MapContentDigest::from_bytes([9; 32]),
        );
        super::super::ArtifactPackage {
            repository_id: RepositoryId::migrate(b"profile-seven-layout", 0),
            package: PackageId::migrate(b"profile-seven-layout", 0),
            package_revision: PackageRevisionDigest::from_bytes([2; 32]),
            semantic_revision: RevisionId::from_digest([3; 32]),
            semantic_state: SemanticStateDigest::from_bytes([4; 32]),
            interface: PackageInterfaceDigest::from_bytes([5; 32]),
            interface_owners: root,
            compilation: CompilationManifestDigest::from_bytes([6; 32]),
            runtime_owners: vec![super::super::ArtifactRuntimeOwner {
                owner,
                kind: OwnerKind::Component,
                object: OwnerObjectDigest::from_bytes([7; 32]),
            }],
            reference_owners: root,
        }
    }

    #[test]
    fn selfhost_artifact_input_inventory_admission_has_no_projection_quota() {
        for count in [700, 701, 1_000, 100_000, 100_001, 1_000_001] {
            assert!(admit_bundle_counts(count, count).is_ok());
        }
        for (units, objects) in [(0, 1), (1, 0), (usize::MAX, 1), (1, usize::MAX)] {
            assert!(admit_bundle_counts(units, objects).is_err());
        }
        assert!(matches!(projection_extent(160, 701, 16), Ok(11_376)));
        assert!(projection_extent(0, usize::MAX, 48).is_err());
        assert!(projection_extent(usize::MAX, 1, 1).is_err());
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn selfhost_artifact_input_measurement_crosses_u32_without_allocating_output()
    -> Result<(), Diagnostic> {
        let boundary =
            usize::try_from(u64::from(u32::MAX) + 1).map_err(|_| unsupported("test host width"))?;
        assert_eq!(projection_extent(boundary - 1, 1, 1)?, boundary);
        let mut projection = Projection::default();
        assert_eq!(projection.reserve(boundary)?, 0);
        assert_eq!(projection.rows(1, 56)?, boundary);
        projection.word(boundary, u64::MAX)?;
        assert_eq!(projection.length, boundary + 56);
        assert!(projection.output.is_none());
        assert!(projection.reserve(usize::MAX).is_err());
        assert_eq!(projection.length, boundary + 56);
        Ok(())
    }

    #[test]
    fn selfhost_artifact_input_wide_fields_and_measured_fill_remain_exact() -> Result<(), Diagnostic>
    {
        let mut output = Projection::writing(8)?;
        output.reserve(8)?;
        output.word(0, u64::MAX)?;
        assert!(output.tag(0, u64::from(u32::MAX) + 1).is_err());
        assert!(output.word(1, 0_u64).is_err());
        assert!(output.reserve(1).is_err());
        assert_eq!(output.finish()?, [255; 8]);
        let mut short = Projection::writing(9)?;
        short.reserve(8)?;
        assert!(short.finish().is_err());
        let mut measurement = Projection::default();
        measurement.reserve(8)?;
        assert!(measurement.word(1, 0_u64).is_err());
        assert!(measurement.relative(0, 9, 8).is_err());
        assert!(measurement.finish().is_err());
        Ok(())
    }

    #[test]
    fn selfhost_artifact_input_code_matches_independent_wide_golden_rows() -> Result<(), Diagnostic>
    {
        let code = CompiledCode {
            parameter_count: 0,
            local_count: 129,
            instructions: vec![
                I::I64(-7),
                I::Call {
                    requirement_arguments: vec![],
                    effect_arguments: vec![],
                    function: u32::MAX,
                    type_arguments: vec![0, u32::MAX],
                    arguments: 129,
                    read_scope_arguments: vec![],
                },
                I::Record {
                    nominal_type: None,
                    type_arguments: vec![],
                    fields: vec![
                        CompiledFieldSelector::Structural(0),
                        CompiledFieldSelector::Structural(u32::MAX),
                    ],
                },
                I::List {
                    item_type: u32::MAX,
                    items: 129,
                },
                I::Return,
            ],
        };
        let actual = filled(|p| {
            let origin = p.reserve(57)?;
            assert_eq!(origin, 0);
            p.code(57, &code)?;
            Ok(())
        })?;
        let mut expected = vec![0; 57];
        append_words(&mut expected, &[0, 129, 32, 5]);
        let index = u64::from(u32::MAX);
        for row in [
            [4, 0, 0, u64::MAX - 6, 0, 0],
            [12, index, 129, 0, 272, 2],
            [15, 0, 0, 0, 288, 2],
            [18, index, 129, 0, 0, 0],
            [27, 0, 0, 0, 0, 0],
        ] {
            append_words(&mut expected, &row);
        }
        append_words(&mut expected, &[0, index, 0, index]);
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn selfhost_artifact_input_signature_matches_independent_wide_golden_rows()
    -> Result<(), Diagnostic> {
        let mut signature = signature();
        let mut first = parameter(1, "x")?;
        first.ty = u32::MAX;
        first.use_mode = ParameterUse::Borrow;
        let mut second = parameter(2, "yy")?;
        second.use_mode = ParameterUse::Consume;
        signature.result.ty = u32::MAX;
        signature.parameters = vec![first, second];
        let actual = filled(|p| {
            p.reserve(23)?;
            p.signature(23, &signature)?;
            Ok(())
        })?;
        let mut expected = vec![0; 23];
        append_words(&mut expected, &[40, 0, 40, 2, u64::from(u32::MAX)]);
        expected.extend_from_slice(&signature.parameters[0].parameter.bytes());
        append_words(&mut expected, &[136, 1, u64::from(u32::MAX), 1]);
        expected.extend_from_slice(&signature.parameters[1].parameter.bytes());
        append_words(&mut expected, &[137, 2, 0, 2]);
        expected.extend_from_slice(b"xyy");
        assert_eq!(actual, expected);

        signature.parameters = (1..=129)
            .map(|serial| parameter(serial, "x"))
            .collect::<Result<_, _>>()?;
        let larger = filled(|p| {
            p.signature(0, &signature)?;
            Ok(())
        })?;
        assert_eq!(read_word(&larger, 24), 129);
        assert_eq!(larger.len(), 40 + 129 * 48 + 129);
        Ok(())
    }

    #[test]
    fn selfhost_artifact_input_component_prefix_and_tables_are_unit_relative()
    -> Result<(), Diagnostic> {
        let compiled = component(1)?;
        let actual = unit(&compiled)?;
        assert_eq!(actual.len(), 448);
        assert_eq!(&actual[..8], &[1, 0, 0, 0, 8, 0, 0, 0]);
        assert_eq!(
            &actual[8..12],
            &u32::from(OwnerKind::Component.tag()).to_le_bytes()
        );
        assert_eq!(&actual[12..16], &[0; 4]);
        assert_eq!(&actual[16..32], &compiled.source.package.bytes());
        assert_eq!(&actual[32..48], &compiled.source.owner.bytes());
        assert_eq!(&actual[48..240], &[1; 192]);
        assert_eq!(read_word(&actual, 240), 272);
        assert_eq!(read_word(&actual, 248), 0);
        assert_eq!(&actual[256..264], &[9, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(read_word(&actual, 264), 432);
        for slot in 0..10 {
            assert_eq!(
                read_word(&actual, 272 + slot * 16),
                if slot == 8 { 0 } else { 432 }
            );
            assert_eq!(read_word(&actual, 280 + slot * 16), 0);
        }
        assert_eq!(read_word(&actual, 432), 448);
        assert_eq!(read_word(&actual, 440), 0);
        Ok(())
    }

    #[test]
    fn selfhost_artifact_input_bundle_header_directory_and_metadata_match_profile_seven()
    -> Result<(), Diagnostic> {
        let units = [component(1)?, component(2)?];
        let package = package(units[0].source.owner);
        let key = ObjectKey::from_digest(ObjectDomain::Blob, [11; 32]);
        let source = [(key, b"abc".as_slice())];
        for allowance in [None, Some(0), Some(u64::MAX)] {
            // This exercises wire representation, including Some(0). The public
            // bundle path rejects its insufficient allowance before output allocation.
            let bytes = filled(|p| project_bundle(p, &package, &units, &source, allowance))?;
            assert_eq!(bytes.len(), 1_515);
            assert_eq!(&bytes[..8], b"LKJCI001");
            assert_eq!(&bytes[8..12], &7_u32.to_le_bytes());
            assert_eq!(
                &bytes[12..16],
                &u32::from(allowance.is_some()).to_le_bytes()
            );
            assert_eq!(read_word(&bytes, 16), 1_515);
            assert_eq!(read_word(&bytes, 24), 2);
            assert_eq!(read_word(&bytes, 32), allowance.unwrap_or(0));
            assert_eq!(
                &bytes[40..72],
                &[0; 32],
                "digest is filled only after output"
            );
            assert_eq!(read_word(&bytes, 72), 160);
            assert_eq!(read_word(&bytes, 80), 2);
            assert_eq!(read_word(&bytes, 88), 192);
            assert_eq!(read_word(&bytes, 96), 1);
            assert_eq!(read_word(&bytes, 104), 248);
            assert_eq!(&bytes[112..120], &[1, 0, 0, 0, 0, 0, 0, 0]);
            assert_eq!(read_word(&bytes, 120), 368);
            assert_eq!(&bytes[128..160], &[0; 32]);
            assert_eq!(read_word(&bytes, 160), 616);
            assert_eq!(read_word(&bytes, 168), 448);
            assert_eq!(read_word(&bytes, 176), 1_064);
            assert_eq!(read_word(&bytes, 184), 448);
            assert_eq!(&bytes[192..196], &u32::from(key.domain.tag()).to_le_bytes());
            assert_eq!(&bytes[196..200], &[0; 4]);
            assert_eq!(read_word(&bytes, 200), 1_512);
            assert_eq!(read_word(&bytes, 208), 3);
            assert_eq!(&bytes[216..248], &[11; 32]);
            assert_eq!(read_word(&bytes, 248 + 192), u64::MAX);
            assert_eq!(read_word(&bytes, 248 + 264), u64::MAX);
            assert_eq!(read_word(&bytes, 248 + 304), 1);
            let owner = crate::platform::kernel::EncodedOwnerKey::new(units[0].source.owner);
            assert_eq!(
                &bytes[560..564],
                &(u32::from(owner.bytes()[0]) - 1).to_le_bytes()
            );
            assert_eq!(
                &bytes[564..568],
                &(u32::from(OwnerKind::Component.tag()) - 1).to_le_bytes()
            );
            assert_eq!(&bytes[568..584], &units[0].source.owner.bytes());
            assert_eq!(&bytes[584..616], &[7; 32]);
            assert_eq!(&bytes[616..1_064], unit(&units[0])?.as_slice());
            assert_eq!(&bytes[1_064..1_512], unit(&units[1])?.as_slice());
            assert_eq!(&bytes[1_512..], b"abc");
        }
        Ok(())
    }

    #[test]
    fn selfhost_artifact_input_second_bundle_unit_keeps_nested_offsets_relative()
    -> Result<(), Diagnostic> {
        let mut second = component(2)?;
        second.source.kind = OwnerKind::PureFunction;
        second.key =
            CompilationUnitKey::derive(&second.source, OptimizationPolicy::DeterministicBaseline)?;
        second.tables.structural_names.push(Name::new("field")?);
        second
            .tables
            .texts
            .push(CompiledText::Inline("payload".into()));
        let mut callable = signature();
        callable.parameters.push(parameter(1, "value")?);
        second.payload = CompilationPayload::Function {
            signature: callable,
            code: CompiledCode {
                parameter_count: 1,
                local_count: 129,
                instructions: vec![
                    I::LoadLocal {
                        local: 0,
                        use_mode: ParameterUse::Unrestricted,
                    },
                    I::Call {
                        requirement_arguments: vec![],
                        effect_arguments: vec![],
                        function: 0,
                        type_arguments: vec![0],
                        arguments: 1,
                        read_scope_arguments: vec![],
                    },
                    I::Record {
                        nominal_type: None,
                        type_arguments: vec![],
                        fields: vec![CompiledFieldSelector::Structural(0)],
                    },
                    I::Text(0),
                    I::Drop,
                    I::Return,
                ],
            },
        };
        let units = [component(1)?, second];
        let package = package(units[0].source.owner);
        let source = [(
            ObjectKey::from_digest(ObjectDomain::Blob, [11; 32]),
            b"x".as_slice(),
        )];
        let bytes = filled(|p| project_bundle(p, &package, &units, &source, None))?;
        let projected = unit(&units[1])?;
        assert_eq!(read_word(&bytes, 176), 1_064);
        assert_eq!(read_word(&bytes, 184), projected.len() as u64);
        assert_eq!(&bytes[1_064..1_064 + projected.len()], projected.as_slice());
        // These literals independently pin the nested layout, in addition to
        // comparing the nonzero-origin carrier slice with the standalone unit.
        assert_eq!(read_word(&projected, 272 + 7 * 16), 432);
        assert_eq!(read_word(&projected, 432), 448);
        assert_eq!(read_word(&projected, 440), 5);
        assert_eq!(&projected[448..453], b"field");
        assert_eq!(read_word(&projected, 272 + 9 * 16), 453);
        assert_eq!(read_word(&projected, 453), 469);
        assert_eq!(read_word(&projected, 461), 7);
        assert_eq!(&projected[469..476], b"payload");
        assert_eq!(read_word(&projected, 248), 476);
        assert_eq!(read_word(&projected, 476 + 16), 516);
        assert_eq!(read_word(&projected, 516 + 16), 564);
        assert_eq!(&projected[564..569], b"value");
        assert_eq!(read_word(&projected, 264), 569);
        assert_eq!(read_word(&projected, 569 + 16), 601);
        assert_eq!(read_word(&projected, 601 + 48 + 32), 889);
        assert_eq!(read_word(&projected, 601 + 2 * 48 + 32), 897);
        assert_eq!(projected.len(), 905);
        Ok(())
    }

    #[test]
    fn selfhost_artifact_input_code_projection_crosses_former_instruction_and_byte_caps()
    -> Result<(), Diagnostic> {
        let mut instructions = Vec::with_capacity(180_002);
        for _ in 0..90_000 {
            instructions.push(I::Unit);
            instructions.push(I::Drop);
        }
        instructions.extend([I::Unit, I::Return]);
        let code = CompiledCode {
            parameter_count: 0,
            local_count: 0,
            instructions,
        };
        let bytes = filled(|p| {
            p.code(0, &code)?;
            Ok(())
        })?;
        assert_eq!(bytes.len(), 32 + 180_002 * 48);
        assert!(bytes.len() > 8 * 1_048_576);
        assert_eq!(read_word(&bytes, 16), 32);
        assert_eq!(read_word(&bytes, 24), 180_002);
        assert_eq!(read_word(&bytes, 32), 2);
        assert_eq!(read_word(&bytes, 80), 9);
        assert_eq!(read_word(&bytes, bytes.len() - 48), 27);
        Ok(())
    }

    #[test]
    fn selfhost_artifact_input_exact_policy_matches_independent_u128_formula() {
        for (length, units, objects) in [
            (1_000, 1, 1),
            (1_000_000, 701, 1_234),
            (4_294_967_296, 1, 100_001),
            (72_057_594_037_927_903, 1, 1),
            (144_115_188_075_855_838, 1, 1),
            (144_115_188_075_855_839, 1, 1),
            (u64::MAX, u64::MAX / 432, u64::MAX / 56),
        ] {
            // Independent wider arithmetic is an oracle, never an output size or policy.
            let exact = 4_096_u128
                + 128 * u128::from(length)
                + 64 * (u128::from(units) * u128::from(units) + u128::from(objects));
            assert!(admit_allowance(length, units, objects, None).is_ok());
            let mut policies = vec![0, 1, 63, 64, 4_096, u64::MAX / 2, 1_u64 << 63, u64::MAX];
            if let Ok(exact) = u64::try_from(exact) {
                policies.extend([exact - 1, exact, exact + 1]);
            }
            for allowance in policies {
                assert_eq!(
                    admit_allowance(length, units, objects, Some(allowance)).is_ok(),
                    u128::from(allowance) >= exact,
                    "length={length} units={units} objects={objects} allowance={allowance}"
                );
            }
        }
    }

    #[test]
    fn selfhost_artifact_input_unmetered_policy_does_not_evaluate_an_overflowing_estimate() {
        let length = 144_115_188_075_855_839;
        assert_eq!(4_096_u128 + 128 * u128::from(length) + 64 * 2, 1_u128 << 64);
        assert!(admit_allowance(length, 1, 1, None).is_ok());
        assert!(admit_allowance(length, 1, 1, Some(u64::MAX)).is_err());
        assert!(admit_allowance(length - 1, 1, 1, Some(u64::MAX)).is_ok());
        assert!(admit_allowance(72_057_594_037_927_903, 1, 1, Some(1_u64 << 63)).is_ok());
        assert!(admit_allowance(72_057_594_037_927_903, 1, 1, Some((1_u64 << 63) - 1)).is_err());
        for (length, units, objects) in [(1_000, 0, 1), (1_000, 1, 0), (431, 1, 1), (432, 1, 8)] {
            for allowance in [None, Some(0), Some(u64::MAX)] {
                assert!(admit_allowance(length, units, objects, allowance).is_err());
            }
        }
    }

    #[test]
    fn selfhost_artifact_input_preserves_supported_family_and_structural_checks()
    -> Result<(), Diagnostic> {
        for code in [
            CompiledCode {
                parameter_count: 2,
                local_count: 1,
                instructions: vec![I::Unit, I::Return],
            },
            CompiledCode {
                parameter_count: 0,
                local_count: 0,
                instructions: vec![],
            },
            CompiledCode {
                parameter_count: 0,
                local_count: 0,
                instructions: vec![I::Invoke { arguments: 0 }],
            },
            CompiledCode {
                parameter_count: 0,
                local_count: 0,
                instructions: vec![I::Record {
                    nominal_type: Some(0),
                    type_arguments: vec![],
                    fields: vec![],
                }],
            },
        ] {
            let mut projection = Projection::default();
            assert!(projection.code(0, &code).is_err());
            assert!(projection.output.is_none());
        }
        let mut constrained = signature();
        let mut resource = parameter(1, "x")?;
        resource.resource_requirement = Some(0);
        constrained.parameters.push(resource);
        assert!(Projection::default().signature(0, &constrained).is_err());
        constrained = signature();
        constrained.task_requirements.push(0);
        assert!(Projection::default().signature(0, &constrained).is_err());
        let mut compiled = component(1)?;
        compiled.graph_contract_version = 2;
        assert!(unit(&compiled).is_err());
        compiled.graph_contract_version = 1;
        compiled.source.test = Some(SemanticDigest::from_bytes([2; 32]));
        assert!(unit(&compiled).is_err());
        compiled.source.test = None;
        compiled.tables.texts.push(CompiledText::Blob {
            digest: crate::platform::kernel::BlobObjectDigest::from_bytes([3; 32]),
            bytes: 1,
        });
        assert!(unit(&compiled).is_err());
        Ok(())
    }
}
