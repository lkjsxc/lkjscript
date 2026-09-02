//! Exact artifact preparation from stable semantic references to dense runtime indexes.

use super::value::{
    ComponentIndex, FunctionIndex, OperationIndex, PortIndex, RecordLayoutIndex, RequirementIndex,
    VariantLayoutIndex,
};
use crate::platform::compiler::LoadedArtifact;
use crate::platform::compiler::manifest::{CompilationBinding, CompilationManifest};
use crate::platform::compiler::unit::{
    CompilationPayload, CompilationUnit, CompiledCode, CompiledFieldSelector, CompiledInstruction,
    CompiledParameter, CompiledPortImplementation, CompiledText,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH;
use crate::platform::kernel::{
    BlobObjectDigest, CaseReference, ComparisonPolicy, DeclarationPayload, DeclarationReference,
    DeclarationVisibility, EncodedOwnerKey, ExternalVisibility, FieldReference, FunctionEffect,
    Idempotency, ImplementationName, Name, OperationReference, OwnerKey, OwnerRecord, PackageId,
    ParameterParent, ParameterUse, PortReference, RequirementReference, ResourceLimit,
    SemanticStateDigest, StructuralTypeField, TypeForm, TypeObject, TypeObjectDigest,
    decode_type_object, encode_type_object,
};
use crate::platform::package::RunnerKind;
use crate::platform::persistent_map::{MapError, MapErrorClass, MapWork, PersistentMap};
use crate::platform::semantic_id::{
    ParameterId, RepositoryId, RevisionId, TargetId, TypeParameterId,
};
use crate::platform::storage::object::{
    ImmutableObjectStore, ObjectDomain, ObjectKey, StoreError, StoreErrorClass, StoreWork,
};
use crate::platform::storage::page_store::ObjectPageReader;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

type TargetMap = BTreeMap<(PackageId, TargetId), NormalizedTarget>;
type RootTargetNames = BTreeMap<Name, TargetId>;
type RuntimeOwnerMap = BTreeMap<(PackageId, OwnerKey), OwnerRecord>;

struct LoadedCompilationInputs {
    units: BTreeMap<(PackageId, OwnerKey), CompilationUnit>,
    manifests: BTreeMap<PackageId, CompilationManifest>,
}

struct FunctionValidationInputs<'a> {
    types: &'a BTreeMap<TypeObjectDigest, TypeObject>,
    requirements: &'a [NormalizedRequirement],
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NormalizedPreparationWork {
    pub packages: u64,
    pub compiler_units: u64,
    pub runtime_owners: u64,
    pub type_objects: u64,
    pub instructions: u64,
    pub functions: u64,
    pub record_layouts: u64,
    pub variant_layouts: u64,
    pub requirements: u64,
    pub operations: u64,
    pub components: u64,
    pub ports: u64,
    pub targets: u64,
    pub tests: u64,
    pub map: MapWork,
    pub store: StoreWork,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedCode {
    pub parameter_count: u32,
    pub local_count: u32,
    pub instructions: Arc<[NormalizedInstruction]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NormalizedInstruction {
    Unit,
    Bool(bool),
    I64(i64),
    Text(Arc<str>),
    StaticText(Arc<str>),
    LoadLocal {
        local: u32,
        use_mode: ParameterUse,
    },
    StoreLocal(u32),
    Drop,
    JumpIfFalse(u32),
    Jump(u32),
    Call {
        function: FunctionIndex,
        type_arguments: Arc<[TypeObjectDigest]>,
        arguments: u32,
    },
    FunctionValue {
        function: FunctionIndex,
        type_arguments: Arc<[TypeObjectDigest]>,
    },
    Invoke {
        arguments: u32,
    },
    Record {
        layout: Option<RecordLayoutIndex>,
        fields: Arc<[NormalizedFieldSelector]>,
    },
    Variant {
        layout: VariantLayoutIndex,
        case: u32,
        has_payload: bool,
    },
    Field(NormalizedFieldSelector),
    List {
        items: u32,
    },
    Map {
        entries: u32,
    },
    SwitchVariant(Arc<[NormalizedVariantJump]>),
    Perform {
        requirement: RequirementIndex,
        operation: OperationIndex,
        arguments: u32,
    },
    BeginTransaction {
        requirement: RequirementIndex,
        binding: u32,
    },
    CommitTransaction {
        requirement: RequirementIndex,
        binding: u32,
    },
    Return,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NormalizedFieldSelector {
    Nominal {
        layout: RecordLayoutIndex,
        offset: u32,
    },
    Structural(Name),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NormalizedVariantJump {
    pub layout: VariantLayoutIndex,
    pub case: u32,
    pub target: u32,
    pub binding_local: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NormalizedFunctionBody {
    Code(NormalizedCode),
    External(ImplementationName),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedFunction {
    pub declaration: DeclarationReference,
    pub type_parameters: Arc<[TypeParameterId]>,
    pub parameter_count: u32,
    pub parameters: Arc<[NormalizedParameter]>,
    pub result: TypeObjectDigest,
    pub task_requirements: Arc<[RequirementIndex]>,
    pub body: NormalizedFunctionBody,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedParameter {
    pub parameter: ParameterId,
    pub name: Name,
    pub ty: TypeObjectDigest,
    pub use_mode: ParameterUse,
    pub resource_requirement: Option<RequirementIndex>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedRecordField {
    pub reference: FieldReference,
    pub name: Name,
    pub ty: TypeObjectDigest,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedRecordLayout {
    pub declaration: DeclarationReference,
    pub fields: Arc<[NormalizedRecordField]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedVariantCase {
    pub reference: CaseReference,
    pub name: Name,
    pub payload: Option<TypeObjectDigest>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedVariantLayout {
    pub declaration: DeclarationReference,
    pub cases: Arc<[NormalizedVariantCase]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedRequirement {
    pub reference: RequirementReference,
    pub name: Name,
    pub interface: DeclarationReference,
    pub operations: Arc<[OperationIndex]>,
    pub limits: Arc<[ResourceLimit]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedOperation {
    pub reference: OperationReference,
    pub name: Name,
    pub parameters: Arc<[NormalizedParameter]>,
    pub result: TypeObjectDigest,
    pub idempotency: Idempotency,
    pub external_visibility: ExternalVisibility,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NormalizedEntryPoint {
    Function(FunctionIndex),
    Code(NormalizedCode),
    PortExpression(NormalizedCode),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedPort {
    pub reference: PortReference,
    pub name: Name,
    pub function_type: TypeObjectDigest,
    pub component: ComponentIndex,
    pub entry: NormalizedEntryPoint,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedComponent {
    pub declaration: DeclarationReference,
    pub requirements: Arc<[RequirementIndex]>,
    pub ports: Arc<[PortIndex]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedTarget {
    pub package: PackageId,
    pub target: TargetId,
    pub name: Name,
    pub runner: RunnerKind,
    pub component: ComponentIndex,
    pub port: PortIndex,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedTest {
    pub declaration: DeclarationReference,
    pub actual: NormalizedCode,
    pub expected: NormalizedCode,
    pub comparison: ComparisonPolicy,
}

#[derive(Clone, Debug)]
pub struct NormalizedProgram {
    artifact: Arc<LoadedArtifact>,
    pub root_repository: RepositoryId,
    pub root_package: PackageId,
    pub root_revision: RevisionId,
    pub root_semantic_state: SemanticStateDigest,
    pub work: NormalizedPreparationWork,
    pub(crate) functions: Arc<[NormalizedFunction]>,
    pub(crate) function_by_declaration: BTreeMap<DeclarationReference, FunctionIndex>,
    pub(crate) records: Arc<[NormalizedRecordLayout]>,
    pub(crate) variants: Arc<[NormalizedVariantLayout]>,
    pub(crate) requirements: Arc<[NormalizedRequirement]>,
    pub(crate) operations: Arc<[NormalizedOperation]>,
    pub(crate) components: Arc<[NormalizedComponent]>,
    pub(crate) ports: Arc<[NormalizedPort]>,
    pub(crate) targets: TargetMap,
    pub(crate) root_target_names: RootTargetNames,
    pub(crate) tests: BTreeMap<DeclarationReference, NormalizedTest>,
    pub(crate) types: BTreeMap<TypeObjectDigest, TypeObject>,
}

impl NormalizedProgram {
    pub fn prepare(artifact: LoadedArtifact) -> Result<Self, Diagnostic> {
        let artifact = Arc::new(artifact);
        let mut work = NormalizedPreparationWork::default();
        let LoadedCompilationInputs { units, manifests } = load_units(&artifact, &mut work)?;
        let root_compilation = manifests
            .get(&artifact.manifest.root_package)
            .ok_or_else(|| {
                runtime_corrupt(
                    "normalized_root_compilation_missing",
                    "normalized artifact has no exact root-package compilation manifest",
                )
            })?;
        let indexes = RuntimeIndexes::build(&units)?;
        let runtime_owners = load_runtime_owners(&artifact, &mut work)?;
        let types = load_type_objects(&artifact, &mut work)?;
        let mut text_cache = BTreeMap::new();

        let records = prepare_records(&units, &indexes, &runtime_owners)?;
        let variants = prepare_variants(&units, &indexes, &runtime_owners)?;
        let requirements = prepare_requirements(&units, &indexes, &runtime_owners)?;
        let operations = prepare_operations(&indexes, &runtime_owners)?;
        let function_validation = FunctionValidationInputs {
            types: &types,
            requirements: &requirements,
        };
        let functions = prepare_functions(
            &artifact,
            &units,
            &indexes,
            &runtime_owners,
            &function_validation,
            &mut text_cache,
            &mut work,
        )?;
        validate_resource_call_graph(&functions)?;
        let (components, ports) = prepare_components(
            &artifact,
            &units,
            &indexes,
            &runtime_owners,
            &mut text_cache,
            &mut work,
        )?;
        let tests = prepare_tests(&artifact, &units, &indexes, &mut text_cache, &mut work)?;
        let (targets, root_target_names) =
            prepare_targets(&artifact, &units, &indexes, &runtime_owners)?;

        work.functions = functions.len() as u64;
        work.record_layouts = records.len() as u64;
        work.variant_layouts = variants.len() as u64;
        work.requirements = requirements.len() as u64;
        work.operations = operations.len() as u64;
        work.components = components.len() as u64;
        work.ports = ports.len() as u64;
        work.targets = targets.len() as u64;
        work.tests = tests.len() as u64;
        Ok(Self {
            root_repository: root_compilation.repository_id,
            root_package: artifact.manifest.root_package,
            root_revision: root_compilation.revision,
            root_semantic_state: root_compilation.semantic_state,
            artifact,
            work,
            functions: functions.into(),
            function_by_declaration: indexes.functions,
            records: records.into(),
            variants: variants.into(),
            requirements: requirements.into(),
            operations: operations.into(),
            components: components.into(),
            ports: ports.into(),
            targets,
            root_target_names,
            tests,
            types,
        })
    }

    pub fn artifact(&self) -> &LoadedArtifact {
        &self.artifact
    }

    pub fn function(&self, declaration: DeclarationReference) -> Option<FunctionIndex> {
        self.function_by_declaration.get(&declaration).copied()
    }

    pub fn target(&self, package: PackageId, target: TargetId) -> Option<&NormalizedTarget> {
        self.targets.get(&(package, target))
    }

    pub fn root_target(&self, name: &Name) -> Option<&NormalizedTarget> {
        self.root_target_names
            .get(name)
            .and_then(|target| self.target(self.root_package, *target))
    }

    pub fn tests(&self) -> impl Iterator<Item = &NormalizedTest> {
        self.tests.values()
    }

    pub(crate) fn substitute_type(
        &self,
        digest: TypeObjectDigest,
        substitutions: &BTreeMap<TypeParameterId, TypeObjectDigest>,
        depth: usize,
    ) -> Option<TypeObjectDigest> {
        if depth > MAXIMUM_TYPE_DEPTH {
            return None;
        }
        let object = self.types.get(&digest)?;
        let next = depth.saturating_add(1);
        let form = match &object.form {
            TypeForm::TypeParameter { parameter } => {
                let digest = substitutions.get(parameter).copied()?;
                return self.types.contains_key(&digest).then_some(digest);
            }
            TypeForm::StructuralRecord { fields } => TypeForm::StructuralRecord {
                fields: fields
                    .iter()
                    .map(|field| {
                        Some(StructuralTypeField {
                            name: field.name.clone(),
                            ty: self.substitute_type(field.ty, substitutions, next)?,
                        })
                    })
                    .collect::<Option<Vec<_>>>()?,
            },
            TypeForm::List { item } => TypeForm::List {
                item: self.substitute_type(*item, substitutions, next)?,
            },
            TypeForm::Map { key, value } => TypeForm::Map {
                key: self.substitute_type(*key, substitutions, next)?,
                value: self.substitute_type(*value, substitutions, next)?,
            },
            TypeForm::Option { item } => TypeForm::Option {
                item: self.substitute_type(*item, substitutions, next)?,
            },
            TypeForm::Result { ok, error } => TypeForm::Result {
                ok: self.substitute_type(*ok, substitutions, next)?,
                error: self.substitute_type(*error, substitutions, next)?,
            },
            TypeForm::Stream { item } => TypeForm::Stream {
                item: self.substitute_type(*item, substitutions, next)?,
            },
            TypeForm::Function { parameters, result } => TypeForm::Function {
                parameters: parameters
                    .iter()
                    .map(|parameter| self.substitute_type(*parameter, substitutions, next))
                    .collect::<Option<Vec<_>>>()?,
                result: self.substitute_type(*result, substitutions, next)?,
            },
            other => other.clone(),
        };
        let object = TypeObject::new(form).ok()?;
        let (digest, _) = encode_type_object(&object).ok()?;
        self.types.contains_key(&digest).then_some(digest)
    }
}

#[derive(Default)]
struct RuntimeIndexes {
    functions: BTreeMap<DeclarationReference, FunctionIndex>,
    records: BTreeMap<DeclarationReference, RecordLayoutIndex>,
    variants: BTreeMap<DeclarationReference, VariantLayoutIndex>,
    fields: BTreeMap<FieldReference, (RecordLayoutIndex, u32)>,
    cases: BTreeMap<CaseReference, (VariantLayoutIndex, u32)>,
    requirements: BTreeMap<RequirementReference, RequirementIndex>,
    operations: BTreeMap<OperationReference, OperationIndex>,
    components: BTreeMap<DeclarationReference, ComponentIndex>,
    ports: BTreeMap<PortReference, PortIndex>,
}

impl RuntimeIndexes {
    fn build(units: &BTreeMap<(PackageId, OwnerKey), CompilationUnit>) -> Result<Self, Diagnostic> {
        let mut function_refs = BTreeSet::new();
        let mut record_refs = BTreeSet::new();
        let mut variant_refs = BTreeSet::new();
        let mut requirement_refs = BTreeSet::new();
        let mut operation_refs = BTreeSet::new();
        let mut component_refs = BTreeSet::new();
        let mut port_refs = BTreeSet::new();
        for ((package, owner), unit) in units {
            let OwnerKey::Declaration(declaration) = owner else {
                continue;
            };
            let reference = DeclarationReference {
                package: *package,
                declaration: *declaration,
            };
            match &unit.payload {
                CompilationPayload::Record { .. } => {
                    record_refs.insert(reference);
                }
                CompilationPayload::Variant { .. } => {
                    variant_refs.insert(reference);
                }
                CompilationPayload::Interface { operations } => {
                    for operation in operations {
                        operation_refs.insert(index_copy(
                            &unit.tables.operations,
                            operation.operation,
                            "normalized interface operation",
                        )?);
                    }
                }
                CompilationPayload::External { signature, .. }
                | CompilationPayload::Function { signature, .. } => {
                    function_refs.insert(reference);
                    for requirement in &signature.task_requirements {
                        requirement_refs.insert(index_copy(
                            &unit.tables.requirements,
                            *requirement,
                            "normalized task requirement",
                        )?);
                    }
                }
                CompilationPayload::Constant { .. } => {
                    function_refs.insert(reference);
                }
                CompilationPayload::Component {
                    requirements,
                    ports,
                } => {
                    component_refs.insert(reference);
                    for requirement in requirements {
                        requirement_refs.insert(index_copy(
                            &unit.tables.requirements,
                            requirement.requirement,
                            "normalized component requirement",
                        )?);
                    }
                    for port in ports {
                        port_refs.insert(index_copy(
                            &unit.tables.ports,
                            port.port,
                            "normalized component port",
                        )?);
                    }
                }
                CompilationPayload::Test { .. } | CompilationPayload::Target { .. } => {}
            }
        }
        let functions = dense_map(function_refs, FunctionIndex)?;
        let records = dense_map(record_refs, RecordLayoutIndex)?;
        let variants = dense_map(variant_refs, VariantLayoutIndex)?;
        let requirements = dense_map(requirement_refs, RequirementIndex)?;
        let operations = dense_map(operation_refs, OperationIndex)?;
        let components = dense_map(component_refs, ComponentIndex)?;
        let ports = dense_map(port_refs, PortIndex)?;
        let mut fields = BTreeMap::new();
        let mut cases = BTreeMap::new();
        for ((package, owner), unit) in units {
            let OwnerKey::Declaration(declaration) = owner else {
                continue;
            };
            let declaration = DeclarationReference {
                package: *package,
                declaration: *declaration,
            };
            match &unit.payload {
                CompilationPayload::Record { fields: layouts } => {
                    let layout = required_index(&records, declaration, "record layout")?;
                    for (offset, field) in layouts.iter().enumerate() {
                        let field = index_copy(
                            &unit.tables.fields,
                            field.field,
                            "normalized record field",
                        )?;
                        let offset = u32_index(offset, "record field offset")?;
                        if fields.insert(field, (layout, offset)).is_some() {
                            return Err(runtime_corrupt(
                                "normalized_field_duplicate",
                                "one exact field appears in multiple runtime record layouts",
                            ));
                        }
                    }
                }
                CompilationPayload::Variant { cases: layouts } => {
                    let layout = required_index(&variants, declaration, "variant layout")?;
                    for (tag, case) in layouts.iter().enumerate() {
                        let case =
                            index_copy(&unit.tables.cases, case.case, "normalized variant case")?;
                        let tag = u32_index(tag, "variant case tag")?;
                        if cases.insert(case, (layout, tag)).is_some() {
                            return Err(runtime_corrupt(
                                "normalized_case_duplicate",
                                "one exact case appears in multiple runtime variant layouts",
                            ));
                        }
                    }
                }
                CompilationPayload::Interface { .. }
                | CompilationPayload::External { .. }
                | CompilationPayload::Function { .. }
                | CompilationPayload::Constant { .. }
                | CompilationPayload::Component { .. }
                | CompilationPayload::Test { .. }
                | CompilationPayload::Target { .. } => {}
            }
        }
        Ok(Self {
            functions,
            records,
            variants,
            fields,
            cases,
            requirements,
            operations,
            components,
            ports,
        })
    }
}

fn load_units(
    artifact: &LoadedArtifact,
    work: &mut NormalizedPreparationWork,
) -> Result<LoadedCompilationInputs, Diagnostic> {
    let mut units = BTreeMap::new();
    let mut compilations = BTreeMap::new();
    for package in &artifact.manifest.packages {
        let bytes = required_object(
            artifact,
            package.compilation.object_key(),
            "normalized runtime compilation manifest is missing",
            &mut work.store,
        )?;
        let compilation = CompilationManifest::decode(&bytes, package.compilation)?;
        if compilations
            .insert(package.package, compilation.clone())
            .is_some()
        {
            return Err(runtime_corrupt(
                "normalized_compilation_duplicate",
                "normalized artifact repeats one exact package compilation manifest",
            ));
        }
        let reader = ObjectPageReader::new(artifact);
        let mut map_work = MapWork::default();
        let mut captured = None;
        let result = PersistentMap::from_root(compilation.units).for_each(
            &reader,
            &mut map_work,
            |key, value| {
                let operation = (|| {
                    let owner = EncodedOwnerKey::decode(key)?;
                    let binding = CompilationBinding::decode(value, owner)?;
                    let bytes = required_object(
                        artifact,
                        binding.object.object_key(),
                        "normalized runtime compiler unit is missing",
                        &mut work.store,
                    )?;
                    let unit = CompilationUnit::decode(&bytes, binding.object.object_key())?;
                    if unit.key != binding.key
                        || unit.source.package != package.package
                        || unit.source.owner != owner
                        || unit.source.kind != binding.kind
                    {
                        return Err(runtime_corrupt(
                            "normalized_unit_binding",
                            "normalized runtime unit disagrees with its compilation manifest",
                        ));
                    }
                    if units.insert((package.package, owner), unit).is_some() {
                        return Err(runtime_corrupt(
                            "normalized_unit_duplicate",
                            "normalized runtime repeats one exact compiler-unit owner",
                        ));
                    }
                    Ok::<(), Diagnostic>(())
                })();
                match operation {
                    Ok(()) => Ok(()),
                    Err(diagnostic) => {
                        captured = Some(diagnostic);
                        Err(MapError {
                            class: MapErrorClass::Corrupt,
                            code: "normalized_unit_iteration_stop",
                            message: "normalized runtime unit iteration stopped after an exact diagnostic"
                                .to_owned(),
                        })
                    }
                }
            },
        );
        add_map_work(&mut work.map, map_work);
        work.store.add(reader.work());
        if let Some(diagnostic) = captured {
            return Err(diagnostic);
        }
        result.map_err(map_diagnostic)?;
        work.packages = work.packages.saturating_add(1);
    }
    work.compiler_units = units.len() as u64;
    Ok(LoadedCompilationInputs {
        units,
        manifests: compilations,
    })
}

fn load_runtime_owners(
    artifact: &LoadedArtifact,
    work: &mut NormalizedPreparationWork,
) -> Result<RuntimeOwnerMap, Diagnostic> {
    let mut owners = BTreeMap::new();
    for package in &artifact.manifest.packages {
        for binding in &package.runtime_owners {
            let record = artifact.runtime_owner(
                package.package,
                binding.owner,
                binding.kind,
                &mut work.store,
            )?;
            if owners
                .insert((package.package, binding.owner), record)
                .is_some()
            {
                return Err(runtime_corrupt(
                    "normalized_runtime_owner_duplicate",
                    "normalized preparation repeats one exact runtime owner",
                ));
            }
        }
    }
    work.runtime_owners = owners.len() as u64;
    Ok(owners)
}

fn load_type_objects(
    artifact: &LoadedArtifact,
    work: &mut NormalizedPreparationWork,
) -> Result<BTreeMap<TypeObjectDigest, TypeObject>, Diagnostic> {
    let keys = artifact
        .objects
        .keys()
        .filter(|key| key.domain == ObjectDomain::Type)
        .copied()
        .collect::<Vec<_>>();
    let mut types = BTreeMap::new();
    for key in keys {
        let digest = TypeObjectDigest::from_bytes(key.digest.bytes());
        let bytes = required_object(
            artifact,
            key,
            "normalized runtime type object is missing",
            &mut work.store,
        )?;
        let object = decode_type_object(&bytes, digest)?;
        if types.insert(digest, object).is_some() {
            return Err(runtime_corrupt(
                "normalized_type_duplicate",
                "normalized preparation repeats one exact type object",
            ));
        }
    }
    work.type_objects = types.len() as u64;
    Ok(types)
}

fn prepare_records(
    units: &BTreeMap<(PackageId, OwnerKey), CompilationUnit>,
    indexes: &RuntimeIndexes,
    runtime_owners: &RuntimeOwnerMap,
) -> Result<Vec<NormalizedRecordLayout>, Diagnostic> {
    let mut records = vec![None; indexes.records.len()];
    for (declaration, index) in &indexes.records {
        let unit = declaration_unit(units, *declaration)?;
        let CompilationPayload::Record { fields } = &unit.payload else {
            return Err(runtime_corrupt(
                "normalized_record_payload",
                "record layout index names another compiler payload",
            ));
        };
        let fields = fields
            .iter()
            .map(|field| {
                let reference =
                    index_copy(&unit.tables.fields, field.field, "normalized record field")?;
                let OwnerRecord::Field(record) = exact_runtime_owner(
                    runtime_owners,
                    reference.package,
                    OwnerKey::Field(reference.field),
                    "record field",
                )?
                else {
                    return Err(runtime_corrupt(
                        "normalized_record_field_kind",
                        "record field runtime metadata has another owner kind",
                    ));
                };
                Ok(NormalizedRecordField {
                    reference,
                    name: record.name.clone(),
                    ty: record.ty,
                })
            })
            .collect::<Result<Vec<_>, Diagnostic>>()?;
        records[index.0 as usize] = Some(NormalizedRecordLayout {
            declaration: *declaration,
            fields: fields.into(),
        });
    }
    finish_dense(records, "record layout")
}

fn prepare_variants(
    units: &BTreeMap<(PackageId, OwnerKey), CompilationUnit>,
    indexes: &RuntimeIndexes,
    runtime_owners: &RuntimeOwnerMap,
) -> Result<Vec<NormalizedVariantLayout>, Diagnostic> {
    let mut variants = vec![None; indexes.variants.len()];
    for (declaration, index) in &indexes.variants {
        let unit = declaration_unit(units, *declaration)?;
        let CompilationPayload::Variant { cases } = &unit.payload else {
            return Err(runtime_corrupt(
                "normalized_variant_payload",
                "variant layout index names another compiler payload",
            ));
        };
        let cases = cases
            .iter()
            .map(|case| {
                let reference =
                    index_copy(&unit.tables.cases, case.case, "normalized variant case")?;
                let OwnerRecord::Case(record) = exact_runtime_owner(
                    runtime_owners,
                    reference.package,
                    OwnerKey::Case(reference.case),
                    "variant case",
                )?
                else {
                    return Err(runtime_corrupt(
                        "normalized_variant_case_kind",
                        "variant case runtime metadata has another owner kind",
                    ));
                };
                Ok(NormalizedVariantCase {
                    reference,
                    name: record.name.clone(),
                    payload: record.payload,
                })
            })
            .collect::<Result<Vec<_>, Diagnostic>>()?;
        variants[index.0 as usize] = Some(NormalizedVariantLayout {
            declaration: *declaration,
            cases: cases.into(),
        });
    }
    finish_dense(variants, "variant layout")
}

fn prepare_requirements(
    _units: &BTreeMap<(PackageId, OwnerKey), CompilationUnit>,
    indexes: &RuntimeIndexes,
    runtime_owners: &RuntimeOwnerMap,
) -> Result<Vec<NormalizedRequirement>, Diagnostic> {
    let mut requirements = vec![None; indexes.requirements.len()];
    for (reference, index) in &indexes.requirements {
        let OwnerRecord::Requirement(record) = exact_runtime_owner(
            runtime_owners,
            reference.package,
            OwnerKey::Requirement(reference.requirement),
            "requirement",
        )?
        else {
            return Err(runtime_corrupt(
                "normalized_requirement_owner_kind",
                "requirement runtime metadata has another owner kind",
            ));
        };
        let operations = record
            .operations
            .iter()
            .map(|operation| required_index(&indexes.operations, *operation, "operation"))
            .collect::<Result<Vec<_>, _>>()?;
        let value = NormalizedRequirement {
            reference: *reference,
            name: record.name.clone(),
            interface: record.interface,
            operations: operations.into(),
            limits: record.limits.clone().into(),
        };
        if requirements[index.0 as usize].replace(value).is_some() {
            return Err(runtime_corrupt(
                "normalized_requirement_duplicate",
                "one exact requirement has multiple runtime definitions",
            ));
        }
    }
    finish_dense(requirements, "requirement")
}

fn prepare_operations(
    indexes: &RuntimeIndexes,
    runtime_owners: &RuntimeOwnerMap,
) -> Result<Vec<NormalizedOperation>, Diagnostic> {
    let mut operations = vec![None; indexes.operations.len()];
    for (reference, index) in &indexes.operations {
        let OwnerRecord::Operation(record) = exact_runtime_owner(
            runtime_owners,
            reference.package,
            OwnerKey::Operation(reference.operation),
            "interface operation",
        )?
        else {
            return Err(runtime_corrupt(
                "normalized_operation_owner_kind",
                "interface operation runtime metadata has another owner kind",
            ));
        };
        let parameters = record
            .parameters
            .iter()
            .map(|parameter| {
                normalized_runtime_parameter(runtime_owners, reference.package, *parameter)
            })
            .collect::<Result<Vec<_>, _>>()?;
        operations[index.0 as usize] = Some(NormalizedOperation {
            reference: *reference,
            name: record.name.clone(),
            parameters: parameters.into(),
            result: record.result,
            idempotency: record.idempotency,
            external_visibility: record.external_visibility,
        });
    }
    finish_dense(operations, "operation")
}

fn prepare_functions(
    artifact: &LoadedArtifact,
    units: &BTreeMap<(PackageId, OwnerKey), CompilationUnit>,
    indexes: &RuntimeIndexes,
    runtime_owners: &RuntimeOwnerMap,
    validation: &FunctionValidationInputs<'_>,
    text_cache: &mut BTreeMap<BlobObjectDigest, Arc<str>>,
    work: &mut NormalizedPreparationWork,
) -> Result<Vec<NormalizedFunction>, Diagnostic> {
    let mut functions = vec![None; indexes.functions.len()];
    for (declaration, index) in &indexes.functions {
        let unit = declaration_unit(units, *declaration)?;
        let (type_parameters, parameters, result, task_requirements, body) = match &unit.payload {
            CompilationPayload::External {
                signature,
                implementation,
            } => (
                signature.type_parameters.clone(),
                normalized_parameters(
                    runtime_owners,
                    declaration.package,
                    unit,
                    &signature.parameters,
                    indexes,
                )?,
                index_copy(&unit.tables.types, signature.result, "external result type")?,
                translate_requirement_indexes(unit, &signature.task_requirements, indexes)?,
                NormalizedFunctionBody::External(implementation.clone()),
            ),
            CompilationPayload::Function { signature, code } => (
                signature.type_parameters.clone(),
                normalized_parameters(
                    runtime_owners,
                    declaration.package,
                    unit,
                    &signature.parameters,
                    indexes,
                )?,
                index_copy(&unit.tables.types, signature.result, "function result type")?,
                translate_requirement_indexes(unit, &signature.task_requirements, indexes)?,
                NormalizedFunctionBody::Code(translate_code(
                    artifact, unit, code, indexes, text_cache, work,
                )?),
            ),
            CompilationPayload::Constant { ty, code } => (
                Vec::new(),
                Vec::new(),
                index_copy(&unit.tables.types, *ty, "constant type")?,
                Vec::new(),
                NormalizedFunctionBody::Code(translate_code(
                    artifact, unit, code, indexes, text_cache, work,
                )?),
            ),
            CompilationPayload::Record { .. }
            | CompilationPayload::Variant { .. }
            | CompilationPayload::Interface { .. }
            | CompilationPayload::Component { .. }
            | CompilationPayload::Test { .. }
            | CompilationPayload::Target { .. } => {
                return Err(runtime_corrupt(
                    "normalized_function_payload",
                    "function dense index names a non-callable compiler payload",
                ));
            }
        };
        let parameter_count = u32_index(parameters.len(), "function parameter count")?;
        validate_normalized_resource_signature(
            *declaration,
            &type_parameters,
            &parameters,
            result,
            &task_requirements,
            &body,
            units,
            runtime_owners,
            validation.types,
            validation.requirements,
        )?;
        functions[index.0 as usize] = Some(NormalizedFunction {
            declaration: *declaration,
            type_parameters: type_parameters.into(),
            parameter_count,
            parameters: parameters.into(),
            result,
            task_requirements: task_requirements.into(),
            body,
        });
    }
    finish_dense(functions, "function")
}

#[allow(
    clippy::too_many_arguments,
    reason = "hostile preparation rechecks one complete resource-bearing signature"
)]
fn validate_normalized_resource_signature(
    declaration: DeclarationReference,
    type_parameters: &[TypeParameterId],
    parameters: &[NormalizedParameter],
    result: TypeObjectDigest,
    task_requirements: &[RequirementIndex],
    body: &NormalizedFunctionBody,
    units: &BTreeMap<(PackageId, OwnerKey), CompilationUnit>,
    owners: &RuntimeOwnerMap,
    types: &BTreeMap<TypeObjectDigest, TypeObject>,
    requirements: &[NormalizedRequirement],
) -> Result<(), Diagnostic> {
    let mut direct = Vec::new();
    for (index, parameter) in parameters.iter().enumerate() {
        let form = types.get(&parameter.ty).ok_or_else(|| {
            runtime_corrupt(
                "normalized_parameter_type_missing",
                "function parameter type is absent from the prepared type closure",
            )
        })?;
        match &form.form {
            TypeForm::CapabilityResource { interface } => {
                direct.push((index, parameter, *interface));
            }
            _ => {
                if normalized_type_contains_resource(
                    parameter.ty,
                    units,
                    types,
                    &mut BTreeSet::new(),
                    &mut BTreeSet::new(),
                )? {
                    return Err(runtime_corrupt(
                        "normalized_function_resource_container",
                        "function parameter contains a resource outside its direct type",
                    ));
                }
                if parameter.use_mode != ParameterUse::Unrestricted
                    || parameter.resource_requirement.is_some()
                {
                    return Err(runtime_corrupt(
                        "normalized_function_parameter_use",
                        "ordinary function parameter carries affine use or requirement metadata",
                    ));
                }
            }
        }
    }
    if direct.is_empty() {
        return Ok(());
    }
    if direct.len() != 1 {
        return Err(runtime_corrupt(
            "normalized_function_resource_count",
            "function signature contains more than one direct resource parameter",
        ));
    }
    let (index, parameter, interface) = direct[0];
    let requirement_index = parameter.resource_requirement.ok_or_else(|| {
        runtime_corrupt(
            "normalized_function_resource_requirement",
            "direct resource function parameter omits its exact requirement binding",
        )
    })?;
    if index.saturating_add(1) != parameters.len()
        || parameter.use_mode != ParameterUse::Consume
        || !type_parameters.is_empty()
        || !matches!(body, NormalizedFunctionBody::Code(_))
        || !task_requirements.contains(&requirement_index)
    {
        return Err(runtime_corrupt(
            "normalized_function_resource_shape",
            "resource signature is not one final consume parameter on a nongeneric task body",
        ));
    }
    if normalized_type_contains_resource(
        result,
        units,
        types,
        &mut BTreeSet::new(),
        &mut BTreeSet::new(),
    )? {
        return Err(runtime_corrupt(
            "normalized_function_resource_result",
            "resource-bearing function returns affine authority",
        ));
    }
    let requirement = requirements
        .get(requirement_index.0 as usize)
        .ok_or_else(|| {
            runtime_corrupt(
                "normalized_function_resource_requirement",
                "resource parameter requirement escaped the prepared table",
            )
        })?;
    if requirement.reference.package != declaration.package || requirement.interface != interface {
        return Err(runtime_corrupt(
            "normalized_function_resource_authority",
            "resource parameter is not bound to the same-package exact requirement and interface",
        ));
    }
    let OwnerRecord::Declaration(record) = exact_runtime_owner(
        owners,
        declaration.package,
        OwnerKey::Declaration(declaration.declaration),
        "resource-bearing function",
    )?
    else {
        return Err(runtime_corrupt(
            "normalized_function_resource_owner",
            "resource-bearing function runtime metadata has another owner kind",
        ));
    };
    let DeclarationPayload::Function(function) = &record.payload else {
        return Err(runtime_corrupt(
            "normalized_function_resource_kind",
            "resource-bearing prepared callable is not a graph task function",
        ));
    };
    let parameter_ids = parameters
        .iter()
        .map(|parameter| parameter.parameter)
        .collect::<Vec<_>>();
    let requirement_references = task_requirements
        .iter()
        .map(|index| {
            requirements
                .get(index.0 as usize)
                .map(|value| value.reference)
        })
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| {
            runtime_corrupt(
                "normalized_function_requirement",
                "task requirement escaped the prepared table",
            )
        })?;
    if record.visibility != DeclarationVisibility::Private
        || function.type_parameters != type_parameters
        || function.parameters != parameter_ids
        || function.result != result
        || !matches!(
            &function.effect,
            FunctionEffect::Task { requirements: canonical }
                if canonical == &requirement_references
                    && canonical.contains(&requirement.reference)
        )
    {
        return Err(runtime_corrupt(
            "normalized_function_resource_binding",
            "prepared resource signature disagrees with its private canonical task declaration",
        ));
    }
    let OwnerRecord::Parameter(canonical_parameter) = exact_runtime_owner(
        owners,
        declaration.package,
        OwnerKey::Parameter(parameter.parameter),
        "resource parameter",
    )?
    else {
        return Err(runtime_corrupt(
            "normalized_function_resource_parameter",
            "resource parameter runtime metadata has another owner kind",
        ));
    };
    if canonical_parameter.parent != ParameterParent::Function(declaration.declaration)
        || canonical_parameter.resource_requirement != Some(requirement.reference)
    {
        return Err(runtime_corrupt(
            "normalized_function_resource_parameter",
            "resource parameter parent or requirement disagrees with its exact function",
        ));
    }
    Ok(())
}

fn normalized_type_contains_resource(
    digest: TypeObjectDigest,
    units: &BTreeMap<(PackageId, OwnerKey), CompilationUnit>,
    types: &BTreeMap<TypeObjectDigest, TypeObject>,
    active_types: &mut BTreeSet<TypeObjectDigest>,
    active_declarations: &mut BTreeSet<DeclarationReference>,
) -> Result<bool, Diagnostic> {
    if !active_types.insert(digest) {
        return Ok(false);
    }
    let object = types.get(&digest).ok_or_else(|| {
        runtime_corrupt(
            "normalized_resource_type_missing",
            "resource-shape validation cannot read one exact prepared type",
        )
    })?;
    let result = match &object.form {
        TypeForm::CapabilityResource { .. } => true,
        TypeForm::Named { declaration } => {
            if !active_declarations.insert(*declaration) {
                false
            } else {
                let unit = declaration_unit(units, *declaration)?;
                let members = match &unit.payload {
                    CompilationPayload::Record { fields } => fields
                        .iter()
                        .map(|field| {
                            index_copy(&unit.tables.types, field.ty, "named record field type")
                                .map(Some)
                        })
                        .collect::<Result<Vec<_>, Diagnostic>>()?,
                    CompilationPayload::Variant { cases } => cases
                        .iter()
                        .map(|case| {
                            case.payload
                                .map(|payload| {
                                    index_copy(
                                        &unit.tables.types,
                                        payload,
                                        "named variant case payload type",
                                    )
                                })
                                .transpose()
                        })
                        .collect::<Result<Vec<_>, Diagnostic>>()?,
                    _ => {
                        return Err(runtime_corrupt(
                            "normalized_resource_declaration_kind",
                            "named resource container is not a record or variant compiler unit",
                        ));
                    }
                };
                let mut contains = false;
                for member in members.into_iter().flatten() {
                    if normalized_type_contains_resource(
                        member,
                        units,
                        types,
                        active_types,
                        active_declarations,
                    )? {
                        contains = true;
                        break;
                    }
                }
                active_declarations.remove(declaration);
                contains
            }
        }
        _ => {
            let mut contains = false;
            for child in object.child_types() {
                if normalized_type_contains_resource(
                    child,
                    units,
                    types,
                    active_types,
                    active_declarations,
                )? {
                    contains = true;
                    break;
                }
            }
            contains
        }
    };
    active_types.remove(&digest);
    Ok(result)
}

fn validate_resource_call_graph(functions: &[NormalizedFunction]) -> Result<(), Diagnostic> {
    let resource_functions = functions
        .iter()
        .enumerate()
        .filter_map(|(index, function)| {
            function
                .parameters
                .iter()
                .any(|parameter| parameter.resource_requirement.is_some())
                .then_some(index)
        })
        .collect::<BTreeSet<_>>();
    let mut edges = BTreeMap::<usize, BTreeSet<usize>>::new();
    let mut incoming = resource_functions
        .iter()
        .copied()
        .map(|index| (index, 0_usize))
        .collect::<BTreeMap<_, _>>();
    for (caller_index, function) in functions.iter().enumerate() {
        let NormalizedFunctionBody::Code(code) = &function.body else {
            continue;
        };
        for (instruction_index, instruction) in code.instructions.iter().enumerate() {
            match instruction {
                NormalizedInstruction::FunctionValue { function, .. }
                    if resource_functions.contains(&(function.0 as usize)) =>
                {
                    return Err(runtime_corrupt(
                        "normalized_resource_function_value",
                        "prepared bytecode creates a value for a resource-bearing function",
                    ));
                }
                NormalizedInstruction::Call {
                    function: callee,
                    arguments,
                    ..
                } if resource_functions.contains(&(callee.0 as usize)) => {
                    let callee_index = callee.0 as usize;
                    let callee = functions.get(callee_index).ok_or_else(|| {
                        runtime_corrupt(
                            "normalized_resource_call_target",
                            "resource call target escaped the prepared function table",
                        )
                    })?;
                    if function.declaration.package != callee.declaration.package
                        || *arguments != callee.parameter_count
                        || !matches!(
                            instruction_index
                                .checked_sub(1)
                                .and_then(|index| code.instructions.get(index)),
                            Some(NormalizedInstruction::LoadLocal {
                                use_mode: ParameterUse::Consume,
                                ..
                            })
                        )
                    {
                        return Err(runtime_corrupt(
                            "normalized_resource_call_transfer",
                            "prepared resource call is not one same-package final consume-local transfer",
                        ));
                    }
                    if resource_functions.contains(&caller_index)
                        && edges.entry(caller_index).or_default().insert(callee_index)
                    {
                        let Some(count) = incoming.get_mut(&callee_index) else {
                            return Err(runtime_corrupt(
                                "normalized_resource_call_graph",
                                "resource call graph target is absent from its exact node set",
                            ));
                        };
                        *count = count.saturating_add(1);
                    }
                }
                _ => {}
            }
        }
    }
    let mut ready = incoming
        .iter()
        .filter_map(|(index, count)| (*count == 0).then_some(*index))
        .collect::<BTreeSet<_>>();
    let mut visited = 0_usize;
    while let Some(index) = ready.pop_first() {
        visited = visited.saturating_add(1);
        for target in edges.get(&index).into_iter().flatten() {
            let Some(count) = incoming.get_mut(target) else {
                return Err(runtime_corrupt(
                    "normalized_resource_call_graph",
                    "resource call graph target is absent from its exact node set",
                ));
            };
            *count = count.saturating_sub(1);
            if *count == 0 {
                ready.insert(*target);
            }
        }
    }
    if visited != resource_functions.len() {
        return Err(runtime_corrupt(
            "normalized_resource_call_cycle",
            "prepared resource-bearing direct-call graph is cyclic",
        ));
    }
    Ok(())
}

fn prepare_components(
    artifact: &LoadedArtifact,
    units: &BTreeMap<(PackageId, OwnerKey), CompilationUnit>,
    indexes: &RuntimeIndexes,
    runtime_owners: &RuntimeOwnerMap,
    text_cache: &mut BTreeMap<BlobObjectDigest, Arc<str>>,
    work: &mut NormalizedPreparationWork,
) -> Result<(Vec<NormalizedComponent>, Vec<NormalizedPort>), Diagnostic> {
    let mut components = vec![None; indexes.components.len()];
    let mut ports = vec![None; indexes.ports.len()];
    for (declaration, component_index) in &indexes.components {
        let unit = declaration_unit(units, *declaration)?;
        let CompilationPayload::Component {
            requirements: compiled_requirements,
            ports: compiled_ports,
        } = &unit.payload
        else {
            return Err(runtime_corrupt(
                "normalized_component_payload",
                "component dense index names another compiler payload",
            ));
        };
        let requirements = compiled_requirements
            .iter()
            .map(|requirement| {
                let reference = index_copy(
                    &unit.tables.requirements,
                    requirement.requirement,
                    "normalized component requirement",
                )?;
                required_index(&indexes.requirements, reference, "requirement")
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut component_ports = Vec::with_capacity(compiled_ports.len());
        for port in compiled_ports {
            let reference = index_copy(&unit.tables.ports, port.port, "normalized component port")?;
            let port_index = required_index(&indexes.ports, reference, "port")?;
            let OwnerRecord::Port(record) = exact_runtime_owner(
                runtime_owners,
                reference.package,
                OwnerKey::Port(reference.port),
                "component port",
            )?
            else {
                return Err(runtime_corrupt(
                    "normalized_port_owner_kind",
                    "component port runtime metadata has another owner kind",
                ));
            };
            let entry = match &port.implementation {
                CompiledPortImplementation::Function(function) => {
                    let declaration = index_copy(
                        &unit.tables.declarations,
                        *function,
                        "normalized port function",
                    )?;
                    NormalizedEntryPoint::Function(required_index(
                        &indexes.functions,
                        declaration,
                        "function",
                    )?)
                }
                CompiledPortImplementation::Expression(code) => {
                    NormalizedEntryPoint::PortExpression(translate_code(
                        artifact, unit, code, indexes, text_cache, work,
                    )?)
                }
            };
            if ports[port_index.0 as usize]
                .replace(NormalizedPort {
                    reference,
                    name: record.name.clone(),
                    function_type: record.function_type,
                    component: *component_index,
                    entry,
                })
                .is_some()
            {
                return Err(runtime_corrupt(
                    "normalized_port_duplicate",
                    "one exact port has multiple runtime definitions",
                ));
            }
            component_ports.push(port_index);
        }
        components[component_index.0 as usize] = Some(NormalizedComponent {
            declaration: *declaration,
            requirements: requirements.into(),
            ports: component_ports.into(),
        });
    }
    Ok((
        finish_dense(components, "component")?,
        finish_dense(ports, "port")?,
    ))
}

fn prepare_tests(
    artifact: &LoadedArtifact,
    units: &BTreeMap<(PackageId, OwnerKey), CompilationUnit>,
    indexes: &RuntimeIndexes,
    text_cache: &mut BTreeMap<BlobObjectDigest, Arc<str>>,
    work: &mut NormalizedPreparationWork,
) -> Result<BTreeMap<DeclarationReference, NormalizedTest>, Diagnostic> {
    let mut tests = BTreeMap::new();
    for ((package, owner), unit) in units {
        let OwnerKey::Declaration(declaration) = owner else {
            continue;
        };
        let CompilationPayload::Test {
            actual,
            expected,
            comparison,
        } = &unit.payload
        else {
            continue;
        };
        let declaration = DeclarationReference {
            package: *package,
            declaration: *declaration,
        };
        let test = NormalizedTest {
            declaration,
            actual: translate_code(artifact, unit, actual, indexes, text_cache, work)?,
            expected: translate_code(artifact, unit, expected, indexes, text_cache, work)?,
            comparison: *comparison,
        };
        if tests.insert(declaration, test).is_some() {
            return Err(runtime_corrupt(
                "normalized_test_duplicate",
                "one exact test has multiple runtime definitions",
            ));
        }
    }
    Ok(tests)
}

fn prepare_targets(
    artifact: &LoadedArtifact,
    units: &BTreeMap<(PackageId, OwnerKey), CompilationUnit>,
    indexes: &RuntimeIndexes,
    runtime_owners: &RuntimeOwnerMap,
) -> Result<(TargetMap, RootTargetNames), Diagnostic> {
    let mut targets = BTreeMap::new();
    let mut root_names = BTreeMap::new();
    for ((package, owner), unit) in units {
        let OwnerKey::Target(target) = owner else {
            continue;
        };
        let CompilationPayload::Target {
            component,
            port,
            runner,
        } = &unit.payload
        else {
            return Err(runtime_corrupt(
                "normalized_target_payload",
                "target unit has another compiler payload",
            ));
        };
        let component = index_copy(
            &unit.tables.declarations,
            *component,
            "normalized target component",
        )?;
        let component = required_index(&indexes.components, component, "component")?;
        let port = index_copy(&unit.tables.ports, *port, "normalized target port")?;
        let port = required_index(&indexes.ports, port, "port")?;
        let OwnerRecord::Target(record) = exact_runtime_owner(
            runtime_owners,
            *package,
            OwnerKey::Target(*target),
            "target",
        )?
        else {
            return Err(runtime_corrupt(
                "normalized_target_owner_kind",
                "target owner binding decoded another owner kind",
            ));
        };
        let target_value = NormalizedTarget {
            package: *package,
            target: *target,
            name: record.name.clone(),
            runner: *runner,
            component,
            port,
        };
        if targets.insert((*package, *target), target_value).is_some() {
            return Err(runtime_corrupt(
                "normalized_target_duplicate",
                "one exact target has multiple runtime definitions",
            ));
        }
        if *package == artifact.manifest.root_package
            && root_names.insert(record.name.clone(), *target).is_some()
        {
            return Err(runtime_corrupt(
                "normalized_target_name_duplicate",
                "root artifact package repeats one target name",
            ));
        }
    }
    Ok((targets, root_names))
}

fn exact_runtime_owner<'a>(
    owners: &'a RuntimeOwnerMap,
    package: PackageId,
    owner: OwnerKey,
    label: &'static str,
) -> Result<&'a OwnerRecord, Diagnostic> {
    owners.get(&(package, owner)).ok_or_else(|| {
        runtime_corrupt(
            "normalized_runtime_owner_missing",
            format!("{label} has no exact runtime-owner metadata"),
        )
    })
}

fn normalized_parameters(
    owners: &RuntimeOwnerMap,
    package: PackageId,
    unit: &CompilationUnit,
    parameters: &[CompiledParameter],
    indexes: &RuntimeIndexes,
) -> Result<Vec<NormalizedParameter>, Diagnostic> {
    parameters
        .iter()
        .map(|parameter| normalized_parameter(owners, package, unit, *parameter, indexes))
        .collect()
}

fn normalized_parameter(
    owners: &RuntimeOwnerMap,
    package: PackageId,
    unit: &CompilationUnit,
    parameter: CompiledParameter,
    indexes: &RuntimeIndexes,
) -> Result<NormalizedParameter, Diagnostic> {
    let parameter_id = parameter.parameter;
    let OwnerRecord::Parameter(record) = exact_runtime_owner(
        owners,
        package,
        OwnerKey::Parameter(parameter_id),
        "parameter",
    )?
    else {
        return Err(runtime_corrupt(
            "normalized_parameter_owner_kind",
            "parameter runtime metadata has another owner kind",
        ));
    };
    let compiled_type = index_copy(&unit.tables.types, parameter.ty, "parameter type")?;
    let compiled_requirement = parameter
        .resource_requirement
        .map(|requirement| {
            let reference = index_copy(
                &unit.tables.requirements,
                requirement,
                "parameter resource requirement",
            )?;
            required_index(
                &indexes.requirements,
                reference,
                "parameter resource requirement",
            )
            .map(|index| (reference, index))
        })
        .transpose()?;
    if compiled_type != record.ty
        || parameter.use_mode != record.use_mode
        || compiled_requirement.map(|(reference, _)| reference) != record.resource_requirement
    {
        return Err(runtime_corrupt(
            "normalized_parameter_signature",
            "compiled parameter type, use, or resource requirement disagrees with exact runtime-owner metadata",
        ));
    }
    Ok(NormalizedParameter {
        parameter: parameter_id,
        name: record.name.clone(),
        ty: record.ty,
        use_mode: record.use_mode,
        resource_requirement: compiled_requirement.map(|(_, index)| index),
    })
}

fn normalized_runtime_parameter(
    owners: &RuntimeOwnerMap,
    package: PackageId,
    parameter: ParameterId,
) -> Result<NormalizedParameter, Diagnostic> {
    let OwnerRecord::Parameter(record) =
        exact_runtime_owner(owners, package, OwnerKey::Parameter(parameter), "parameter")?
    else {
        return Err(runtime_corrupt(
            "normalized_parameter_owner_kind",
            "parameter runtime metadata has another owner kind",
        ));
    };
    if record.resource_requirement.is_some() {
        return Err(runtime_corrupt(
            "normalized_operation_resource_binding",
            "operation parameter carries a function resource requirement binding",
        ));
    }
    Ok(NormalizedParameter {
        parameter,
        name: record.name.clone(),
        ty: record.ty,
        use_mode: record.use_mode,
        resource_requirement: None,
    })
}

fn translate_code(
    artifact: &LoadedArtifact,
    unit: &CompilationUnit,
    code: &CompiledCode,
    indexes: &RuntimeIndexes,
    text_cache: &mut BTreeMap<BlobObjectDigest, Arc<str>>,
    work: &mut NormalizedPreparationWork,
) -> Result<NormalizedCode, Diagnostic> {
    let mut instructions = Vec::with_capacity(code.instructions.len());
    for instruction in &code.instructions {
        let translated = match instruction {
            CompiledInstruction::Unit => NormalizedInstruction::Unit,
            CompiledInstruction::Bool(value) => NormalizedInstruction::Bool(*value),
            CompiledInstruction::I64(value) => NormalizedInstruction::I64(*value),
            CompiledInstruction::Text(text) => NormalizedInstruction::Text(resolve_text(
                artifact,
                index_ref(&unit.tables.texts, *text, "normalized text")?,
                text_cache,
                &mut work.store,
            )?),
            CompiledInstruction::StaticText(text) => {
                NormalizedInstruction::StaticText(resolve_text(
                    artifact,
                    index_ref(&unit.tables.texts, *text, "normalized static text")?,
                    text_cache,
                    &mut work.store,
                )?)
            }
            CompiledInstruction::LoadLocal { local, use_mode } => {
                NormalizedInstruction::LoadLocal {
                    local: *local,
                    use_mode: *use_mode,
                }
            }
            CompiledInstruction::StoreLocal(local) => NormalizedInstruction::StoreLocal(*local),
            CompiledInstruction::Drop => NormalizedInstruction::Drop,
            CompiledInstruction::JumpIfFalse(target) => NormalizedInstruction::JumpIfFalse(*target),
            CompiledInstruction::Jump(target) => NormalizedInstruction::Jump(*target),
            CompiledInstruction::Call {
                function,
                type_arguments,
                arguments,
            } => {
                let declaration = index_copy(
                    &unit.tables.declarations,
                    *function,
                    "normalized call target",
                )?;
                NormalizedInstruction::Call {
                    function: required_index(&indexes.functions, declaration, "function")?,
                    type_arguments: type_arguments
                        .iter()
                        .map(|index| {
                            index_copy(&unit.tables.types, *index, "normalized call type argument")
                        })
                        .collect::<Result<Vec<_>, _>>()?
                        .into(),
                    arguments: *arguments,
                }
            }
            CompiledInstruction::FunctionValue {
                function,
                type_arguments,
            } => {
                let declaration = index_copy(
                    &unit.tables.declarations,
                    *function,
                    "normalized function value",
                )?;
                NormalizedInstruction::FunctionValue {
                    function: required_index(&indexes.functions, declaration, "function")?,
                    type_arguments: type_arguments
                        .iter()
                        .map(|index| {
                            index_copy(
                                &unit.tables.types,
                                *index,
                                "normalized function-value type argument",
                            )
                        })
                        .collect::<Result<Vec<_>, _>>()?
                        .into(),
                }
            }
            CompiledInstruction::Invoke { arguments } => NormalizedInstruction::Invoke {
                arguments: *arguments,
            },
            CompiledInstruction::Record {
                nominal_type,
                fields,
            } => {
                let layout = nominal_type
                    .map(|declaration| {
                        let declaration = index_copy(
                            &unit.tables.declarations,
                            declaration,
                            "normalized nominal record",
                        )?;
                        required_index(&indexes.records, declaration, "record layout")
                    })
                    .transpose()?;
                let fields = fields
                    .iter()
                    .map(|field| translate_field(unit, field, indexes))
                    .collect::<Result<Vec<_>, _>>()?;
                if fields.iter().any(|field| match (layout, field) {
                    (Some(expected), NormalizedFieldSelector::Nominal { layout, .. }) => {
                        expected != *layout
                    }
                    (None, NormalizedFieldSelector::Structural(_)) => false,
                    _ => true,
                }) {
                    return Err(runtime_corrupt(
                        "normalized_record_selector",
                        "record construction mixes a foreign nominal or structural field selector",
                    ));
                }
                NormalizedInstruction::Record {
                    layout,
                    fields: fields.into(),
                }
            }
            CompiledInstruction::Variant { case, has_payload } => {
                let reference = index_copy(&unit.tables.cases, *case, "normalized variant case")?;
                let (layout, case) = required_index(&indexes.cases, reference, "case")?;
                NormalizedInstruction::Variant {
                    layout,
                    case,
                    has_payload: *has_payload,
                }
            }
            CompiledInstruction::Field(field) => {
                NormalizedInstruction::Field(translate_field(unit, field, indexes)?)
            }
            CompiledInstruction::List { items, .. } => {
                NormalizedInstruction::List { items: *items }
            }
            CompiledInstruction::Map { entries, .. } => {
                NormalizedInstruction::Map { entries: *entries }
            }
            CompiledInstruction::SwitchVariant(jumps) => {
                let jumps = jumps
                    .iter()
                    .map(|jump| {
                        let reference =
                            index_copy(&unit.tables.cases, jump.case, "normalized switch case")?;
                        let (layout, case) = required_index(&indexes.cases, reference, "case")?;
                        Ok(NormalizedVariantJump {
                            layout,
                            case,
                            target: jump.target,
                            binding_local: jump.binding_local,
                        })
                    })
                    .collect::<Result<Vec<_>, Diagnostic>>()?;
                NormalizedInstruction::SwitchVariant(jumps.into())
            }
            CompiledInstruction::Perform {
                requirement,
                operation,
                arguments,
            } => {
                let requirement = index_copy(
                    &unit.tables.requirements,
                    *requirement,
                    "normalized capability requirement",
                )?;
                let operation = index_copy(
                    &unit.tables.operations,
                    *operation,
                    "normalized capability operation",
                )?;
                NormalizedInstruction::Perform {
                    requirement: required_index(&indexes.requirements, requirement, "requirement")?,
                    operation: required_index(&indexes.operations, operation, "operation")?,
                    arguments: *arguments,
                }
            }
            CompiledInstruction::BeginTransaction {
                requirement,
                binding,
            } => {
                let requirement = index_copy(
                    &unit.tables.requirements,
                    *requirement,
                    "normalized transaction requirement",
                )?;
                NormalizedInstruction::BeginTransaction {
                    requirement: required_index(&indexes.requirements, requirement, "requirement")?,
                    binding: *binding,
                }
            }
            CompiledInstruction::CommitTransaction {
                requirement,
                binding,
            } => {
                let requirement = index_copy(
                    &unit.tables.requirements,
                    *requirement,
                    "normalized transaction requirement",
                )?;
                NormalizedInstruction::CommitTransaction {
                    requirement: required_index(&indexes.requirements, requirement, "requirement")?,
                    binding: *binding,
                }
            }
            CompiledInstruction::Return => NormalizedInstruction::Return,
        };
        instructions.push(translated);
    }
    work.instructions = work.instructions.saturating_add(instructions.len() as u64);
    Ok(NormalizedCode {
        parameter_count: code.parameter_count,
        local_count: code.local_count,
        instructions: instructions.into(),
    })
}

fn translate_field(
    unit: &CompilationUnit,
    field: &CompiledFieldSelector,
    indexes: &RuntimeIndexes,
) -> Result<NormalizedFieldSelector, Diagnostic> {
    match field {
        CompiledFieldSelector::Nominal(field) => {
            let reference = index_copy(&unit.tables.fields, *field, "normalized nominal field")?;
            let (layout, offset) = required_index(&indexes.fields, reference, "field")?;
            Ok(NormalizedFieldSelector::Nominal { layout, offset })
        }
        CompiledFieldSelector::Structural(name) => Ok(NormalizedFieldSelector::Structural(
            index_ref(
                &unit.tables.structural_names,
                *name,
                "normalized structural field",
            )?
            .clone(),
        )),
    }
}

fn translate_requirement_indexes(
    unit: &CompilationUnit,
    local: &[u32],
    indexes: &RuntimeIndexes,
) -> Result<Vec<RequirementIndex>, Diagnostic> {
    local
        .iter()
        .map(|requirement| {
            let reference = index_copy(
                &unit.tables.requirements,
                *requirement,
                "normalized task requirement",
            )?;
            required_index(&indexes.requirements, reference, "requirement")
        })
        .collect()
}

fn resolve_text(
    artifact: &LoadedArtifact,
    text: &CompiledText,
    cache: &mut BTreeMap<BlobObjectDigest, Arc<str>>,
    work: &mut StoreWork,
) -> Result<Arc<str>, Diagnostic> {
    match text {
        CompiledText::Inline(value) => Ok(Arc::from(value.as_str())),
        CompiledText::Blob { digest, bytes } => {
            if let Some(value) = cache.get(digest) {
                return Ok(value.clone());
            }
            let key = ObjectKey::from_digest(ObjectDomain::Blob, digest.bytes());
            let value = required_object(artifact, key, "normalized text blob is missing", work)?;
            if value.len() as u64 != *bytes {
                return Err(runtime_corrupt(
                    "normalized_text_blob_length",
                    "normalized text blob length disagrees with its compiler unit",
                ));
            }
            let value = std::str::from_utf8(&value).map_err(|_| {
                runtime_corrupt(
                    "normalized_text_blob_utf8",
                    "normalized text blob is not valid UTF-8",
                )
            })?;
            let value: Arc<str> = Arc::from(value);
            cache.insert(*digest, value.clone());
            Ok(value)
        }
    }
}

fn declaration_unit(
    units: &BTreeMap<(PackageId, OwnerKey), CompilationUnit>,
    declaration: DeclarationReference,
) -> Result<&CompilationUnit, Diagnostic> {
    units
        .get(&(
            declaration.package,
            OwnerKey::Declaration(declaration.declaration),
        ))
        .ok_or_else(|| {
            runtime_corrupt(
                "normalized_declaration_unit_missing",
                "dense declaration reference has no compiler unit",
            )
        })
}

fn required_object(
    artifact: &LoadedArtifact,
    key: ObjectKey,
    message: &'static str,
    work: &mut StoreWork,
) -> Result<Vec<u8>, Diagnostic> {
    artifact
        .read(key, key.domain.maximum_bytes(), work)
        .map_err(store_diagnostic)?
        .ok_or_else(|| runtime_corrupt("normalized_object_missing", message))
}

fn index_ref<'a, T>(values: &'a [T], index: u32, label: &'static str) -> Result<&'a T, Diagnostic> {
    values.get(index as usize).ok_or_else(|| {
        runtime_corrupt(
            "normalized_dense_index",
            format!("{label} dense index is outside its verified table"),
        )
    })
}

fn index_copy<T: Copy>(values: &[T], index: u32, label: &'static str) -> Result<T, Diagnostic> {
    index_ref(values, index, label).copied()
}

fn required_index<K: Ord + std::fmt::Debug, V: Copy>(
    values: &BTreeMap<K, V>,
    key: K,
    label: &'static str,
) -> Result<V, Diagnostic> {
    values.get(&key).copied().ok_or_else(|| {
        runtime_corrupt(
            "normalized_relocation_missing",
            format!("exact {label} reference {key:?} has no dense runtime binding"),
        )
    })
}

fn dense_map<K: Ord, V: Copy>(
    values: BTreeSet<K>,
    wrap: impl Fn(u32) -> V,
) -> Result<BTreeMap<K, V>, Diagnostic> {
    values
        .into_iter()
        .enumerate()
        .map(|(index, value)| Ok((value, wrap(u32_index(index, "dense runtime index")?))))
        .collect()
}

fn finish_dense<T>(values: Vec<Option<T>>, label: &'static str) -> Result<Vec<T>, Diagnostic> {
    values
        .into_iter()
        .map(|value| {
            value.ok_or_else(|| {
                runtime_corrupt(
                    "normalized_dense_hole",
                    format!("dense {label} table contains an uninitialized slot"),
                )
            })
        })
        .collect()
}

fn u32_index(value: usize, label: &'static str) -> Result<u32, Diagnostic> {
    u32::try_from(value).map_err(|_| {
        runtime_error(
            DiagnosticClass::Resource,
            "normalized_index_count",
            format!("{label} exceeds the dense runtime index domain"),
        )
    })
}

fn add_map_work(total: &mut MapWork, other: MapWork) {
    total.pages_read = total.pages_read.saturating_add(other.pages_read);
    total.pages_decoded = total.pages_decoded.saturating_add(other.pages_decoded);
    total.pages_encoded = total.pages_encoded.saturating_add(other.pages_encoded);
    total.pages_written = total.pages_written.saturating_add(other.pages_written);
    total.pages_reused = total.pages_reused.saturating_add(other.pages_reused);
    total.bytes_read = total.bytes_read.saturating_add(other.bytes_read);
    total.bytes_encoded = total.bytes_encoded.saturating_add(other.bytes_encoded);
    total.bytes_written = total.bytes_written.saturating_add(other.bytes_written);
    total.key_comparisons = total.key_comparisons.saturating_add(other.key_comparisons);
    total.entries_visited = total.entries_visited.saturating_add(other.entries_visited);
    total.differences_emitted = total
        .differences_emitted
        .saturating_add(other.differences_emitted);
    total.subtrees_skipped = total
        .subtrees_skipped
        .saturating_add(other.subtrees_skipped);
    total.entries_skipped = total.entries_skipped.saturating_add(other.entries_skipped);
}

fn map_diagnostic(error: MapError) -> Diagnostic {
    runtime_error(
        match error.class {
            MapErrorClass::Input => DiagnosticClass::Source,
            MapErrorClass::Resource => DiagnosticClass::Resource,
            MapErrorClass::Corrupt => DiagnosticClass::Corrupt,
            MapErrorClass::Store => DiagnosticClass::Infrastructure,
        },
        error.code,
        error.message,
    )
}

fn store_diagnostic(error: StoreError) -> Diagnostic {
    runtime_error(
        match error.class {
            StoreErrorClass::Input => DiagnosticClass::Source,
            StoreErrorClass::Resource => DiagnosticClass::Resource,
            StoreErrorClass::Corrupt => DiagnosticClass::Corrupt,
            StoreErrorClass::Io => DiagnosticClass::Infrastructure,
        },
        error.code,
        error.message,
    )
}

fn runtime_corrupt(code: &'static str, message: impl Into<String>) -> Diagnostic {
    runtime_error(DiagnosticClass::Corrupt, code, message)
}

fn runtime_error(
    class: DiagnosticClass,
    code: &'static str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}
