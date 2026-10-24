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
use std::num::NonZeroU64;

pub(crate) const ARTIFACT_PROFILE: u32 = 5;
const MAXIMUM_PROJECTED_UNIT_BYTES: usize = 8 * 1_048_576;
const MINIMUM_PROJECTED_UNIT_BYTES: u64 = 336;
const SOURCE_OBJECT_ROW_BYTES: u64 = 48;

fn unsupported(what: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Source,
        "compiler_artifact_input_unsupported",
        format!("artifact unit projection does not support {what}"),
    )
}

fn count_at_most(count: impl TryInto<u64>, maximum: u64, what: &str) -> Result<(), Diagnostic> {
    let count = count.try_into().map_err(|_| unsupported(what))?;
    if count > maximum {
        return Err(unsupported(what));
    }
    Ok(())
}

fn admit_bundle_counts(units: usize, objects: usize) -> Result<(), Diagnostic> {
    if units == 0 {
        return Err(unsupported("unit count"));
    }
    // The expandable map has no aggregate unit quota. Its input directory must
    // still fit the current unsigned 32-bit projection before projection allocation.
    projection_extent(160, units, 8)?;
    if objects == 0 || objects > 100_000 {
        return Err(unsupported("source-object count"));
    }
    Ok(())
}

fn admit_metadata_counts(
    runtime_owners: usize,
    interface_entries: u64,
    reference_entries: u64,
) -> Result<(), Diagnostic> {
    count_at_most(runtime_owners, 100_000, "runtime-owner count")?;
    count_at_most(interface_entries, 100_000, "interface-owner count")?;
    count_at_most(reference_entries, 100_000, "reference-owner count")
}

/// Offsets, extents and counts in the mode-1 projection are unsigned 32-bit
/// fields. This is a representation boundary, independent of caller allowance.
fn projection_extent(start: usize, count: usize, stride: usize) -> Result<usize, Diagnostic> {
    let end = count
        .checked_mul(stride)
        .and_then(|size| start.checked_add(size))
        .ok_or_else(|| unsupported("projection extent arithmetic"))?;
    u32::try_from(end).map_err(|_| unsupported("unsigned 32-bit projection extent"))?;
    Ok(end)
}

