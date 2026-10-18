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

fn unsupported(what: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Source,
        "compiler_artifact_input_unsupported",
        format!("artifact unit projection does not support {what}"),
    )
}

#[derive(Default)]
struct Projection {
    bytes: Vec<u8>,
}
impl Projection {
    fn reserve(&mut self, size: usize) -> usize {
        let at = self.bytes.len();
        self.bytes.resize(at + size, 0);
        at
    }
    fn append(&mut self, bytes: &[u8]) -> usize {
        let at = self.bytes.len();
        self.bytes.extend_from_slice(bytes);
        at
    }
    fn word(&mut self, at: usize, value: usize) {
        self.bytes[at..at + 4].copy_from_slice(&(value as u32).to_le_bytes());
    }
    fn span(&mut self, at: usize, bytes: &[u8]) {
        let offset = self.append(bytes);
        self.word(at, offset);
        self.word(at + 4, bytes.len());
    }
    fn array(&mut self, at: usize, bytes: &[u8], count: usize) {
        let offset = self.append(bytes);
        self.word(at, offset);
        self.word(at + 4, count);
    }
    fn numbers(&mut self, at: usize, numbers: &[u32]) {
        let bytes = numbers
            .iter()
            .flat_map(|x| x.to_le_bytes())
            .collect::<Vec<_>>();
        self.array(at, &bytes, numbers.len());
    }
    fn strings<'a>(&mut self, at: usize, values: impl IntoIterator<Item = &'a str>) {
        let values = values.into_iter().collect::<Vec<_>>();
        let offset = self.reserve(values.len() * 8);
        self.word(at, offset);
        self.word(at + 4, values.len());
        for (i, value) in values.iter().enumerate() {
            self.span(offset + i * 8, value.as_bytes());
        }
    }
    fn signature(&mut self, signature: &CompiledSignature) -> Result<usize, Diagnostic> {
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
        let at = self.reserve(20);
        let types = signature
            .type_parameters
            .iter()
            .flat_map(|id| id.bytes())
            .collect::<Vec<_>>();
        self.array(at, &types, signature.type_parameters.len());
        let parameters = self.reserve(signature.parameters.len() * 32);
        self.word(at + 8, parameters);
        self.word(at + 12, signature.parameters.len());
        self.word(at + 16, signature.result.ty as usize);
        for (i, parameter) in signature.parameters.iter().enumerate() {
            if parameter.resource_requirement.is_some() {
                return Err(unsupported("parameter resources"));
            }
            let p = parameters + i * 32;
            self.bytes[p..p + 16].copy_from_slice(&parameter.parameter.bytes());
            self.span(p + 16, parameter.name.as_str().as_bytes());
            self.word(p + 24, parameter.ty as usize);
            self.word(p + 28, parameter_use(parameter.use_mode));
        }
        Ok(at)
    }
    fn code(&mut self, code: &CompiledCode) -> Result<usize, Diagnostic> {
        let at = self.reserve(16);
        self.word(at, code.parameter_count as usize);
        self.word(at + 4, code.local_count as usize);
        let nodes = self.reserve(code.instructions.len() * 32);
        self.word(at + 8, nodes);
        self.word(at + 12, code.instructions.len());
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
                    self.numbers(n + 24, type_arguments);
                    (12, *function as usize, *arguments as usize)
                }
                I::Record {
                    nominal_type: None,
                    type_arguments,
                    fields,
                } if type_arguments.is_empty() => {
                    let fields = fields
                        .iter()
                        .map(|field| match field {
                            CompiledFieldSelector::Structural(index) => Ok(*index),
                            _ => Err(unsupported("nominal record field")),
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    self.numbers(n + 24, &fields);
                    (15, 0, 0)
                }
                I::Field(CompiledFieldSelector::Structural(index)) => (17, *index as usize, 0),
                I::List { item_type, items } => (18, *item_type as usize, *items as usize),
                I::Return => (27, 0, 0),
                _ => {
                    return Err(unsupported(&format!(
                        "canonical instruction {instruction:?}"
                    )));
                }
            };
            self.word(n, tag);
            self.word(n + 4, a);
            self.word(n + 8, b);
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
    let mut p = Projection::default();
    p.reserve(256);
    p.word(0, owner);
    p.word(4, kind);
    p.word(8, source.kind.tag() as usize);
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
    let tables = p.reserve(80);
    p.word(240, tables);
    macro_rules! references {
        ($index:expr, $values:expr, $field:ident) => {{
            let values = &$values;
            let bytes = values
                .iter()
                .flat_map(|value| {
                    value
                        .package
                        .bytes()
                        .into_iter()
                        .chain(value.$field.bytes())
                })
                .collect::<Vec<_>>();
            p.array(tables + $index * 8, &bytes, values.len());
        }};
    }
    references!(0, unit.tables.declarations, declaration);
    references!(1, unit.tables.fields, field);
    references!(2, unit.tables.cases, case);
    references!(3, unit.tables.requirements, requirement);
    references!(4, unit.tables.operations, operation);
    references!(5, unit.tables.ports, port);
    let types = unit
        .tables
        .types
        .iter()
        .flat_map(|ty| ty.bytes())
        .collect::<Vec<_>>();
    p.array(tables + 48, &types, unit.tables.types.len());
    p.strings(
        tables + 56,
        unit.tables
            .structural_names
            .iter()
            .map(|name| name.as_str()),
    );
    if !unit.tables.read_view_fields.is_empty() {
        return Err(unsupported("read-view fields"));
    }
    let texts = unit
        .tables
        .texts
        .iter()
        .map(|text| match text {
            CompiledText::Inline(text) => Ok(text.as_str()),
            CompiledText::Blob { .. } => Err(unsupported("blob-backed text")),
        })
        .collect::<Result<Vec<_>, _>>()?;
    p.strings(tables + 72, texts);
    let (tag, payload) = match &unit.payload {
        CompilationPayload::External {
            signature,
            implementation,
        } => {
            let signature = p.signature(signature)?;
            p.word(244, signature);
            let at = p.reserve(8);
            p.span(at, implementation.as_str().as_bytes());
            (6, at)
        }
        CompilationPayload::Function { signature, code } => {
            let signature = p.signature(signature)?;
            p.word(244, signature);
            (7, p.code(code)?)
        }
        CompilationPayload::Component {
            requirements,
            ports,
        } if requirements.is_empty() => {
            let at = p.reserve(8);
            let offset = p.reserve(ports.len() * 16);
            p.word(at, offset);
            p.word(at + 4, ports.len());
            for (i, port) in ports.iter().enumerate() {
                let CompiledPortImplementation::Function(function) = port.implementation else {
                    return Err(unsupported("expression port"));
                };
                p.word(offset + i * 16, port.port as usize);
                p.word(offset + i * 16 + 4, port.function_type as usize);
                p.word(offset + i * 16 + 12, function as usize);
            }
            (9, at)
        }
        CompilationPayload::Target {
            component,
            port,
            routes,
            runner: RunnerKind::Command,
        } if routes.is_empty() => {
            let at = p.reserve(16);
            p.word(at, *component as usize);
            p.word(at + 4, usize::from(port.is_some()));
            p.word(at + 8, port.unwrap_or(0) as usize);
            (11, at)
        }
        _ => return Err(unsupported("unit payload")),
    };
    p.word(248, tag);
    p.word(252, payload);
    if p.bytes.len() > super::selfhost_input::MAX_INPUT {
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

fn metadata(package: &super::ArtifactPackage) -> Vec<u8> {
    let mut p = Projection::default();
    p.reserve(312 + package.runtime_owners.len() * 56);
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
        p.word(at + 32, root.entries() as usize);
        p.bytes[at + 40..at + 72].copy_from_slice(&root.content().bytes());
    }
    p.word(304, package.runtime_owners.len());
    for (i, owner) in package.runtime_owners.iter().enumerate() {
        let at = 312 + i * 56;
        p.word(
            at,
            usize::from(crate::platform::kernel::EncodedOwnerKey::new(owner.owner).bytes()[0] - 1),
        );
        p.word(at + 4, usize::from(owner.kind.tag() - 1));
        p.bytes[at + 8..at + 24].copy_from_slice(&owner.owner.bytes());
        p.bytes[at + 24..at + 56].copy_from_slice(&owner.object.bytes());
    }
    p.bytes
}

/// Current profile-4 artifact projection of one host-admitted package closure.
/// The compiler validates framing, unit layout and object identity. Source semantic
/// admission and correspondence to canonical unit IR remain a Tier 1 host boundary.
pub(crate) fn bundle(artifact: &super::LoadedArtifact) -> Result<Vec<u8>, Diagnostic> {
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
    if units.is_empty()
        || units.len() > 700
        || units
            .windows(2)
            .any(|pair| pair[0].source.owner == pair[1].source.owner)
    {
        return Err(unsupported("unit count or identity order"));
    }
    let mut p = Projection::default();
    p.reserve(160);
    let directory = p.reserve(units.len() * 8);
    let rows = p.reserve(source.len() * 48);
    let meta = metadata(package);
    let meta_at = p.append(&meta);
    p.word(72, directory);
    p.word(76, units.len());
    p.word(80, rows);
    p.word(84, source.len());
    p.word(88, meta_at);
    p.word(92, meta.len());
    for (i, compiled) in units.iter().enumerate() {
        p.span(directory + i * 8, &unit(compiled)?);
    }
    for (i, (key, bytes)) in source.iter().enumerate() {
        let at = rows + i * 48;
        p.word(at, usize::from(key.domain.tag()));
        p.span(at + 8, bytes);
        p.bytes[at + 16..at + 48].copy_from_slice(&key.digest.bytes());
    }
    if p.bytes.len() > super::selfhost_input::MAX_INPUT {
        return Err(unsupported("complete package input extent"));
    }
    p.bytes[..8].copy_from_slice(b"LKJCI001");
    p.word(8, 4);
    let length = p.bytes.len() as u64;
    p.bytes[16..24].copy_from_slice(&length.to_le_bytes());
    p.bytes[24..32].copy_from_slice(&(units.len() as u64).to_le_bytes());
    let fuel = 4096 + 128 * length + 64 * (units.len() * units.len() + source.len()) as u64;
    p.bytes[32..40].copy_from_slice(&fuel.to_le_bytes());
    p.word(112, 1);
    let mut digest = blake3::Hasher::new();
    digest.update(&p.bytes[..40]);
    digest.update(&p.bytes[72..]);
    p.bytes[40..72].copy_from_slice(digest.finalize().as_bytes());
    Ok(p.bytes)
}