#[derive(Default)]
struct Projection {
    bytes: Vec<u8>,
}
impl Projection {
    fn reserve(&mut self, size: usize) -> Result<usize, Diagnostic> {
        let at = self.bytes.len();
        let end = projection_extent(at, size, 1)?;
        self.bytes
            .try_reserve(size)
            .map_err(|_| unsupported("host allocation for projection"))?;
        self.bytes.resize(end, 0);
        Ok(at)
    }
    fn rows(&mut self, count: usize, stride: usize) -> Result<usize, Diagnostic> {
        self.reserve(projection_extent(0, count, stride)?)
    }
    fn append(&mut self, bytes: &[u8]) -> Result<usize, Diagnostic> {
        let at = self.reserve(bytes.len())?;
        self.bytes[at..].copy_from_slice(bytes);
        Ok(at)
    }
    fn word(&mut self, at: usize, value: impl TryInto<u32>) -> Result<(), Diagnostic> {
        let value = value
            .try_into()
            .map_err(|_| unsupported("unsigned 32-bit projection field"))?;
        let end = projection_extent(at, 4, 1)?;
        self.bytes
            .get_mut(at..end)
            .ok_or_else(|| unsupported("projection field outside reserved storage"))?
            .copy_from_slice(&value.to_le_bytes());
        Ok(())
    }
    fn span(&mut self, at: usize, bytes: &[u8]) -> Result<(), Diagnostic> {
        let offset = self.append(bytes)?;
        self.word(at, offset)?;
        self.word(at + 4, bytes.len())
    }
    fn fixed<const WIDTH: usize>(
        &mut self,
        at: usize,
        values: impl ExactSizeIterator<Item = Result<[u8; WIDTH], Diagnostic>>,
    ) -> Result<(), Diagnostic> {
        let count = values.len();
        let offset = self.rows(count, WIDTH)?;
        self.word(at, offset)?;
        self.word(at + 4, count)?;
        for (i, value) in values.enumerate() {
            let row = offset + i * WIDTH;
            self.bytes[row..row + WIDTH].copy_from_slice(&value?);
        }
        Ok(())
    }
    fn numbers(&mut self, at: usize, numbers: &[u32]) -> Result<(), Diagnostic> {
        self.fixed(at, numbers.iter().map(|value| Ok(value.to_le_bytes())))
    }
    fn strings<'a>(
        &mut self,
        at: usize,
        values: impl ExactSizeIterator<Item = Result<&'a str, Diagnostic>>,
    ) -> Result<(), Diagnostic> {
        let count = values.len();
        let offset = self.rows(count, 8)?;
        self.word(at, offset)?;
        self.word(at + 4, count)?;
        for (i, value) in values.enumerate() {
            self.span(offset + i * 8, value?.as_bytes())?;
        }
        Ok(())
    }
    fn signature(&mut self, signature: &CompiledSignature) -> Result<usize, Diagnostic> {
        count_at_most(
            signature.type_parameters.len(),
            128,
            "signature type-parameter count",
        )?;
        count_at_most(signature.parameters.len(), 128, "signature parameter count")?;
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
        let at = self.reserve(20)?;
        self.fixed(
            at,
            signature.type_parameters.iter().map(|id| Ok(id.bytes())),
        )?;
        let parameters = self.rows(signature.parameters.len(), 32)?;
        self.word(at + 8, parameters)?;
        self.word(at + 12, signature.parameters.len())?;
        self.word(at + 16, signature.result.ty as usize)?;
        for (i, parameter) in signature.parameters.iter().enumerate() {
            if parameter.resource_requirement.is_some() {
                return Err(unsupported("parameter resources"));
            }
            let p = parameters + i * 32;
            self.bytes[p..p + 16].copy_from_slice(&parameter.parameter.bytes());
            self.span(p + 16, parameter.name.as_str().as_bytes())?;
            self.word(p + 24, parameter.ty as usize)?;
            self.word(p + 28, parameter_use(parameter.use_mode))?;
        }
        Ok(at)
    }
    fn code(&mut self, code: &CompiledCode) -> Result<usize, Diagnostic> {
        count_at_most(code.local_count, 128, "code local count")?;
        count_at_most(code.instructions.len(), 32_768, "instruction count")?;
        if code.parameter_count > code.local_count || code.instructions.is_empty() {
            return Err(unsupported("code parameter or instruction count"));
        }
        let at = self.reserve(16)?;
        self.word(at, code.parameter_count as usize)?;
        self.word(at + 4, code.local_count as usize)?;
        let nodes = self.rows(code.instructions.len(), 32)?;
        self.word(at + 8, nodes)?;
        self.word(at + 12, code.instructions.len())?;
        for (i, instruction) in code.instructions.iter().enumerate() {
            let n = nodes + i * 32;
            let (tag, a, b) = match instruction {
                I::Unit => (2, 0, 0),
                I::Bool(value) => (3, usize::from(*value), 0),
                I::I64(value) => {
                    self.bytes[n + 16..n + 24].copy_from_slice(&value.to_le_bytes());
                    (4, 0, 0)
                }
                I::Text(index) => (5, *index as usize, 0),
                I::StaticText(index) => (6, *index as usize, 0),
                I::LoadLocal { local, use_mode } => (7, *local as usize, parameter_use(*use_mode)),
                I::StoreLocal(local) => (8, *local as usize, 0),
                I::Drop => (9, 0, 0),
                I::JumpIfFalse(target) => (10, *target as usize, 0),
                I::Jump(target) => (11, *target as usize, 0),
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
                    count_at_most(type_arguments.len(), 128, "call type-argument count")?;
                    count_at_most(*arguments, 128, "call argument count")?;
                    self.numbers(n + 24, type_arguments)?;
                    (12, *function as usize, *arguments as usize)
                }
                I::Record {
                    nominal_type: None,
                    type_arguments,
                    fields,
                } if type_arguments.is_empty() => {
                    count_at_most(fields.len(), 128, "record field count")?;
                    self.fixed(
                        n + 24,
                        fields.iter().map(|field| match field {
                            CompiledFieldSelector::Structural(index) => Ok(index.to_le_bytes()),
                            _ => Err(unsupported("nominal record field")),
                        }),
                    )?;
                    (15, 0, 0)
                }
                I::Field(CompiledFieldSelector::Structural(index)) => (17, *index as usize, 0),
                I::List { item_type, items } => {
                    count_at_most(*items, 128, "list item count")?;
                    (18, *item_type as usize, *items as usize)
                }
                I::Return => (27, 0, 0),
                _ => {
                    return Err(unsupported(&format!(
                        "canonical instruction {instruction:?}"
                    )));
                }
            };
            self.word(n, tag)?;
            self.word(n + 4, a)?;
            self.word(n + 8, b)?;
        }
        Ok(at)
    }
}
fn parameter_use(value: ParameterUse) -> usize {
    match value {
        ParameterUse::Unrestricted => 0,
        ParameterUse::Borrow => 1,
        ParameterUse::Consume => 2,
    }
}

pub(crate) fn unit(unit: &CompilationUnit) -> Result<Vec<u8>, Diagnostic> {
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
    let owner = match source.owner {
        OwnerKey::Declaration(_) => 1,
        OwnerKey::Target(_) => 11,
        _ => return Err(unsupported("unit owner")),
    };
    let kind = match source.kind {
        OwnerKind::External => 4,
        OwnerKind::PureFunction => 5,
        OwnerKind::Component => 8,
        OwnerKind::Target => 19,
        _ => return Err(unsupported("unit kind")),
    };
    for (count, name) in [
        (unit.tables.declarations.len(), "declaration table count"),
        (unit.tables.fields.len(), "field table count"),
        (unit.tables.cases.len(), "case table count"),
        (unit.tables.requirements.len(), "requirement table count"),
        (unit.tables.operations.len(), "operation table count"),
        (unit.tables.ports.len(), "port table count"),
        (unit.tables.types.len(), "type table count"),
        (
            unit.tables.structural_names.len(),
            "structural-name table count",
        ),
        (unit.tables.texts.len(), "text table count"),
    ] {
        count_at_most(count, 32_768, name)?;
    }
    let mut p = Projection::default();
    p.reserve(256)?;
    p.word(0, owner)?;
    p.word(4, kind)?;
    p.word(8, source.kind.tag() as usize)?;
    p.bytes[16..32].copy_from_slice(&source.package.bytes());
    p.bytes[32..48].copy_from_slice(&source.owner.bytes());
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
        p.bytes[48 + i * 32..80 + i * 32].copy_from_slice(&digest.bytes());
    }
    let tables = p.reserve(80)?;
    p.word(240, tables)?;
    macro_rules! references {
        ($index:expr, $values:expr, $field:ident) => {{
            let values = &$values;
            p.fixed(
                tables + $index * 8,
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
        tables + 48,
        unit.tables.types.iter().map(|ty| Ok(ty.bytes())),
    )?;
    p.strings(
        tables + 56,
        unit.tables
            .structural_names
            .iter()
            .map(|name| Ok(name.as_str())),
    )?;
    if !unit.tables.read_view_fields.is_empty() {
        return Err(unsupported("read-view fields"));
    }
    p.strings(
        tables + 72,
        unit.tables.texts.iter().map(|text| match text {
            CompiledText::Inline(text) => Ok(text.as_str()),
            CompiledText::Blob { .. } => Err(unsupported("blob-backed text")),
        }),
    )?;
    let (tag, payload) = match &unit.payload {
        CompilationPayload::External {
            signature,
            implementation,
        } => {
            let signature = p.signature(signature)?;
            p.word(244, signature)?;
            let at = p.reserve(8)?;
            p.span(at, implementation.as_str().as_bytes())?;
            (6, at)
        }
        CompilationPayload::Function { signature, code } => {
            if usize::try_from(code.parameter_count).ok() != Some(signature.parameters.len()) {
                return Err(unsupported("code/signature parameter agreement"));
            }
            let signature = p.signature(signature)?;
            p.word(244, signature)?;
            (7, p.code(code)?)
        }
        CompilationPayload::Component {
            requirements,
            ports,
        } if requirements.is_empty() => {
            count_at_most(ports.len(), 128, "component port count")?;
            let at = p.reserve(8)?;
            let offset = p.rows(ports.len(), 16)?;
            p.word(at, offset)?;
            p.word(at + 4, ports.len())?;
            for (i, port) in ports.iter().enumerate() {
                let CompiledPortImplementation::Function(function) = port.implementation else {
                    return Err(unsupported("expression port"));
                };
                p.word(offset + i * 16, port.port as usize)?;
                p.word(offset + i * 16 + 4, port.function_type as usize)?;
                p.word(offset + i * 16 + 12, function as usize)?;
            }
            (9, at)
        }
        CompilationPayload::Target {
            component,
            port,
            routes,
            runner: RunnerKind::Command,
        } if routes.is_empty() => {
            let at = p.reserve(16)?;
            p.word(at, *component as usize)?;
            p.word(at + 4, usize::from(port.is_some()))?;
            p.word(at + 8, port.unwrap_or(0) as usize)?;
            (11, at)
        }
        _ => return Err(unsupported("unit payload")),
    };
    p.word(248, tag)?;
    p.word(252, payload)?;
    if p.bytes.len() > MAXIMUM_PROJECTED_UNIT_BYTES {
        return Err(unsupported("unit input extent"));
    }
    Ok(p.bytes)
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

fn metadata(package: &super::ArtifactPackage) -> Result<Vec<u8>, Diagnostic> {
    admit_metadata_counts(
        package.runtime_owners.len(),
        package.interface_owners.entries(),
        package.reference_owners.entries(),
    )?;
    let mut p = Projection::default();
    p.reserve(312)?;
    p.rows(package.runtime_owners.len(), 56)?;
    p.bytes[..16].copy_from_slice(&package.repository_id.bytes());
    p.bytes[16..32].copy_from_slice(&package.package.bytes());
    for (i, digest) in [
        package.package_revision.bytes(),
        package.semantic_revision.bytes(),
        package.semantic_state.bytes(),
        package.interface.bytes(),
    ]
    .iter()
    .enumerate()
    {
        p.bytes[32 + i * 32..64 + i * 32].copy_from_slice(digest);
    }
    for (i, root) in [package.interface_owners, package.reference_owners]
        .iter()
        .enumerate()
    {
        let at = 160 + i * 72;
        p.bytes[at..at + 32].copy_from_slice(&root.page().bytes());
        p.word(at + 32, root.entries())?;
        p.bytes[at + 40..at + 72].copy_from_slice(&root.content().bytes());
    }
    p.word(304, package.runtime_owners.len())?;
    for (i, owner) in package.runtime_owners.iter().enumerate() {
        let at = 312 + i * 56;
        p.word(
            at,
            usize::from(crate::platform::kernel::EncodedOwnerKey::new(owner.owner).bytes()[0] - 1),
        )?;
        p.word(at + 4, usize::from(owner.kind.tag() - 1))?;
        p.bytes[at + 8..at + 24].copy_from_slice(&owner.owner.bytes());
        p.bytes[at + 24..at + 56].copy_from_slice(&owner.object.bytes());
    }
    Ok(p.bytes)
}

/// Current profile-5 artifact projection of one host-admitted package closure.
/// The compiler validates framing, unit layout and object identity. Source semantic
/// admission and correspondence to canonical unit IR remain a Tier 1 host boundary.
pub(crate) fn bundle(artifact: &super::LoadedArtifact) -> Result<Vec<u8>, Diagnostic> {
    bundle_with_allowance(artifact, None)
}

/// None selects unmetered admission. A selected positive allowance is preserved
/// exactly; it must cover the format's deterministic minimum work estimate.
pub(crate) fn bundle_with_allowance(
    artifact: &super::LoadedArtifact,
    allowance: Option<NonZeroU64>,
) -> Result<Vec<u8>, Diagnostic> {
    use crate::platform::persistent_map::{MapWork, PersistentMap};
    use crate::platform::storage::object::ObjectDomain;
    artifact.require_current_admission(|| Ok(()))?;
    if artifact.manifest.packages.len() != 1 {
        return Err(unsupported("multiple artifact packages"));
    }
    let package = &artifact.manifest.packages[0];
    admit_metadata_counts(
        package.runtime_owners.len(),
        package.interface_owners.entries(),
        package.reference_owners.entries(),
    )?;
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
        match key.domain {
            ObjectDomain::CompilerUnit => units.push(CompilationUnit::decode(bytes, *key)?),
            ObjectDomain::CompilationManifest => {}
            ObjectDomain::MapPage => {
                if source_pages.contains(&crate::platform::persistent_map::PageDigest::from_bytes(
                    key.digest.bytes(),
                )) {
                    source.push((*key, bytes.as_slice()));
                }
            }
            ObjectDomain::Owner
            | ObjectDomain::Type
            | ObjectDomain::Blob
            | ObjectDomain::PackageRevision
            | ObjectDomain::PackageInterface => source.push((*key, bytes.as_slice())),
            _ => return Err(unsupported("non-source artifact object")),
        }
    }
    units.sort_by_key(|unit| unit.source.owner);
    // Hierarchical map emission preserves the per-page format bound without
    // imposing an aggregate unit quota. Input directories remain extent-checked.
    admit_bundle_counts(units.len(), source.len())?;
    if units
        .windows(2)
        .any(|pair| pair[0].source.owner == pair[1].source.owner)
    {
        return Err(unsupported("unit identity order"));
    }
    let mut p = Projection::default();
    p.reserve(160)?;
    let directory = p.rows(units.len(), 8)?;
    let rows = p.rows(source.len(), 48)?;
    let meta = metadata(package)?;
    let meta_at = p.append(&meta)?;
    p.word(72, directory)?;
    p.word(76, units.len())?;
    p.word(80, rows)?;
    p.word(84, source.len())?;
    p.word(88, meta_at)?;
    p.word(92, meta.len())?;
    for (i, compiled) in units.iter().enumerate() {
        p.span(directory + i * 8, &unit(compiled)?)?;
    }
    for (i, (key, bytes)) in source.iter().enumerate() {
        let at = rows + i * 48;
        p.word(at, usize::from(key.domain.tag()))?;
        p.span(at + 8, bytes)?;
        p.bytes[at + 16..at + 48].copy_from_slice(&key.digest.bytes());
    }
    p.bytes[..8].copy_from_slice(b"LKJCI001");
    p.word(8, ARTIFACT_PROFILE)?;
    p.word(16, p.bytes.len())?;
    p.word(24, units.len())?;
    let selected = allowance.map_or(0, NonZeroU64::get);
    if let Some(allowance) = allowance {
        let required = minimum_allowance(
            p.bytes.len() as u64,
            units.len() as u64,
            source.len() as u64,
        )?;
        if allowance.get() < required {
            return Err(unsupported("selected artifact allowance"));
        }
    }
    p.bytes[32..40].copy_from_slice(&selected.to_le_bytes());
    p.word(112, 1)?;
    let mut digest = blake3::Hasher::new();
    digest.update(&p.bytes[..40]);
    digest.update(&p.bytes[72..]);
    p.bytes[40..72].copy_from_slice(digest.finalize().as_bytes());
    Ok(p.bytes)
}

/// Aggregate minimum spans precede the quadratic expression. Contiguous unit and
/// source-object directories establish these necessary bounds independently of
/// whether the caller selected a quota.
fn minimum_allowance(length: u64, units: u64, objects: u64) -> Result<u64, Diagnostic> {
    if length > u64::from(u32::MAX)
        || units > length / MINIMUM_PROJECTED_UNIT_BYTES
        || objects > length / SOURCE_OBJECT_ROW_BYTES
    {
        return Err(unsupported("artifact allowance layout bounds"));
    }
    units
        .checked_mul(units)
        .and_then(|work| work.checked_add(objects))
        .and_then(|work| work.checked_mul(64))
        .and_then(|work| {
            length
                .checked_mul(128)
                .and_then(|bytes| work.checked_add(bytes))
        })
        .and_then(|work| work.checked_add(4096))
        .ok_or_else(|| unsupported("artifact allowance arithmetic"))
}

#[cfg(test)]
mod tests {
    use super::{
        CompiledCode, I, Projection, admit_bundle_counts, admit_metadata_counts, minimum_allowance,
        projection_extent,
    };

    #[test]
    fn selfhost_artifact_input_preserves_retained_inventory_boundaries() {
        for units in [700, 701, 1_000, 100_000] {
            assert!(admit_bundle_counts(units, 100_000).is_ok());
        }
        for (units, objects) in [(0, 1), (1, 0), (usize::MAX, 1), (1, 100_001)] {
            assert!(admit_bundle_counts(units, objects).is_err());
        }
        assert!(admit_metadata_counts(0, 0, 0).is_ok());
        assert!(admit_metadata_counts(100_000, 100_000, 100_000).is_ok());
        for (owners, interface, references) in [
            (100_001, 0, 0),
            (0, 100_001, 0),
            (0, 0, 100_001),
            (0, u64::MAX, 0),
            (0, 0, u64::MAX),
        ] {
            assert!(admit_metadata_counts(owners, interface, references).is_err());
        }
        let mut projection = Projection::default();
        assert!(
            projection
                .code(&CompiledCode {
                    parameter_count: 0,
                    local_count: 129,
                    instructions: vec![I::Unit, I::Return],
                })
                .is_err()
        );
        assert!(projection.bytes.is_empty());
        assert!(
            projection
                .code(&CompiledCode {
                    parameter_count: 0,
                    local_count: 0,
                    instructions: vec![
                        I::List {
                            item_type: 0,
                            items: 129
                        },
                        I::Return
                    ],
                })
                .is_err()
        );
    }

    #[test]
    fn selfhost_artifact_input_projection_checks_representation_before_allocation() {
        let largest = u32::MAX as usize;
        assert!(matches!(projection_extent(largest, 0, 48), Ok(value) if value == largest));
        assert!(projection_extent(largest, 1, 1).is_err());
        assert!(projection_extent(0, usize::MAX, 48).is_err());
        assert!(matches!(projection_extent(160, 701, 8), Ok(5768)));

        let mut projection = Projection::default();
        assert!(projection.reserve(4).is_ok());
        assert!(projection.word(0, u32::MAX).is_ok());
        assert_eq!(projection.bytes, [255, 255, 255, 255]);
        assert!(projection.word(0, u64::from(u32::MAX) + 1).is_err());
        assert!(projection.word(largest, 0).is_err());
        assert_eq!(projection.bytes, [255, 255, 255, 255]);
    }

    #[test]
    fn selfhost_artifact_input_allowance_arithmetic_has_a_layout_bound() {
        // Independent maximal conservative bound: unit spans and object rows
        // each fit the carrier, even though both extrema cannot coexist in it.
        assert!(matches!(
            minimum_allowance(4_294_967_295, 12_782_640, 89_478_485),
            Ok(10_457_892_146_095_296)
        ));
        assert!(minimum_allowance(4_294_967_296, 1, 1).is_err());
        assert!(minimum_allowance(160, u64::from(u32::MAX), 1).is_err());
        assert!(minimum_allowance(3_200_000_160, 400_000_000, 1).is_err());
        assert!(minimum_allowance(336, 2, 1).is_err());
        assert!(minimum_allowance(336, 1, 8).is_err());
    }
}
