//! Exact point-read lowering from admitted canonical records into one compiler unit.

use super::unit::{
    BYTECODE_CONTRACT_VERSION, COMPILER_UNIT_CONTRACT_VERSION, CompilationPayload,
    CompilationSource, CompilationTables, CompilationUnit, CompilationUnitKey, CompiledCaseLayout,
    CompiledCode, CompiledFieldLayout, CompiledFieldSelector, CompiledHttpRoute,
    CompiledInstruction, CompiledOperationLayout, CompiledParameter, CompiledPort,
    CompiledPortImplementation, CompiledRequirement, CompiledSignature, CompiledText,
    CompiledTransactionOutcome, CompiledTransactionRequirement, CompiledVariantJump,
    MAXIMUM_COMPILER_UNIT_ITEMS, OptimizationPolicy,
};
use crate::platform::builtin_standard::BuiltinStandard;
use crate::platform::change::{
    CanonicalBaseRead, CanonicalRead, CanonicalReadWork, WitnessBaseRead, WitnessReadWork,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    BindingKind, BindingRecord, CallableResult, CaseReference, DeclarationPayload,
    DeclarationRecord, DeclarationReference, ExactOwnerKey, ExpressionOperation, ExpressionRead,
    ExpressionRecord, FieldReference, FieldSelector, FunctionEffect, LocalValueReference,
    OperationReference, OwnerKey, OwnerRecord, PackageId, PackageInterfaceRecord, ParameterParent,
    ParameterUse, PortImplementation, PortReference, ReadScopeArgument, ReadScopeBinder,
    ReadScopeParameter, ReadScopeSource, ReadViewField, ReadViewFieldKind, ReadViewFieldReference,
    RelationEndpoint, RelationKind, RequirementReference, ResultRelation, StructuralTypeField,
    TextValue, TypeForm, TypeObject, TypeObjectDigest, encode_type_object,
};
use crate::platform::package::RunnerKind;
use crate::platform::semantic_id::{
    BindingId, CaseId, DeclarationId, ExpressionId, FieldId, HttpRouteId, OperationId, ParameterId,
    PortId, RequirementId, TypeParameterId,
};
use crate::platform::session::{CanonicalSessionRead, validate_session_function_type};
use crate::platform::storage::object::ObjectKey;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};

struct SignatureGenerics<'a> {
    types: &'a [crate::platform::semantic_id::TypeParameterId],
    effects: &'a [crate::platform::semantic_id::EffectParameterId],
    requirements: &'a [crate::platform::semantic_id::RequirementParameterId],
    read_scope_parameters: &'a [ReadScopeParameter],
    read_scope_sources: &'a [ReadScopeSource<ParameterId>],
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CompilationWork {
    pub canonical: CanonicalReadWork,
    pub witness: WitnessReadWork,
    pub owner_records_read: u64,
    pub expression_records_read: u64,
    pub instructions_emitted: u64,
    pub maximum_expression_depth: u32,
    pub bytes_encoded: u64,
}

#[derive(Clone, Debug)]
pub struct CompilationReceipt {
    pub key: CompilationUnitKey,
    pub object: ObjectKey,
    pub unit: CompilationUnit,
    pub bytes: Vec<u8>,
    pub work: CompilationWork,
}

/// Compiles one declaration or target directly from exact normalized authority.
///
/// The reusable key excludes names, modules, and accepted revision identity. It binds the exact
/// semantic interface (including visibility), executable summary dimensions, and validation-
/// dependency digest instead. A stable rename or move can therefore reuse identical bytes, while
/// signature, body, type, effect, capability, and executable dependency changes cannot.
pub fn compile_unit<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    canonical: &B,
    witness: &W,
    owner: OwnerKey,
    optimization: OptimizationPolicy,
) -> Result<CompilationReceipt, Diagnostic> {
    if canonical.package_id() != witness.witness_package_id()
        || canonical.repository_id() != witness.witness_repository_id()
        || !witness.witness_contract_is_current()
    {
        return Err(compiler_error(
            DiagnosticClass::Corrupt,
            "compiler_unit_witness_binding",
            "canonical authority and validation witness do not form one current repository binding",
        ));
    }
    let (semantic_root, _) = crate::platform::kernel::encode_root(canonical.semantic_root())?;
    if semantic_root != witness.witness_manifest().semantic_root {
        return Err(compiler_error(
            DiagnosticClass::Corrupt,
            "compiler_unit_semantic_root",
            "validation witness is bound to another semantic root",
        ));
    }

    let summary_read = witness.read_owner_summary(owner)?;
    let mut builder = UnitBuilder {
        canonical,
        package: canonical.package_id(),
        scope: match owner {
            OwnerKey::Declaration(declaration) => Some(declaration),
            _ => None,
        },
        tables: TablesBuilder::default(),
        derived_type_objects: BTreeMap::new(),
        derived_type_admission: DerivedTypeAdmission::default(),
        work: CompilationWork {
            witness: summary_read.work,
            ..CompilationWork::default()
        },
    };
    let bound_summary = summary_read.value.ok_or_else(|| {
        compiler_error(
            DiagnosticClass::Corrupt,
            "compiler_unit_summary_missing",
            "compiler-unit owner has no committed owner summary",
        )
    })?;
    if bound_summary.summary.owner != owner {
        return Err(compiler_error(
            DiagnosticClass::Corrupt,
            "compiler_unit_summary_owner",
            "owner-summary binding returned another stable owner",
        ));
    }
    let record = builder.required_owner(owner, "compiler-unit source owner is missing")?;
    let (record_digest, _) = crate::platform::kernel::encode_owner(&record)?;
    if bound_summary.summary.record != record_digest || bound_summary.summary.kind != record.kind()
    {
        return Err(compiler_error(
            DiagnosticClass::Corrupt,
            "compiler_unit_summary_record",
            "compiler-unit source record disagrees with its committed owner summary",
        ));
    }
    let source = CompilationSource {
        package: builder.package,
        owner,
        kind: record.kind(),
        semantic_interface: bound_summary.summary.semantic_interface,
        implementation: bound_summary.summary.implementation,
        type_digest: bound_summary.summary.type_digest,
        effect: bound_summary.summary.effect,
        capability: bound_summary.summary.capability,
        test: bound_summary.summary.test,
        validation_dependencies: bound_summary.summary.validation_dependencies,
    };
    let key = CompilationUnitKey::derive(&source, optimization)?;
    let route_ids = target_route_ids(witness, owner, &mut builder.work)?;
    let payload = builder.compile_payload(owner, record, &route_ids)?;
    builder.complete_owned_record_type_closure()?;
    let tables = builder.tables.finish();
    let mut work = builder.work;
    let unit = CompilationUnit {
        contract_version: COMPILER_UNIT_CONTRACT_VERSION,
        graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
        bytecode_contract_version: BYTECODE_CONTRACT_VERSION,
        key,
        source,
        optimization,
        tables,
        payload,
    };
    let (object, bytes) = unit.encode()?;
    work.bytes_encoded = u64::try_from(bytes.len()).map_err(|_| {
        compiler_error(
            DiagnosticClass::Resource,
            "compiler_unit_encoded_length",
            "compiled-unit byte length does not fit its observation domain",
        )
    })?;
    Ok(CompilationReceipt {
        key,
        object,
        unit,
        bytes,
        work,
    })
}

fn target_route_ids<W: WitnessBaseRead + ?Sized>(
    witness: &W,
    owner: OwnerKey,
    work: &mut CompilationWork,
) -> Result<Vec<HttpRouteId>, Diagnostic> {
    if !matches!(owner, OwnerKey::Target(_)) {
        return Ok(Vec::new());
    }
    let read = witness.read_incoming_relations_of_kind(
        owner,
        RelationKind::HttpRouteTarget,
        Some(crate::platform::kernel::contract::MAXIMUM_HTTP_ROUTES_PER_TARGET.saturating_add(1)),
    )?;
    work.witness.add(read.work);
    if read.value.truncated {
        return Err(compiler_error(
            DiagnosticClass::Resource,
            "compiler_http_route_relation_limit",
            "HTTP target incoming route relations exceed the compiler bound",
        ));
    }
    let expected_target = RelationEndpoint::Owner(ExactOwnerKey {
        package: witness.witness_package_id(),
        owner,
    });
    let mut routes = Vec::new();
    for edge in read.value.edges {
        if edge.kind != RelationKind::HttpRouteTarget {
            continue;
        }
        if edge.target != expected_target {
            return Err(compiler_corrupt(
                "compiler_http_route_relation_target",
                "HTTP route relation disagrees with the selected target",
            ));
        }
        let RelationEndpoint::Owner(ExactOwnerKey {
            package,
            owner: OwnerKey::HttpRoute(route),
        }) = edge.source
        else {
            return Err(compiler_corrupt(
                "compiler_http_route_relation_source",
                "HTTP route relation has a foreign source",
            ));
        };
        if package != witness.witness_package_id() {
            return Err(compiler_corrupt(
                "compiler_http_route_relation_package",
                "HTTP route relation source belongs to another package",
            ));
        }
        routes.push(route);
    }
    routes.sort_unstable();
    if routes.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(compiler_corrupt(
            "compiler_http_route_relation_duplicate",
            "HTTP target repeats one route relation",
        ));
    }
    Ok(routes)
}

struct UnitBuilder<'a, B: ?Sized> {
    canonical: &'a B,
    package: PackageId,
    scope: Option<DeclarationId>,
    tables: TablesBuilder,
    derived_type_objects: BTreeMap<ObjectKey, Vec<u8>>,
    derived_type_admission: DerivedTypeAdmission,
    work: CompilationWork,
}

struct DerivedTypeAdmission {
    bytes: usize,
    maximum_bytes: usize,
    // Shared across every child in this unit, including unused witness metadata.
    parallel: crate::platform::kernel::owned_associated::OwnedAssociatedAdmission,
    parallel_steps: Cell<usize>,
}

impl Default for DerivedTypeAdmission {
    fn default() -> Self {
        Self {
            bytes: 0,
            maximum_bytes: 64 * 1_048_576,
            parallel: crate::platform::kernel::owned_associated::OwnedAssociatedAdmission::new(),
            parallel_steps: Cell::new(0),
        }
    }
}

#[path = "parallel_lower.rs"]
mod parallel_lower;

#[cfg(test)]
#[path = "derived_type_admission_tests.rs"]
mod derived_type_admission_tests;

#[cfg(test)]
#[path = "parallel_associated_lower_tests.rs"]
mod parallel_associated_lower_tests;

/// Exact point reads needed for expression lowering. Artifact admission supplies these from its
/// independently admitted canonical closure, without a repository or a producer's witness.
pub(super) trait CodeRead {
    fn code_dependency(&self, package: PackageId) -> Result<CanonicalRead<bool>, Diagnostic>;
    fn code_step(&self) -> Result<(), Diagnostic> {
        Ok(())
    }
    fn code_owner(&self, owner: OwnerKey)
    -> Result<CanonicalRead<Option<OwnerRecord>>, Diagnostic>;
    fn code_type(
        &self,
        ty: TypeObjectDigest,
    ) -> Result<CanonicalRead<Option<crate::platform::kernel::TypeObject>>, Diagnostic>;
    fn code_interface(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<PackageInterfaceRecord>>, Diagnostic>;
    fn code_representation_dependency(
        &self,
        package: PackageId,
    ) -> Result<CanonicalRead<bool>, Diagnostic> {
        self.code_dependency(package)
    }
    fn code_representation_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<PackageInterfaceRecord>>, Diagnostic> {
        self.code_interface(package, owner)
    }
}

fn representation_dependency<B: CanonicalBaseRead + ?Sized>(
    read: &B,
    package: PackageId,
) -> Result<CanonicalRead<Option<crate::platform::kernel::DependencyRecord>>, Diagnostic> {
    let roots = read.read_dependencies()?;
    let mut work = roots.work;
    let mut selected = None;
    for root in roots.value {
        read.validation_checkpoint()?;
        let dependency = read.read_representation_dependency(&root, package)?;
        work.add(dependency.work);
        if let Some(binding) = dependency.value {
            if binding.package != package
                || selected.as_ref().is_some_and(|prior| *prior != binding)
            {
                return Err(compiler_corrupt(
                    "compiler_representation_binding",
                    "exact imported representations bind one package to conflicting revisions",
                ));
            }
            selected = Some(binding);
        }
    }
    Ok(CanonicalRead {
        value: selected,
        work,
    })
}

impl<B: CanonicalBaseRead + ?Sized> CodeRead for B {
    fn code_step(&self) -> Result<(), Diagnostic> {
        self.validation_checkpoint()
    }
    fn code_dependency(&self, package: PackageId) -> Result<CanonicalRead<bool>, Diagnostic> {
        let r = self.read_dependency(package)?;
        Ok(CanonicalRead {
            value: r.value.is_some(),
            work: r.work,
        })
    }
    fn code_owner(
        &self,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<OwnerRecord>>, Diagnostic> {
        self.read_owner(owner)
    }
    fn code_type(
        &self,
        ty: TypeObjectDigest,
    ) -> Result<CanonicalRead<Option<crate::platform::kernel::TypeObject>>, Diagnostic> {
        self.read_type_object(ty)
    }
    fn code_interface(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<PackageInterfaceRecord>>, Diagnostic> {
        let dependency = self.read_dependency(package)?;
        let binding = dependency.value.ok_or_else(|| {
            compiler_corrupt(
                "compiler_dependency_missing",
                "exact compiler reference names an unbound dependency package",
            )
        })?;
        let mut read = self.read_package_interface_owner(&binding, owner)?;
        read.work.add(dependency.work);
        Ok(read)
    }
    fn code_representation_dependency(
        &self,
        package: PackageId,
    ) -> Result<CanonicalRead<bool>, Diagnostic> {
        let read = representation_dependency(self, package)?;
        Ok(CanonicalRead {
            value: read.value.is_some(),
            work: read.work,
        })
    }
    fn code_representation_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<PackageInterfaceRecord>>, Diagnostic> {
        let dependency = representation_dependency(self, package)?;
        let Some(binding) = dependency.value else {
            return Ok(CanonicalRead {
                value: None,
                work: dependency.work,
            });
        };
        let mut read = self.read_package_interface_owner(&binding, owner)?;
        read.work.add(dependency.work);
        Ok(read)
    }
}

/// The same nominal representation proof runs against repository point reads and the
/// independently admitted artifact closure. Derived substitutions remain compiler metadata.
struct CodeExpressionRead<'a, B: ?Sized> {
    canonical: &'a B,
    package: PackageId,
    derived: &'a BTreeMap<ObjectKey, Vec<u8>>,
    work: RefCell<CanonicalReadWork>,
    owners: Cell<u64>,
    parallel_steps: Option<&'a Cell<usize>>,
}

impl<B: CodeRead + ?Sized> crate::platform::kernel::ExpressionRead for CodeExpressionRead<'_, B> {
    fn package_id(&self) -> PackageId {
        self.package
    }

    fn owner(&self, owner: OwnerKey) -> Result<Option<OwnerRecord>, Diagnostic> {
        let read = self.canonical.code_owner(owner)?;
        self.work.borrow_mut().add(read.work);
        self.owners.set(self.owners.get().saturating_add(1));
        Ok(read.value)
    }

    fn type_object(&self, ty: TypeObjectDigest) -> Result<Option<TypeObject>, Diagnostic> {
        let key = ObjectKey::from_digest(
            crate::platform::storage::object::ObjectDomain::Type,
            ty.bytes(),
        );
        if let Some(bytes) = self.derived.get(&key) {
            self.canonical.code_step()?;
            return crate::platform::kernel::decode_type_object(bytes, ty).map(Some);
        }
        let read = self.canonical.code_type(ty)?;
        self.work.borrow_mut().add(read.work);
        Ok(read.value)
    }

    fn package_interface_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<PackageInterfaceRecord>, Diagnostic> {
        let read = self.canonical.code_interface(package, owner)?;
        self.work.borrow_mut().add(read.work);
        Ok(read.value)
    }

    fn has_dependency(&self, package: PackageId) -> Result<bool, Diagnostic> {
        let read = self.canonical.code_dependency(package)?;
        self.work.borrow_mut().add(read.work);
        Ok(read.value)
    }
    fn has_representation_dependency(&self, package: PackageId) -> Result<bool, Diagnostic> {
        let read = self.canonical.code_representation_dependency(package)?;
        self.work.borrow_mut().add(read.work);
        Ok(read.value)
    }
    fn package_representation_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<PackageInterfaceRecord>, Diagnostic> {
        let read = self.canonical.code_representation_owner(package, owner)?;
        self.work.borrow_mut().add(read.work);
        Ok(read.value)
    }

    fn validation_work(&self) -> Result<(), Diagnostic> {
        if let Some(steps) = self.parallel_steps {
            parallel_lower::proof_step(steps)?;
        }
        self.canonical.code_step()
    }
}

pub(super) fn canonical_code<B: CodeRead + ?Sized>(
    read: &B,
    package: PackageId,
    scope: Option<DeclarationId>,
    tables: &CompilationTables,
    root: ExpressionId,
    parameters: &[ParameterId],
) -> Result<CompiledCode, Diagnostic> {
    let mut builder = UnitBuilder {
        canonical: read,
        package,
        scope,
        tables: TablesBuilder::from_tables(tables),
        derived_type_objects: BTreeMap::new(),
        derived_type_admission: DerivedTypeAdmission::default(),
        work: CompilationWork::default(),
    };
    builder.compile_code(root, parameters)
}

/// Inferred expression types are recomputable metadata. Recreate their bytes at
/// the artifact boundary, including when an unchanged compiler unit was cached;
/// no operational compiler cache write adds objects to accepted semantic authority.
pub(super) fn reconstruct_parallel_result_types<B: CanonicalBaseRead + ?Sized>(
    read: &B,
    unit: &CompilationUnit,
    remaining: &mut usize,
) -> Result<(BTreeMap<ObjectKey, Vec<u8>>, CanonicalReadWork), Diagnostic> {
    if unit.source.package != read.package_id() {
        return Err(compiler_corrupt(
            "compiler_parallel_source",
            "cached unit belongs to a different canonical package",
        ));
    }
    let allowance = Cell::new(*remaining);
    let result = (|| {
        let read = parallel_lower::ReconstructionRead {
            source: read,
            remaining: &allowance,
        };
        // Reserve every copied table entry before constructing the indexed tables.
        // Instruction/owner/type walks then debit the same borrowed allowance.
        for count in [
            unit.tables.declarations.len(),
            unit.tables.fields.len(),
            unit.tables.cases.len(),
            unit.tables.requirements.len(),
            unit.tables.operations.len(),
            unit.tables.ports.len(),
            unit.tables.types.len(),
            unit.tables.structural_names.len(),
            unit.tables.read_view_fields.len(),
            unit.tables.texts.len(),
        ] {
            read.reserve(count)?;
        }
        let mut builder = UnitBuilder {
            canonical: &read,
            package: unit.source.package,
            scope: match unit.source.owner {
                OwnerKey::Declaration(declaration) => Some(declaration),
                _ => None,
            },
            tables: TablesBuilder::from_tables(&unit.tables),
            derived_type_objects: BTreeMap::new(),
            derived_type_admission: DerivedTypeAdmission::default(),
            work: CompilationWork::default(),
        };
        // Re-lower exact canonical roots, not cached instruction declarations. This
        // recovers both child expressions, their own headers, the wrapper header,
        // all witness operands and the original lexical owner without fabricated
        // generations. compile_code does not enter this reconstruction path.
        let record =
            builder.required_owner(unit.source.owner, "cached unit canonical source is absent")?;
        let mut check =
            |root, parameters: &[ParameterId], actual: &CompiledCode| -> Result<(), Diagnostic> {
                read.reserve(parameters.len())?;
                let expected = builder.compile_code(root, parameters)?;
                if expected != *actual {
                    return Err(compiler_corrupt(
                        "compiler_parallel_control_meaning",
                        "cached code differs from its exact canonical source applications",
                    ));
                }
                Ok(())
            };
        match (&unit.payload, record) {
            (CompilationPayload::Function { code, .. }, OwnerRecord::Declaration(record)) => {
                let DeclarationPayload::Function(function) = record.payload else {
                    return Err(reconstruction_source());
                };
                check(function.body, &function.parameters, code)?;
            }
            (CompilationPayload::Constant { code, .. }, OwnerRecord::Declaration(record)) => {
                let DeclarationPayload::Constant { value, .. } = record.payload else {
                    return Err(reconstruction_source());
                };
                check(value, &[], code)?;
            }
            (
                CompilationPayload::Test {
                    actual, expected, ..
                },
                OwnerRecord::Declaration(record),
            ) => {
                let DeclarationPayload::Test {
                    actual: root,
                    expected: expected_root,
                    ..
                } = record.payload
                else {
                    return Err(reconstruction_source());
                };
                check(root, &[], actual)?;
                check(expected_root, &[], expected)?;
            }
            (CompilationPayload::Component { ports, .. }, OwnerRecord::Declaration(record)) => {
                if !matches!(record.payload, DeclarationPayload::Component { .. }) {
                    return Err(reconstruction_source());
                }
                // Release the closure borrow for exact canonical port reads.
                for port in ports {
                    let CompiledPortImplementation::Expression(actual) = &port.implementation
                    else {
                        continue;
                    };
                    let reference = unit
                        .tables
                        .ports
                        .get(port.port as usize)
                        .ok_or_else(reconstruction_source)?;
                    if reference.package != unit.source.package {
                        return Err(reconstruction_source());
                    }
                    let Some(OwnerRecord::Port(record)) =
                        read.code_owner(OwnerKey::Port(reference.port))?.value
                    else {
                        return Err(reconstruction_source());
                    };
                    let PortImplementation::Expression(root) = record.implementation else {
                        return Err(reconstruction_source());
                    };
                    check(root, &[], actual)?;
                }
            }
            (
                CompilationPayload::Function { .. }
                | CompilationPayload::Constant { .. }
                | CompilationPayload::Test { .. }
                | CompilationPayload::Component { .. },
                _,
            ) => return Err(reconstruction_source()),
            _ => {}
        }
        // A nominal can occur only inside a table root's syntactic children.
        // Recreate all reachable nominal fields before the strict cached-table
        // walk, whose reverse order may encounter a derived field first. Parallel
        // roots have already been recovered by canonical re-lowering above.
        read.reserve(unit.tables.types.len())?;
        builder
            .derived_type_admission
            .parallel
            .reserve::<TypeObjectDigest>(unit.tables.types.len())?;
        let mut pending = unit.tables.types.clone();
        let mut seen = BTreeSet::new();
        while let Some(ty) = pending.pop() {
            // Charge duplicate and missing attempts too. A missing root is never
            // marked seen: a later reachable layout can derive and requeue it.
            read.reserve(1)?;
            if seen.contains(&ty) {
                continue;
            }
            let Some(object) = builder.with_expression_read(|read| read.type_object(ty))? else {
                continue;
            };
            if seen.len() >= MAXIMUM_COMPILER_UNIT_ITEMS {
                return Err(compiler_error(
                    DiagnosticClass::Resource,
                    "compiler_owned_record_type_limit",
                    "owned record type closure exceeds the compiler-unit bound",
                ));
            }
            read.reserve(1)?;
            builder
                .derived_type_admission
                .parallel
                .reserve::<TypeObjectDigest>(1)?;
            seen.insert(ty);
            let children = object.child_type_count();
            read.reserve(children)?;
            // Reserve both the transient child vector and the pending entries
            // before child_types allocates or the worklist grows.
            builder
                .derived_type_admission
                .parallel
                .reserve::<TypeObjectDigest>(children)?;
            builder
                .derived_type_admission
                .parallel
                .reserve::<TypeObjectDigest>(children)?;
            pending.extend(object.child_types());
            if matches!(&object.form, TypeForm::ReadViewApplied { .. }) {
                let fields = builder
                    .read_view_layout(
                        ty,
                        crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
                        false,
                    )?
                    .fields;
                read.reserve(fields.len())?;
                builder
                    .derived_type_admission
                    .parallel
                    .reserve::<TypeObjectDigest>(fields.len())?;
                pending.extend(fields.into_iter().map(|field| field.ty));
            }
            if matches!(
                object.form,
                TypeForm::Named { .. } | TypeForm::Applied { .. }
            ) {
                let package = builder.package;
                if builder.with_expression_read(|read| {
                    crate::platform::kernel::owned_record::is_owned(read, package, ty)
                })? {
                    let fields = builder.owned_record_fields(ty, false)?;
                    read.reserve(fields.len())?;
                    builder
                        .derived_type_admission
                        .parallel
                        .reserve::<TypeObjectDigest>(fields.len())?;
                    pending.extend(fields.into_iter().map(|(_, ty)| ty));
                }
            }
        }
        // The seed pass defers absent derived entries, never admits them. Every
        // cached table root must now resolve through the complete strict closure.
        builder.complete_owned_record_type_closure()?;
        Ok((builder.derived_type_objects, builder.work.canonical))
    })();
    *remaining = allowance.get();
    result
}

fn reconstruction_source() -> Diagnostic {
    compiler_corrupt(
        "compiler_parallel_source",
        "cached executable has no exact canonical payload root",
    )
}

impl<B: CanonicalBaseRead + ?Sized> UnitBuilder<'_, B> {
    fn compile_payload(
        &mut self,
        selected: OwnerKey,
        record: OwnerRecord,
        route_ids: &[HttpRouteId],
    ) -> Result<CompilationPayload, Diagnostic> {
        match (selected, record) {
            (OwnerKey::Declaration(declaration), OwnerRecord::Declaration(record)) => {
                self.compile_declaration(declaration, record)
            }
            (OwnerKey::Target(target_id), OwnerRecord::Target(record)) => {
                let component = self.tables.declaration(record.component)?;
                if record.runner == RunnerKind::Http {
                    if record.port.is_some() {
                        return Err(compiler_corrupt(
                            "compiler_http_target_universal_port",
                            "HTTP target retained a universal port",
                        ));
                    }
                    let mut routes = Vec::with_capacity(route_ids.len());
                    for route_id in route_ids {
                        let OwnerRecord::HttpRoute(route) = self.required_owner(
                            OwnerKey::HttpRoute(*route_id),
                            "HTTP target route owner is missing",
                        )?
                        else {
                            return Err(compiler_corrupt(
                                "compiler_http_route_kind",
                                "HTTP route identity names another owner kind",
                            ));
                        };
                        if route.target != target_id {
                            return Err(compiler_corrupt(
                                "compiler_http_route_target",
                                "HTTP route belongs to another target",
                            ));
                        }
                        if route.port.package != self.package {
                            return Err(compiler_corrupt(
                                "compiler_http_route_port_package",
                                "HTTP route port belongs to another package",
                            ));
                        }
                        let OwnerRecord::Port(port) = self.required_owner(
                            OwnerKey::Port(route.port.port),
                            "HTTP route references a missing port",
                        )?
                        else {
                            return Err(compiler_corrupt(
                                "compiler_http_route_port_kind",
                                "HTTP route port identity names another owner kind",
                            ));
                        };
                        let capture_count = route.selector.capture_count();
                        let mut http_types = crate::platform::kernel::TypeObjectInterner::default();
                        let read = self.canonical.read_type_object(port.function_type)?;
                        self.work.canonical.add(read.work);
                        let shape = read.value.ok_or_else(|| {
                            compiler_corrupt(
                                "compiler_http_route_port_type",
                                "HTTP route callable type is absent",
                            )
                        })?;
                        let matches_shape = crate::platform::http::has_semantic_http_route_shape(
                            &shape.form,
                            capture_count,
                        )?;
                        let PortImplementation::Function(function) = port.implementation else {
                            return Err(compiler_corrupt(
                                "compiler_http_route_port_relation",
                                "HTTP route port has the wrong component, shape, or implementation",
                            ));
                        };
                        if port.declaration != record.component.declaration || !matches_shape {
                            return Err(compiler_corrupt(
                                "compiler_http_route_port_relation",
                                "HTTP route port has the wrong component or selector-indexed shape",
                            ));
                        }
                        let (parameters, result) = self.http_function_parameters(function)?;
                        let semantic_http =
                            crate::platform::http::semantic_http_types(&mut http_types)?;
                        if parameters.len() != capture_count.saturating_add(1)
                            || parameters
                                .first()
                                .is_none_or(|parameter| parameter.ty != semantic_http.request_type)
                            || result != semantic_http.response_type
                        {
                            return Err(compiler_corrupt(
                                "compiler_http_route_function_signature",
                                "HTTP route backing function disagrees with the request/capture/response contract",
                            ));
                        }
                        let capture_names = route.selector.capture_names();
                        for (parameter, capture) in parameters.iter().skip(1).zip(&capture_names) {
                            if parameter.name.as_str() != capture.as_str()
                                || parameter.ty != semantic_http.text_type
                                || parameter.use_mode != ParameterUse::Unrestricted
                                || parameter.resource_requirement.is_some()
                            {
                                return Err(compiler_corrupt(
                                    "compiler_http_route_capture_parameter",
                                    "HTTP route capture disagrees with its indexed function parameter",
                                ));
                            }
                        }
                        routes.push(CompiledHttpRoute {
                            route: *route_id,
                            method: route.method,
                            selector: route.selector,
                            port: self.tables.port(route.port)?,
                            capture_parameters: parameters
                                .iter()
                                .skip(1)
                                .map(|parameter| match parameter.header.owner {
                                    OwnerKey::Parameter(parameter) => Ok(parameter),
                                    _ => Err(compiler_corrupt(
                                        "compiler_http_route_capture_parameter_identity",
                                        "HTTP route capture parameter has another owner identity",
                                    )),
                                })
                                .collect::<Result<Vec<_>, _>>()?,
                        });
                    }
                    routes.sort_by(|left, right| {
                        left.method
                            .as_bytes()
                            .cmp(right.method.as_bytes())
                            .then_with(|| {
                                crate::platform::kernel::http_route_selector_cmp(
                                    &left.selector,
                                    &right.selector,
                                )
                            })
                    });
                    return Ok(CompilationPayload::Target {
                        component,
                        port: None,
                        routes,
                        runner: record.runner,
                    });
                }
                if !route_ids.is_empty() {
                    return Err(compiler_corrupt(
                        "compiler_non_http_routes",
                        "non-HTTP target owns HTTP routes",
                    ));
                }
                let port_reference = record.port.ok_or_else(|| {
                    compiler_corrupt(
                        "compiler_target_port_missing",
                        "non-HTTP target has no exact port",
                    )
                })?;
                if record.runner == RunnerKind::Interactive {
                    if port_reference.package != self.package {
                        return Err(compiler_corrupt(
                            "compiler_session_port_package",
                            "interactive target port must belong to the compiled root package",
                        ));
                    }
                    let OwnerRecord::Port(port_record) = self.required_owner(
                        OwnerKey::Port(port_reference.port),
                        "interactive target references a missing port",
                    )?
                    else {
                        return Err(compiler_corrupt(
                            "compiler_session_port_kind",
                            "interactive target port identity names another owner kind",
                        ));
                    };
                    let standard = BuiltinStandard::load()?.session_contract()?;
                    let session_read = CanonicalSessionRead::new(self.canonical);
                    let validation = validate_session_function_type(
                        &session_read,
                        standard,
                        port_record.function_type,
                    );
                    self.work.canonical.add(session_read.work());
                    let _ = validation?;
                }
                let port = self.tables.port(port_reference)?;
                Ok(CompilationPayload::Target {
                    component,
                    port: Some(port),
                    routes: Vec::new(),
                    runner: record.runner,
                })
            }
            _ => Err(compiler_error(
                DiagnosticClass::Semantic,
                "compiler_unit_owner",
                "only declarations and targets are normalized compiler units",
            )),
        }
    }

    fn compile_declaration(
        &mut self,
        declaration: DeclarationId,
        record: DeclarationRecord,
    ) -> Result<CompilationPayload, Diagnostic> {
        match record.payload {
            DeclarationPayload::ReadView(view) => {
                let reference = DeclarationReference {
                    package: self.package,
                    declaration,
                };
                view.validate_local_at(reference)?;
                let type_parameter_constraints =
                    self.compile_type_parameter_constraints(declaration, &view.type_parameters)?;
                self.tables.declaration(reference)?;
                for field in &view.fields {
                    self.canonical.code_step()?;
                    self.tables.ty(field.ty)?;
                    self.tables.read_view_field(ReadViewFieldReference {
                        declaration: reference,
                        field: field.id,
                    })?;
                }
                Ok(CompilationPayload::ReadView {
                    declaration: view,
                    type_parameter_constraints,
                })
            }
            DeclarationPayload::OwnedContract(c) => {
                let parameters = std::iter::once(c.self_parameter)
                    .chain(c.type_parameters.iter().copied())
                    .collect::<Vec<_>>();
                self.compile_type_parameter_constraints(declaration, &parameters)?;
                for ty in c.type_roots() {
                    self.tables.ty(ty)?;
                }
                Ok(CompilationPayload::OwnedContract(c))
            }
            DeclarationPayload::OwnedImplementation(i) => {
                self.compile_type_parameter_constraints(declaration, &i.type_parameters)?;
                for binding in &i.associated_types {
                    self.tables.ty(binding.value.ty())?;
                }
                self.tables.declaration(i.contract)?;
                self.tables.ty(i.self_type)?;
                for ty in &i.type_arguments {
                    self.tables.ty(*ty)?;
                }
                for parameter in &i.implementation_parameters {
                    self.tables.declaration(parameter.contract)?;
                    self.tables.ty(parameter.self_type)?;
                    for ty in &parameter.type_arguments {
                        self.tables.ty(*ty)?;
                    }
                }
                for m in &i.methods {
                    self.tables.declaration(m.function)?;
                    for ty in &m.type_arguments {
                        self.tables.ty(*ty)?;
                    }
                    for operand in &m.implementations {
                        self.implementation_operand(operand)?;
                    }
                }
                Ok(CompilationPayload::OwnedImplementation(i))
            }
            DeclarationPayload::OwnedRecord {
                fields,
                type_parameters,
            } => {
                let type_parameter_constraints =
                    self.compile_type_parameter_constraints(declaration, &type_parameters)?;
                let mut compiled = Vec::with_capacity(fields.len());
                for field in fields {
                    let field_record = self.required_field(field, declaration)?;
                    compiled.push(CompiledFieldLayout {
                        field: self.tables.field(FieldReference {
                            package: self.package,
                            field,
                        })?,
                        ty: self.tables.ty(field_record.ty)?,
                    });
                }
                Ok(CompilationPayload::OwnedRecord {
                    type_parameters,
                    type_parameter_constraints,
                    fields: compiled,
                })
            }
            DeclarationPayload::Record {
                fields,
                type_parameters,
            } => {
                let type_parameter_constraints =
                    self.compile_type_parameter_constraints(declaration, &type_parameters)?;
                let mut compiled = Vec::with_capacity(fields.len());
                for field in fields {
                    let field_record = self.required_field(field, declaration)?;
                    compiled.push(CompiledFieldLayout {
                        field: self.tables.field(FieldReference {
                            package: self.package,
                            field,
                        })?,
                        ty: self.tables.ty(field_record.ty)?,
                    });
                }
                Ok(CompilationPayload::Record {
                    type_parameters,
                    type_parameter_constraints,
                    fields: compiled,
                })
            }
            DeclarationPayload::Variant {
                cases,
                type_parameters,
            } => {
                let type_parameter_constraints =
                    self.compile_type_parameter_constraints(declaration, &type_parameters)?;
                let mut compiled = Vec::with_capacity(cases.len());
                for case in cases {
                    let case_record = self.required_case(case, declaration)?;
                    compiled.push(CompiledCaseLayout {
                        case: self.tables.case(CaseReference {
                            package: self.package,
                            case,
                        })?,
                        payload: case_record
                            .payload
                            .map(|payload| self.tables.ty(payload))
                            .transpose()?,
                    });
                }
                Ok(CompilationPayload::Variant {
                    type_parameters,
                    type_parameter_constraints,
                    cases: compiled,
                })
            }
            DeclarationPayload::Interface { operations } => {
                let mut compiled = Vec::with_capacity(operations.len());
                for operation in operations {
                    compiled.push(self.compile_operation(operation, declaration)?);
                }
                Ok(CompilationPayload::Interface {
                    operations: compiled,
                })
            }
            DeclarationPayload::External(external) => {
                let signature = self.compile_signature(
                    declaration,
                    SignatureGenerics {
                        types: &external.type_parameters,
                        effects: &[],
                        requirements: &[],
                        read_scope_parameters: &[],
                        read_scope_sources: &[],
                    },
                    &external.parameters,
                    CallableResult {
                        ty: external.result,
                        relation: ResultRelation::Value,
                    },
                    &FunctionEffect::Pure,
                )?;
                Ok(CompilationPayload::External {
                    signature,
                    implementation: external.implementation,
                })
            }
            DeclarationPayload::Function(function) => {
                for p in &function.implementation_parameters {
                    self.tables.ty(p.self_type)?;
                    for ty in &p.type_arguments {
                        self.tables.ty(*ty)?;
                    }
                    self.tables.declaration(p.contract)?;
                }
                let mut signature = self.compile_signature(
                    declaration,
                    SignatureGenerics {
                        types: &function.type_parameters,
                        effects: &function.effect_parameters,
                        requirements: &function.requirement_parameters,
                        read_scope_parameters: &function.read_scope_parameters,
                        read_scope_sources: &function.read_scope_sources,
                    },
                    &function.parameters,
                    function.result,
                    &function.effect,
                )?;
                signature.implementation_parameters = function.implementation_parameters;
                let code = self.compile_code(function.body, &function.parameters)?;
                Ok(CompilationPayload::Function { signature, code })
            }
            DeclarationPayload::Constant { ty, value } => Ok(CompilationPayload::Constant {
                ty: self.tables.ty(ty)?,
                code: self.compile_code(value, &[])?,
            }),
            DeclarationPayload::Component {
                requirements,
                ports,
            } => {
                let requirements = requirements
                    .into_iter()
                    .map(|requirement| self.compile_requirement(requirement, declaration))
                    .collect::<Result<Vec<_>, _>>()?;
                let ports = ports
                    .into_iter()
                    .map(|port| self.compile_port(port, declaration))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(CompilationPayload::Component {
                    requirements,
                    ports,
                })
            }
            DeclarationPayload::Test {
                actual,
                expected,
                comparison,
            } => Ok(CompilationPayload::Test {
                actual: self.compile_code(actual, &[])?,
                expected: self.compile_code(expected, &[])?,
                comparison,
            }),
        }
    }

    fn compile_type_parameter_constraints(
        &mut self,
        declaration: DeclarationId,
        parameters: &[TypeParameterId],
    ) -> Result<Vec<crate::platform::kernel::TypeParameterConstraints>, Diagnostic> {
        let mut constraints = Vec::new();
        for parameter in parameters {
            match self.required_owner(
                OwnerKey::TypeParameter(*parameter),
                "declaration references a missing type parameter",
            )? {
                OwnerRecord::TypeParameter(record) if record.declaration == declaration => {
                    constraints.push(record.constraints)
                }
                _ => {
                    return Err(compiler_corrupt(
                        "compiler_unit_type_parameter_parent",
                        "type parameter has no exact declaration owner",
                    ));
                }
            }
        }
        Ok(constraints)
    }

    fn compile_signature(
        &mut self,
        declaration: DeclarationId,
        generics: SignatureGenerics<'_>,
        parameters: &[ParameterId],
        result: CallableResult<ParameterId>,
        effect: &FunctionEffect,
    ) -> Result<CompiledSignature, Diagnostic> {
        let SignatureGenerics {
            types: type_parameters,
            effects: effect_parameters,
            requirements: requirement_parameters,
            read_scope_parameters,
            read_scope_sources,
        } = generics;
        let type_parameter_constraints =
            self.compile_type_parameter_constraints(declaration, type_parameters)?;
        let mut compiled_parameters = Vec::with_capacity(parameters.len());
        for parameter in parameters {
            let parameter_record = self.required_parameter(
                *parameter,
                ParameterParent::Function(declaration),
                "function parameter",
            )?;
            compiled_parameters.push(CompiledParameter {
                parameter: *parameter,
                name: parameter_record.name.clone(),
                ty: self.tables.ty(parameter_record.ty)?,
                use_mode: parameter_record.use_mode,
                resource_requirement: parameter_record
                    .resource_requirement
                    .map(|requirement| self.tables.requirement(requirement))
                    .transpose()?,
            });
        }
        crate::platform::kernel::read_view::validate_read_scope_inventory_at(
            ReadScopeBinder::Declaration(DeclarationReference {
                package: self.package,
                declaration,
            }),
            read_scope_parameters,
            read_scope_sources,
            |source| {
                compiled_parameters.iter().any(|parameter| {
                    parameter.parameter == source && parameter.use_mode == ParameterUse::Borrow
                })
            },
        )?;
        self.derived_type_admission
            .parallel
            .reserve::<ReadScopeParameter>(read_scope_parameters.len())?;
        self.derived_type_admission
            .parallel
            .reserve::<ReadScopeSource<ParameterId>>(read_scope_sources.len())?;
        for parameter in read_scope_parameters {
            self.canonical.code_step()?;
            self.derived_type_admission
                .parallel
                .reserve_bytes(parameter.name.as_str().len())?;
        }
        for source in read_scope_sources {
            self.canonical.code_step()?;
            self.tables.declaration(source.scope.declaration())?;
        }
        let task_requirements = match effect {
            FunctionEffect::Pure => Vec::new(),
            FunctionEffect::Task {
                effect_parameters: _,
                requirements,
            } => requirements
                .iter()
                .filter_map(|reference| reference.concrete())
                .map(|reference| self.tables.requirement(reference))
                .collect::<Result<Vec<_>, _>>()?,
        };
        Ok(CompiledSignature {
            implementation_parameters: Vec::new(),
            requirement_parameters: requirement_parameters.to_vec(),
            effect_parameters: effect_parameters.to_vec(),
            effect: effect.clone(),
            type_parameters: type_parameters.to_vec(),
            type_parameter_constraints,
            parameters: compiled_parameters,
            result: result.try_map_type(|ty| self.tables.ty(ty))?,
            read_scope_parameters: read_scope_parameters.to_vec(),
            read_scope_sources: read_scope_sources.to_vec(),
            task_requirements,
        })
    }

    fn compile_operation(
        &mut self,
        operation: OperationId,
        declaration: DeclarationId,
    ) -> Result<CompiledOperationLayout, Diagnostic> {
        let record = match self.required_owner(
            OwnerKey::Operation(operation),
            "interface references a missing operation",
        )? {
            OwnerRecord::Operation(record) if record.declaration == declaration => record,
            OwnerRecord::Operation(_) => {
                return Err(compiler_corrupt(
                    "compiler_unit_operation_parent",
                    "interface operation belongs to another declaration",
                ));
            }
            _ => {
                return Err(compiler_corrupt(
                    "compiler_unit_operation_kind",
                    "interface operation identity names another owner kind",
                ));
            }
        };
        let mut parameters = Vec::with_capacity(record.parameters.len());
        for parameter in &record.parameters {
            let parameter_record = self.required_parameter(
                *parameter,
                ParameterParent::Operation(operation),
                "operation parameter",
            )?;
            parameters.push(CompiledParameter {
                parameter: *parameter,
                name: parameter_record.name.clone(),
                ty: self.tables.ty(parameter_record.ty)?,
                use_mode: parameter_record.use_mode,
                resource_requirement: None,
            });
        }
        Ok(CompiledOperationLayout {
            operation: self.tables.operation(OperationReference {
                package: self.package,
                operation,
            })?,
            parameters,
            result: self.tables.ty(record.result)?,
            idempotency: record.idempotency,
            external_visibility: record.external_visibility,
        })
    }

    fn compile_requirement(
        &mut self,
        requirement: RequirementId,
        declaration: DeclarationId,
    ) -> Result<CompiledRequirement, Diagnostic> {
        let record = match self.required_owner(
            OwnerKey::Requirement(requirement),
            "component references a missing requirement",
        )? {
            OwnerRecord::Requirement(record) if record.declaration == declaration => record,
            OwnerRecord::Requirement(_) => {
                return Err(compiler_corrupt(
                    "compiler_unit_requirement_parent",
                    "component requirement belongs to another declaration",
                ));
            }
            _ => {
                return Err(compiler_corrupt(
                    "compiler_unit_requirement_kind",
                    "component requirement identity names another owner kind",
                ));
            }
        };
        Ok(CompiledRequirement {
            requirement: self.tables.requirement(RequirementReference {
                package: self.package,
                requirement,
            })?,
            interface: self.tables.declaration(record.interface)?,
            operations: record
                .operations
                .iter()
                .map(|operation| self.tables.operation(*operation))
                .collect::<Result<Vec<_>, _>>()?,
            limits: record.limits,
        })
    }

    fn compile_port(
        &mut self,
        port: PortId,
        declaration: DeclarationId,
    ) -> Result<CompiledPort, Diagnostic> {
        let record = match self
            .required_owner(OwnerKey::Port(port), "component references a missing port")?
        {
            OwnerRecord::Port(record) if record.declaration == declaration => record,
            OwnerRecord::Port(_) => {
                return Err(compiler_corrupt(
                    "compiler_unit_port_parent",
                    "component port belongs to another declaration",
                ));
            }
            _ => {
                return Err(compiler_corrupt(
                    "compiler_unit_port_kind",
                    "component port identity names another owner kind",
                ));
            }
        };
        let implementation = match record.implementation {
            PortImplementation::Function(function) => {
                self.require_value_callable(function)?;
                CompiledPortImplementation::Function(self.tables.declaration(function)?)
            }
            PortImplementation::Expression(expression) => {
                CompiledPortImplementation::Expression(self.compile_code(expression, &[])?)
            }
        };
        Ok(CompiledPort {
            port: self.tables.port(PortReference {
                package: self.package,
                port,
            })?,
            function_type: self.tables.ty(record.function_type)?,
            implementation,
        })
    }
}

impl<B: CodeRead + ?Sized> UnitBuilder<'_, B> {
    fn retain_derived_type(
        &mut self,
        expected: Option<TypeObjectDigest>,
        object: &TypeObject,
    ) -> Result<TypeObjectDigest, Diagnostic> {
        if let Some(digest) = expected
            && self
                .derived_type_objects
                .contains_key(&ObjectKey::from_digest(
                    crate::platform::storage::object::ObjectDomain::Type,
                    digest.bytes(),
                ))
        {
            return Ok(digest);
        }
        if self.derived_type_objects.len() >= MAXIMUM_COMPILER_UNIT_ITEMS {
            return Err(compiler_error(
                DiagnosticClass::Resource,
                "compiler_derived_type_storage",
                "derived type inventory exceeds the compiler-unit bound",
            ));
        }
        // Count canonical payload bytes without constructing a buffer. Reserve the
        // retained map entry and temporary encoding buffers before producing them.
        let mut size = bincode::enc::write::SizeWriter::default();
        bincode::encode_into_writer(
            object,
            &mut size,
            bincode::config::standard()
                .with_little_endian()
                .with_variable_int_encoding(),
        )
        .map_err(|error| {
            compiler_corrupt(
                "compiler_derived_type_encoding",
                format!("derived type size admission failed: {error}"),
            )
        })?;
        let reserve = size
            .bytes_written
            .checked_add(256)
            .and_then(|bytes| bytes.checked_mul(4))
            .and_then(|bytes| {
                bytes.checked_add(
                    std::mem::size_of::<(ObjectKey, Vec<u8>)>() + 4 * std::mem::size_of::<usize>(),
                )
            })
            .ok_or_else(|| {
                compiler_error(
                    DiagnosticClass::Resource,
                    "compiler_derived_type_storage",
                    "derived type metadata size overflow",
                )
            })?;
        self.derived_type_admission.bytes = self
            .derived_type_admission
            .bytes
            .checked_add(reserve)
            .filter(|bytes| *bytes <= self.derived_type_admission.maximum_bytes)
            .ok_or_else(|| {
                compiler_error(
                    DiagnosticClass::Resource,
                    "compiler_derived_type_storage",
                    "cumulative derived type metadata exceeds its storage admission",
                )
            })?;
        let (digest, bytes) = encode_type_object(object)?;
        if expected.is_some_and(|expected| expected != digest) {
            return Err(compiler_corrupt(
                "compiler_owned_record_type",
                "derived field type has another digest",
            ));
        }
        let key = ObjectKey::from_digest(
            crate::platform::storage::object::ObjectDomain::Type,
            digest.bytes(),
        );
        match self.derived_type_objects.entry(key) {
            std::collections::btree_map::Entry::Occupied(_) => {
                self.derived_type_admission.bytes -= reserve;
            }
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(bytes);
            }
        }
        Ok(digest)
    }

    fn complete_owned_record_type_closure(&mut self) -> Result<(), Diagnostic> {
        self.derived_type_admission
            .parallel
            .reserve::<TypeObjectDigest>(self.tables.types.values.len())?;
        let mut pending = self.tables.types.values.clone();
        let mut seen = BTreeSet::new();
        while let Some(ty) = pending.pop() {
            self.canonical.code_step()?;
            if !seen.insert(ty) {
                continue;
            }
            if seen.len() > MAXIMUM_COMPILER_UNIT_ITEMS {
                return Err(compiler_error(
                    DiagnosticClass::Resource,
                    "compiler_owned_record_type_limit",
                    "owned record type closure exceeds the compiler-unit bound",
                ));
            }
            let object = self
                .with_expression_read(|read| read.type_object(ty))?
                .ok_or_else(|| {
                    compiler_corrupt(
                        "compiler_owned_record_type",
                        "type closure omits an exact type object",
                    )
                })?;
            self.derived_type_admission
                .parallel
                .reserve::<TypeObjectDigest>(object.child_type_count())?;
            self.derived_type_admission
                .parallel
                .reserve::<TypeObjectDigest>(object.child_type_count())?;
            pending.extend(object.child_types());
            if matches!(&object.form, TypeForm::ReadViewApplied { .. }) {
                let fields = self
                    .read_view_layout(
                        ty,
                        crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
                        false,
                    )?
                    .fields;
                self.derived_type_admission
                    .parallel
                    .reserve::<TypeObjectDigest>(fields.len())?;
                pending.extend(fields.into_iter().map(|field| field.ty));
            }
            if matches!(
                object.form,
                TypeForm::Named { .. } | TypeForm::Applied { .. }
            ) {
                let package = self.package;
                if self.with_expression_read(|read| {
                    crate::platform::kernel::owned_record::is_owned(read, package, ty)
                })? {
                    pending.extend(
                        self.owned_record_fields(ty, false)?
                            .into_iter()
                            .map(|(_, ty)| ty),
                    );
                }
            }
        }
        Ok(())
    }

    fn with_expression_read<T>(
        &mut self,
        operation: impl FnOnce(&CodeExpressionRead<'_, B>) -> Result<T, Diagnostic>,
    ) -> Result<T, Diagnostic> {
        let read = CodeExpressionRead {
            canonical: self.canonical,
            package: self.package,
            derived: &self.derived_type_objects,
            work: RefCell::new(CanonicalReadWork::default()),
            owners: Cell::new(0),
            parallel_steps: None,
        };
        let result = operation(&read);
        self.work.canonical.add(read.work.into_inner());
        self.work.owner_records_read = self
            .work
            .owner_records_read
            .saturating_add(read.owners.get());
        result
    }

    fn with_admission_read<T>(
        &mut self,
        operation: impl FnOnce(
            &CodeExpressionRead<'_, B>,
            &crate::platform::kernel::owned_associated::OwnedAssociatedAdmission,
        ) -> Result<T, Diagnostic>,
    ) -> Result<T, Diagnostic> {
        let read = CodeExpressionRead {
            canonical: self.canonical,
            package: self.package,
            derived: &self.derived_type_objects,
            work: RefCell::new(CanonicalReadWork::default()),
            owners: Cell::new(0),
            parallel_steps: Some(&self.derived_type_admission.parallel_steps),
        };
        let result = operation(&read, &self.derived_type_admission.parallel);
        self.work.canonical.add(read.work.into_inner());
        self.work.owner_records_read = self
            .work
            .owner_records_read
            .saturating_add(read.owners.get());
        result
    }

    fn read_view_layout(
        &mut self,
        ty: TypeObjectDigest,
        generation: u16,
        require_access: bool,
    ) -> Result<
        crate::platform::kernel::owned_associated::read_view_admission::ViewLayout,
        Diagnostic,
    > {
        let caller = self.scope.map(|declaration| DeclarationReference {
            package: self.package,
            declaration,
        });
        let occurrence = crate::platform::kernel::owned_associated::OwnedTypeOccurrence::authored(
            caller, generation,
        );
        let mut layout = self.with_admission_read(|read, admission| {
            use crate::platform::kernel::owned_associated::read_view_admission as views;
            let layout = views::layout(read, ty, occurrence, admission)?;
            if require_access {
                views::require_access(
                    read,
                    layout.declaration,
                    caller.ok_or_else(|| {
                        compiler_corrupt(
                            "compiler_read_view_scope",
                            "private read-view access has no exact containing declaration",
                        )
                    })?,
                    admission,
                )?;
                views::validate_nominal_at(read, ty, occurrence, admission)?;
            }
            Ok(layout)
        })?;
        for (digest, object) in std::mem::take(&mut layout.types) {
            self.canonical.code_step()?;
            self.retain_derived_type(Some(digest), &object)?;
        }
        self.tables.declaration(layout.declaration)?;
        for field in &layout.fields {
            self.canonical.code_step()?;
            self.tables.ty(field.ty)?;
            self.tables.read_view_field(ReadViewFieldReference {
                declaration: layout.declaration,
                field: field.id,
            })?;
        }
        Ok(layout)
    }

    fn owned_record_fields(
        &mut self,
        ty: TypeObjectDigest,
        require_access: bool,
    ) -> Result<Vec<(FieldReference, TypeObjectDigest)>, Diagnostic> {
        let scope = self.scope.map(|declaration| DeclarationReference {
            package: self.package,
            declaration,
        });
        let layout = self.with_expression_read(|read| {
            if require_access {
                crate::platform::kernel::owned_record::require_access(read, ty, scope)?;
            }
            crate::platform::kernel::owned_record::layout(read, ty)
        })?;
        for (digest, object) in layout.types {
            self.canonical.code_step()?;
            self.retain_derived_type(Some(digest), &object)?;
        }
        for (field, ty) in &layout.fields {
            self.tables.field(*field)?;
            self.tables.ty(*ty)?;
        }
        Ok(layout.fields)
    }

    fn implementation_operand(
        &mut self,
        operand: &crate::platform::kernel::ImplementationOperand,
    ) -> Result<(), Diagnostic> {
        operand.validate_local()?;
        for operand in operand.walk() {
            let reference = match operand {
                crate::platform::kernel::ImplementationOperand::Concrete {
                    implementation,
                    type_arguments,
                    ..
                } => {
                    for ty in type_arguments {
                        self.tables.ty(*ty)?;
                    }
                    *implementation
                }
                crate::platform::kernel::ImplementationOperand::Parameter { scope, .. } => *scope,
            };
            self.tables.declaration(reference)?;
        }
        Ok(())
    }

    fn owned_method(
        &mut self,
        contract: DeclarationReference,
        method: crate::platform::semantic_id::MethodId,
    ) -> Result<crate::platform::kernel::OwnedMethod, Diagnostic> {
        let c = if contract.package == self.package {
            match self.required_owner(
                OwnerKey::Declaration(contract.declaration),
                "owned method contract",
            )? {
                OwnerRecord::Declaration(d) => match d.payload {
                    DeclarationPayload::OwnedContract(c) => Some(c),
                    _ => None,
                },
                _ => None,
            }
        } else {
            match self.exact_package_interface_owner(
                contract.package,
                OwnerKey::Declaration(contract.declaration),
            )? {
                PackageInterfaceRecord::Declaration(d) => match d.payload {
                    crate::platform::kernel::PackageInterfaceDeclarationPayload::OwnedContract(
                        c,
                    ) => Some(c),
                    _ => None,
                },
                _ => None,
            }
        }
        .ok_or_else(|| {
            compiler_corrupt(
                "compiler_owned_contract",
                "missing exact owned method contract",
            )
        })?;
        c.methods
            .into_iter()
            .find(|m| m.id == method)
            .ok_or_else(|| {
                compiler_corrupt("compiler_owned_contract", "missing exact owned method")
            })
    }

    fn owned_local_type(&mut self, ty: TypeObjectDigest) -> Result<bool, Diagnostic> {
        let object = self.with_expression_read(|read| read.type_object(ty))?;
        Ok(match object.map(|t| t.form) {
            Some(
                TypeForm::RegionCap {
                    mode:
                        crate::platform::kernel::RegionMode::Unique
                        | crate::platform::kernel::RegionMode::Frozen
                        | crate::platform::kernel::RegionMode::Pinned,
                    ..
                }
                | TypeForm::ByteBuffer
                | TypeForm::OwnedI64Cell
                | TypeForm::OwnedProduct { .. }
                | TypeForm::OwnedChoice { .. }
                | TypeForm::OwnedSequence { .. },
            ) => true,
            Some(TypeForm::TypeParameter { parameter }) => matches!(
                self.required_owner(OwnerKey::TypeParameter(parameter), "owned local parameter")?,
                OwnerRecord::TypeParameter(p) if p.constraints.has_owned()
            ),
            Some(TypeForm::Named { .. } | TypeForm::Applied { .. }) => {
                let package = self.package;
                self.with_expression_read(|read| {
                    crate::platform::kernel::owned_record::is_owned(read, package, ty)
                })?
            }
            _ => false,
        })
    }
    fn compile_code(
        &mut self,
        root: ExpressionId,
        parameters: &[ParameterId],
    ) -> Result<CompiledCode, Diagnostic> {
        let mut compiler = CodeCompiler::new(self, parameters)?;
        let relation = compiler.return_relation;
        compiler.expression_with_use(
            root,
            0,
            if matches!(relation, ResultRelation::ReadFrom { .. }) {
                ParameterUse::Borrow
            } else {
                ParameterUse::Unrestricted
            },
        )?;
        compiler.push(match relation {
            ResultRelation::Value => CompiledInstruction::Return,
            ResultRelation::ReadFrom { .. } => CompiledInstruction::ReturnBorrowed,
            ResultRelation::ViewFrom { .. } => CompiledInstruction::ReturnView,
        })?;
        Ok(CompiledCode {
            parameter_count: u32_count("compiled parameters", parameters.len())?,
            local_count: compiler.next_local,
            instructions: compiler.instructions,
        })
    }

    fn required_owner(
        &mut self,
        owner: OwnerKey,
        missing: &'static str,
    ) -> Result<OwnerRecord, Diagnostic> {
        let read = self.canonical.code_owner(owner)?;
        self.work.canonical.add(read.work);
        let record = read.value.ok_or_else(|| {
            compiler_error(
                DiagnosticClass::Corrupt,
                "compiler_unit_owner_missing",
                missing,
            )
        })?;
        if record.owner() != owner {
            return Err(compiler_corrupt(
                "compiler_unit_owner_identity",
                "exact owner read returned another stable identity",
            ));
        }
        self.work.owner_records_read = self.work.owner_records_read.saturating_add(1);
        if matches!(record, OwnerRecord::Expression(_)) {
            self.work.expression_records_read = self.work.expression_records_read.saturating_add(1);
        }
        Ok(record)
    }

    fn required_field(
        &mut self,
        field: FieldId,
        declaration: DeclarationId,
    ) -> Result<crate::platform::kernel::FieldRecord, Diagnostic> {
        match self.required_owner(
            OwnerKey::Field(field),
            "record declaration references a missing field",
        )? {
            OwnerRecord::Field(record) if record.declaration == declaration => Ok(record),
            OwnerRecord::Field(_) => Err(compiler_corrupt(
                "compiler_unit_field_parent",
                "record field belongs to another declaration",
            )),
            _ => Err(compiler_corrupt(
                "compiler_unit_field_kind",
                "record field identity names another owner kind",
            )),
        }
    }

    fn required_case(
        &mut self,
        case: CaseId,
        declaration: DeclarationId,
    ) -> Result<crate::platform::kernel::CaseRecord, Diagnostic> {
        match self.required_owner(
            OwnerKey::Case(case),
            "variant declaration references a missing case",
        )? {
            OwnerRecord::Case(record) if record.declaration == declaration => Ok(record),
            OwnerRecord::Case(_) => Err(compiler_corrupt(
                "compiler_unit_case_parent",
                "variant case belongs to another declaration",
            )),
            _ => Err(compiler_corrupt(
                "compiler_unit_case_kind",
                "variant case identity names another owner kind",
            )),
        }
    }

    fn required_parameter(
        &mut self,
        parameter: ParameterId,
        parent: ParameterParent,
        label: &'static str,
    ) -> Result<crate::platform::kernel::ParameterRecord, Diagnostic> {
        match self.required_owner(
            OwnerKey::Parameter(parameter),
            "signature references a missing parameter",
        )? {
            OwnerRecord::Parameter(record) if record.parent == parent => Ok(record),
            OwnerRecord::Parameter(_) => Err(compiler_corrupt(
                "compiler_unit_parameter_parent",
                format!("{label} belongs to another semantic parent"),
            )),
            _ => Err(compiler_corrupt(
                "compiler_unit_parameter_kind",
                format!("{label} identity names another owner kind"),
            )),
        }
    }

    fn exact_operation(
        &mut self,
        reference: OperationReference,
    ) -> Result<crate::platform::kernel::OperationRecord, Diagnostic> {
        if reference.package == self.package {
            return match self.required_owner(
                OwnerKey::Operation(reference.operation),
                "capability call references a missing local operation",
            )? {
                OwnerRecord::Operation(record) => Ok(record),
                _ => Err(compiler_corrupt(
                    "compiler_capability_operation_kind",
                    "capability operation identity names another owner kind",
                )),
            };
        }
        match self.exact_package_interface_owner(
            reference.package,
            OwnerKey::Operation(reference.operation),
        )? {
            PackageInterfaceRecord::Operation(record) => Ok(record),
            _ => Err(compiler_corrupt(
                "compiler_capability_operation_kind",
                "dependency capability operation identity names another owner kind",
            )),
        }
    }

    fn exact_parameter(
        &mut self,
        package: PackageId,
        parameter: ParameterId,
    ) -> Result<crate::platform::kernel::ParameterRecord, Diagnostic> {
        if package == self.package {
            return match self.required_owner(
                OwnerKey::Parameter(parameter),
                "capability operation references a missing local parameter",
            )? {
                OwnerRecord::Parameter(record) => Ok(record),
                _ => Err(compiler_corrupt(
                    "compiler_capability_parameter_kind",
                    "capability parameter identity names another owner kind",
                )),
            };
        }
        match self.exact_package_interface_owner(package, OwnerKey::Parameter(parameter))? {
            PackageInterfaceRecord::Parameter(record) => Ok(record),
            _ => Err(compiler_corrupt(
                "compiler_capability_parameter_kind",
                "dependency capability parameter identity names another owner kind",
            )),
        }
    }

    fn exact_case(
        &mut self,
        reference: CaseReference,
    ) -> Result<crate::platform::kernel::CaseRecord, Diagnostic> {
        if reference.package == self.package {
            return match self.required_owner(
                OwnerKey::Case(reference.case),
                "variant expression references a missing local case",
            )? {
                OwnerRecord::Case(record) => Ok(record),
                _ => Err(compiler_corrupt(
                    "compiler_variant_case_kind",
                    "variant case identity names another owner kind",
                )),
            };
        }
        match self
            .exact_package_interface_owner(reference.package, OwnerKey::Case(reference.case))?
        {
            PackageInterfaceRecord::Case(record) => Ok(record),
            _ => Err(compiler_corrupt(
                "compiler_variant_case_kind",
                "dependency variant case identity names another owner kind",
            )),
        }
    }

    fn exact_package_interface_owner(
        &mut self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<PackageInterfaceRecord, Diagnostic> {
        let read = self.canonical.code_interface(package, owner)?;
        self.work.canonical.add(read.work);
        self.work.owner_records_read = self.work.owner_records_read.saturating_add(1);
        read.value.ok_or_else(|| {
            compiler_corrupt(
                "compiler_dependency_owner_missing",
                "exact dependency interface owner is missing",
            )
        })
    }

    fn operation_parameter_uses(
        &mut self,
        operation: OperationReference,
    ) -> Result<Vec<ParameterUse>, Diagnostic> {
        let record = self.exact_operation(operation)?;
        record
            .parameters
            .into_iter()
            .map(|parameter| {
                self.exact_parameter(operation.package, parameter)
                    .map(|record| record.use_mode)
            })
            .collect()
    }

    fn function_parameter_uses(
        &mut self,
        function: DeclarationReference,
    ) -> Result<Vec<ParameterUse>, Diagnostic> {
        let parameters = if function.package == self.package {
            match self.required_owner(
                OwnerKey::Declaration(function.declaration),
                "call references a missing local function",
            )? {
                OwnerRecord::Declaration(record) => match record.payload {
                    DeclarationPayload::Function(signature) => signature.parameters,
                    DeclarationPayload::External(signature) => signature.parameters,
                    _ => {
                        return Err(compiler_corrupt(
                            "compiler_call_function_kind",
                            "call declaration has a non-callable payload",
                        ));
                    }
                },
                _ => {
                    return Err(compiler_corrupt(
                        "compiler_call_function_kind",
                        "call declaration identity names another owner kind",
                    ));
                }
            }
        } else {
            match self.exact_package_interface_owner(
                function.package,
                OwnerKey::Declaration(function.declaration),
            )? {
                PackageInterfaceRecord::Declaration(record) => match record.payload {
                    crate::platform::kernel::PackageInterfaceDeclarationPayload::Function(
                        signature,
                    ) => signature.parameters,
                    crate::platform::kernel::PackageInterfaceDeclarationPayload::External(
                        signature,
                    ) => signature.parameters,
                    _ => {
                        return Err(compiler_corrupt(
                            "compiler_call_function_kind",
                            "dependency call declaration has a non-callable payload",
                        ));
                    }
                },
                _ => {
                    return Err(compiler_corrupt(
                        "compiler_call_function_kind",
                        "dependency call declaration identity names another owner kind",
                    ));
                }
            }
        };
        parameters
            .into_iter()
            .map(|parameter| {
                self.exact_parameter(function.package, parameter)
                    .map(|record| record.use_mode)
            })
            .collect()
    }

    fn http_function_parameters(
        &mut self,
        function: DeclarationReference,
    ) -> Result<
        (
            Vec<crate::platform::kernel::ParameterRecord>,
            TypeObjectDigest,
        ),
        Diagnostic,
    > {
        let (parameters, result) = if function.package == self.package {
            match self.required_owner(
                OwnerKey::Declaration(function.declaration),
                "HTTP route references a missing backing function",
            )? {
                OwnerRecord::Declaration(record) => match record.payload {
                    DeclarationPayload::Function(signature)
                        if signature.type_parameters.is_empty()
                            && signature.effect_parameters.is_empty()
                            && signature.read_scope_parameters.is_empty()
                            && matches!(signature.result.relation, ResultRelation::Value) =>
                    {
                        (signature.parameters, signature.result.ty)
                    }
                    _ => {
                        return Err(compiler_corrupt(
                            "compiler_http_route_function",
                            "HTTP route backing declaration must be one nongeneric function",
                        ));
                    }
                },
                _ => {
                    return Err(compiler_corrupt(
                        "compiler_http_route_function",
                        "HTTP route backing function identity names another owner kind",
                    ));
                }
            }
        } else {
            match self.exact_package_interface_owner(
                function.package,
                OwnerKey::Declaration(function.declaration),
            )? {
                PackageInterfaceRecord::Declaration(record) => match record.payload {
                    crate::platform::kernel::PackageInterfaceDeclarationPayload::Function(
                        signature,
                    ) if signature.type_parameters.is_empty()
                        && signature.effect_parameters.is_empty()
                        && signature.read_scope_parameters.is_empty()
                        && matches!(signature.result.relation, ResultRelation::Value) =>
                    {
                        (signature.parameters, signature.result.ty)
                    }
                    _ => {
                        return Err(compiler_corrupt(
                            "compiler_http_route_function",
                            "HTTP route dependency backing declaration must be one nongeneric function",
                        ));
                    }
                },
                _ => {
                    return Err(compiler_corrupt(
                        "compiler_http_route_function",
                        "HTTP route dependency backing function has another owner kind",
                    ));
                }
            }
        };
        let records = parameters
            .into_iter()
            .map(|parameter| self.exact_parameter(function.package, parameter))
            .collect::<Result<Vec<_>, _>>()?;
        if records
            .iter()
            .any(|parameter| parameter.parent != ParameterParent::Function(function.declaration))
        {
            return Err(compiler_corrupt(
                "compiler_http_route_parameter_parent",
                "HTTP route backing function parameter belongs to another declaration",
            ));
        }
        Ok((records, result))
    }

    fn require_value_callable(&mut self, function: DeclarationReference) -> Result<(), Diagnostic> {
        let relation = if function.package == self.package {
            match self.required_owner(
                OwnerKey::Declaration(function.declaration),
                "callable result contract",
            )? {
                OwnerRecord::Declaration(record) => match record.payload {
                    DeclarationPayload::Function(signature) => signature.result.relation,
                    DeclarationPayload::External(_) => ResultRelation::Value,
                    _ => {
                        return Err(compiler_corrupt(
                            "compiler_callable_kind",
                            "callable identity has a different exact declaration kind",
                        ));
                    }
                },
                _ => {
                    return Err(compiler_corrupt(
                        "compiler_callable_kind",
                        "callable identity has a different exact owner kind",
                    ));
                }
            }
        } else {
            match self.exact_package_interface_owner(
                function.package,
                OwnerKey::Declaration(function.declaration),
            )? {
                PackageInterfaceRecord::Declaration(record) => match record.payload {
                    crate::platform::kernel::PackageInterfaceDeclarationPayload::Function(
                        signature,
                    ) => signature.result.relation,
                    crate::platform::kernel::PackageInterfaceDeclarationPayload::External(_) => {
                        ResultRelation::Value
                    }
                    _ => {
                        return Err(compiler_corrupt(
                            "compiler_callable_kind",
                            "imported callable has a different exact declaration kind",
                        ));
                    }
                },
                _ => {
                    return Err(compiler_corrupt(
                        "compiler_callable_kind",
                        "imported callable has a different exact owner kind",
                    ));
                }
            }
        };
        if !matches!(relation, ResultRelation::Value) {
            return Err(compiler_corrupt(
                "compiler_callable_result",
                "first-class and component callables require a Value result contract",
            ));
        }
        Ok(())
    }

    fn case_payload_is_resource(&mut self, reference: CaseReference) -> Result<bool, Diagnostic> {
        let Some(payload) = self.exact_case(reference)?.payload else {
            return Ok(false);
        };
        let read = self.canonical.code_type(payload)?;
        self.work.canonical.add(read.work);
        Ok(read
            .value
            .is_some_and(|object| matches!(object.form, TypeForm::CapabilityResource { .. })))
    }

    /// Reconstruct the pair from canonical child signatures. Strict loading calls this
    /// independently too, so a rehashed table operand cannot select another result type.
    fn parallel_result_type(
        &mut self,
        left: TypeObjectDigest,
        right: TypeObjectDigest,
        owned: bool,
    ) -> Result<u32, Diagnostic> {
        // The exact canonical child admission proves category and transferability;
        // a type-form catch-all must not classify an associated member as ordinary.
        self.canonical.code_step()?;
        if self.derived_type_objects.len() >= MAXIMUM_COMPILER_UNIT_ITEMS {
            return Err(compiler_error(
                DiagnosticClass::Resource,
                "compiler_parallel_result_type_limit",
                "derived parallel result types exceed the compiler-unit bound",
            ));
        }
        let fields = vec![
            StructuralTypeField {
                name: crate::platform::kernel::Name::new("left")?,
                ty: left,
            },
            StructuralTypeField {
                name: crate::platform::kernel::Name::new("right")?,
                ty: right,
            },
        ];
        let object = TypeObject::new(if owned {
            TypeForm::OwnedProduct { fields }
        } else {
            TypeForm::StructuralRecord { fields }
        })?;
        let digest = self.retain_derived_type(None, &object)?;
        self.tables.ty(digest)
    }
}

struct CodeCompiler<'a, 'b, B: ?Sized> {
    unit: &'a mut UnitBuilder<'b, B>,
    instructions: Vec<CompiledInstruction>,
    locals: BTreeMap<LocalValueReference, u32>,
    next_local: u32,
    active: BTreeSet<ExpressionId>,
    compiled: BTreeSet<ExpressionId>,
    live_loans: BTreeSet<BindingId>,
    read_scope_order: Vec<BindingId>,
    function_scope: Option<(DeclarationReference, u16)>,
    return_relation: ResultRelation<ParameterId>,
}

impl<'a, 'b, B: CodeRead + ?Sized> CodeCompiler<'a, 'b, B> {
    fn new(
        unit: &'a mut UnitBuilder<'b, B>,
        parameters: &[ParameterId],
    ) -> Result<Self, Diagnostic> {
        let (function_scope, return_relation) = match unit.scope {
            Some(declaration) => match unit.required_owner(
                OwnerKey::Declaration(declaration),
                "compiled callable owner",
            )? {
                OwnerRecord::Declaration(record) => match record.payload {
                    DeclarationPayload::Function(function) => (
                        Some((
                            DeclarationReference {
                                package: unit.package,
                                declaration,
                            },
                            record.header.contract_version,
                        )),
                        function.result.relation,
                    ),
                    _ => (None, ResultRelation::Value),
                },
                _ => (None, ResultRelation::Value),
            },
            None => (None, ResultRelation::Value),
        };
        let mut compiler = Self {
            unit,
            instructions: Vec::new(),
            locals: BTreeMap::new(),
            next_local: 0,
            active: BTreeSet::new(),
            compiled: BTreeSet::new(),
            live_loans: BTreeSet::new(),
            read_scope_order: Vec::new(),
            function_scope,
            return_relation,
        };
        for parameter in parameters {
            compiler.bind(LocalValueReference::FunctionParameter(*parameter))?;
        }
        Ok(compiler)
    }

    fn expression(&mut self, expression: ExpressionId, depth: usize) -> Result<(), Diagnostic> {
        self.expression_with_use(expression, depth, ParameterUse::Unrestricted)
    }

    fn application_requirements(
        &mut self,
        requirements: &[crate::platform::kernel::RequirementOperand],
        effects: &[crate::platform::kernel::EffectRow],
    ) -> Result<(), Diagnostic> {
        for reference in requirements
            .iter()
            .chain(effects.iter().flat_map(|row| &row.requirements))
            .filter_map(|operand| operand.concrete())
        {
            self.unit.tables.requirement(reference)?;
        }
        Ok(())
    }

    fn application_scopes(&mut self, scopes: &[ReadScopeArgument]) -> Result<(), Diagnostic> {
        self.application_scopes_with_prospective(scopes, None)
    }

    fn application_scopes_with_prospective(
        &mut self,
        scopes: &[ReadScopeArgument],
        prospective: Option<BindingId>,
    ) -> Result<(), Diagnostic> {
        let caller = self.function_scope.map(|(reference, _)| reference);
        let mut prospective_loans = None;
        if let Some(binding) = prospective {
            self.unit
                .derived_type_admission
                .parallel
                .reserve::<BindingId>(self.live_loans.len().saturating_add(1))?;
            let mut loans = self.live_loans.clone();
            if !loans.insert(binding) {
                return Err(compiler_corrupt(
                    "compiler_read_view_scope",
                    "prospective invocation scope is already live",
                ));
            }
            prospective_loans = Some(loans);
        }
        let live_loans = prospective_loans.as_ref().unwrap_or(&self.live_loans);
        self.unit.with_admission_read(|read, admission| {
            let authority =
                crate::platform::kernel::owned_associated::read_view_admission::ScopeAuthority {
                    binder: caller.map(ReadScopeBinder::Declaration),
                    lexical: caller,
                    live_loans,
                };
            for scope in scopes {
                authority.validate(read, *scope, admission)?;
            }
            Ok(())
        })?;
        for scope in scopes {
            self.unit.canonical.code_step()?;
            self.unit.tables.declaration(scope.declaration())?;
        }
        Ok(())
    }

    fn local_slot(
        &mut self,
        value: LocalValueReference,
    ) -> Result<(u32, TypeObjectDigest), Diagnostic> {
        let local = self.locals.get(&value).copied().ok_or_else(|| {
            compiler_corrupt(
                "compiler_read_view_source",
                "explicit source is outside the exact compiled lexical scope",
            )
        })?;
        let ty = match value {
            LocalValueReference::FunctionParameter(parameter) => {
                let scope = self.unit.scope.ok_or_else(|| {
                    compiler_corrupt(
                        "compiler_read_view_source",
                        "source parameter has no containing callable",
                    )
                })?;
                self.unit
                    .required_parameter(
                        parameter,
                        ParameterParent::Function(scope),
                        "explicit read source",
                    )?
                    .ty
            }
            LocalValueReference::LexicalBinding(binding)
            | LocalValueReference::MatchPayload(binding) => {
                match self
                    .unit
                    .required_owner(OwnerKey::Binding(binding), "explicit read source binding")?
                {
                    OwnerRecord::Binding(record) => record.declared_type.ok_or_else(|| {
                        compiler_corrupt(
                            "compiler_read_view_source",
                            "explicit source requires an exact type annotation",
                        )
                    })?,
                    _ => {
                        return Err(compiler_corrupt(
                            "compiler_read_view_source",
                            "explicit source has a different exact owner kind",
                        ));
                    }
                }
            }
            _ => {
                return Err(compiler_corrupt(
                    "compiler_read_view_source",
                    "explicit source has no callable or lexical carrier",
                ));
            }
        };
        self.unit.tables.ty(ty)?;
        Ok((local, ty))
    }

    fn view_scope(
        &mut self,
        ty: TypeObjectDigest,
        generation: u16,
    ) -> Result<ReadScopeArgument, Diagnostic> {
        let caller = self.function_scope.map(|(reference, _)| reference);
        let occurrence = crate::platform::kernel::owned_associated::OwnedTypeOccurrence::authored(
            caller, generation,
        );
        let scope = self.unit.with_admission_read(|read, admission| {
            if !crate::platform::kernel::memory::view_at(read, ty, occurrence, admission)? {
                return Err(compiler_corrupt(
                    "compiler_read_view_type",
                    "operation requires the positive ReadView category",
                ));
            }
            let object = read.type_object(ty)?.ok_or_else(|| {
                compiler_corrupt("compiler_read_view_type", "missing exact read-view type")
            })?;
            object.form.read_scope().ok_or_else(|| {
                compiler_corrupt(
                    "compiler_read_view_type",
                    "view category has no exact symbolic scope",
                )
            })
        })?;
        self.application_scopes(std::slice::from_ref(&scope))?;
        Ok(scope)
    }

    fn prove_reborrow_scope(
        &mut self,
        parent: ReadScopeArgument,
        child: ReadScopeArgument,
    ) -> Result<(), Diagnostic> {
        let caller = self.function_scope.map(|(reference, _)| reference);
        let live_loans = &self.live_loans;
        let order = &self.read_scope_order;
        self.unit.with_admission_read(|read, admission| {
            crate::platform::kernel::owned_associated::read_view_admission::prove_reborrow_scope(
                read,
                parent,
                child,
                crate::platform::kernel::owned_associated::read_view_admission::ScopeAuthority {
                    binder: caller.map(ReadScopeBinder::Declaration),
                    lexical: caller,
                    live_loans,
                },
                order,
                admission,
            )
        })
    }

    fn view_field(
        &mut self,
        value: LocalValueReference,
        reference: ReadViewFieldReference,
        generation: u16,
    ) -> Result<(u32, u32, u32, ReadViewField), Diagnostic> {
        let (source_local, ty) = self.local_slot(value)?;
        self.view_scope(ty, generation)?;
        let layout = self.unit.read_view_layout(ty, generation, true)?;
        if layout.declaration != reference.declaration {
            return Err(compiler_corrupt(
                "compiler_read_view_field",
                "inline field names another exact read-view declaration",
            ));
        }
        for field in layout.fields {
            self.unit.canonical.code_step()?;
            if field.id == reference.field {
                let view_type = self.unit.tables.ty(ty)?;
                let relocation = self.unit.tables.read_view_field(reference)?;
                return Ok((source_local, view_type, relocation, field));
            }
        }
        Err(compiler_corrupt(
            "compiler_read_view_field",
            "inline field is outside the complete exact declaration inventory",
        ))
    }

    fn local_is_memory(&mut self, value: LocalValueReference) -> Result<bool, Diagnostic> {
        let key = match value {
            LocalValueReference::FunctionParameter(p) => Some(OwnerKey::Parameter(p)),
            LocalValueReference::LexicalBinding(b) | LocalValueReference::MatchPayload(b) => {
                Some(OwnerKey::Binding(b))
            }
            _ => None,
        };
        let ty = if let Some(key) = key {
            match self.unit.required_owner(key, "local ownership type")? {
                OwnerRecord::Parameter(p) => Some(p.ty),
                OwnerRecord::Binding(b) => b.declared_type,
                _ => None,
            }
        } else {
            None
        };
        if let Some(ty) = ty {
            self.unit.owned_local_type(ty)
        } else {
            Ok(false)
        }
    }

    fn local_is_view(&mut self, value: LocalValueReference) -> Result<bool, Diagnostic> {
        let key = match value {
            LocalValueReference::FunctionParameter(parameter) => OwnerKey::Parameter(parameter),
            LocalValueReference::LexicalBinding(binding)
            | LocalValueReference::MatchPayload(binding) => OwnerKey::Binding(binding),
            _ => return Ok(false),
        };
        let ty = match self.unit.required_owner(key, "local read-view category")? {
            OwnerRecord::Parameter(parameter) => Some(parameter.ty),
            OwnerRecord::Binding(binding) => {
                if matches!(
                    binding.kind,
                    BindingKind::ReadView | BindingKind::ReadViewBorrow
                ) && binding.declared_type.is_none()
                {
                    return Err(compiler_corrupt(
                        "compiler_read_view_type",
                        "view local omits its exact annotation",
                    ));
                }
                binding.declared_type
            }
            _ => {
                return Err(compiler_corrupt(
                    "compiler_local_type",
                    "local type points to a different exact owner kind",
                ));
            }
        };
        match ty {
            Some(ty) => self.unit.with_expression_read(|read| {
                let object = read.type_object(ty)?.ok_or_else(|| {
                    compiler_corrupt(
                        "compiler_local_type",
                        "local annotation omits its exact type object",
                    )
                })?;
                Ok(object.form.is_read_view())
            }),
            None => Ok(false),
        }
    }

    fn expression_with_use(
        &mut self,
        expression: ExpressionId,
        depth: usize,
        use_mode: ParameterUse,
    ) -> Result<(), Diagnostic> {
        self.expression_with_prospective_scope(expression, depth, use_mode, None)
    }

    fn expression_with_prospective_scope(
        &mut self,
        expression: ExpressionId,
        depth: usize,
        use_mode: ParameterUse,
        prospective: Option<BindingId>,
    ) -> Result<(), Diagnostic> {
        let record = self.begin_expression(expression, depth)?;
        let result = self.operation(
            record.operation,
            record.contract_version,
            depth + 1,
            use_mode,
            prospective,
        );
        self.active.remove(&expression);
        result
    }

    fn begin_expression(
        &mut self,
        expression: ExpressionId,
        depth: usize,
    ) -> Result<ExpressionRecord, Diagnostic> {
        if depth > crate::platform::kernel::contract::MAXIMUM_EXPRESSION_DEPTH {
            return Err(compiler_error(
                DiagnosticClass::Resource,
                "compiler_expression_depth",
                "normalized expression exceeds the compiler recursion bound",
            ));
        }
        self.unit.work.maximum_expression_depth = self
            .unit
            .work
            .maximum_expression_depth
            .max(u32::try_from(depth).unwrap_or(u32::MAX));
        if !self.active.insert(expression) {
            return Err(compiler_corrupt(
                "compiler_expression_cycle",
                "normalized compiler input contains an expression cycle",
            ));
        }
        if !self.compiled.insert(expression) {
            self.active.remove(&expression);
            return Err(compiler_corrupt(
                "compiler_expression_shared",
                "normalized compiler input shares one expression across two semantic positions",
            ));
        }
        let record = self.unit.required_owner(
            OwnerKey::Expression(expression),
            "compiled expression is missing from canonical authority",
        )?;
        let OwnerRecord::Expression(record) = record else {
            self.active.remove(&expression);
            return Err(compiler_corrupt(
                "compiler_expression_kind",
                "expression identity names another canonical owner kind",
            ));
        };
        if record.id != expression {
            self.active.remove(&expression);
            return Err(compiler_corrupt(
                "compiler_expression_identity",
                "expression record identity changed during exact lowering",
            ));
        }
        Ok(record)
    }

    /// Traverse the canonical child Call exactly once, but evaluate only its
    /// arguments in the parent. The task body belongs to the joined child.
    fn parallel_call(
        &mut self,
        expression: ExpressionId,
        depth: usize,
        wrapper_generation: u16,
    ) -> Result<parallel_lower::Child, Diagnostic> {
        let record = self.begin_expression(expression, depth)?;
        let result = (|| {
            let (function, arguments, type_arguments, implementations, read_scope_arguments) =
                match record.operation {
                    ExpressionOperation::Call {
                        function,
                        arguments,
                        type_arguments,
                        effect_arguments,
                        requirement_arguments,
                        read_scope_arguments,
                    } if effect_arguments.is_empty() && requirement_arguments.is_empty() => (
                        function,
                        arguments,
                        type_arguments,
                        Vec::new(),
                        read_scope_arguments,
                    ),
                    ExpressionOperation::ImplementationCall {
                        function,
                        arguments,
                        type_arguments,
                        implementations,
                        effect_arguments,
                        requirement_arguments,
                        read_scope_arguments,
                    } if effect_arguments.is_empty() && requirement_arguments.is_empty() => (
                        function,
                        arguments,
                        type_arguments,
                        implementations,
                        read_scope_arguments,
                    ),
                    _ => {
                        return Err(compiler_corrupt(
                            "compiler_parallel_call",
                            "parallel child must be a closed named call",
                        ));
                    }
                };
            let admitted = self.unit.parallel_call_contract(
                expression,
                wrapper_generation,
                &self.live_loans,
            )?;
            if admitted.function != function || admitted.arguments != arguments {
                return Err(compiler_corrupt(
                    "compiler_parallel_call",
                    "parallel child canonical identity changed during lowering",
                ));
            }
            let types = type_arguments
                .into_iter()
                .map(|ty| self.unit.tables.ty(ty))
                .collect::<Result<Vec<_>, _>>()?;
            for operand in &implementations {
                self.unit.implementation_operand(operand)?;
            }
            if admitted.parameters.len() != arguments.len() {
                return Err(compiler_corrupt(
                    "compiler_parallel_call",
                    "parallel child arity differs from its exact signature",
                ));
            }
            let function = self.unit.tables.declaration(function)?;
            let count = u32_count("parallel child arguments", arguments.len())?;
            for (argument, parameter) in arguments.into_iter().zip(admitted.parameters) {
                self.expression_with_use(argument, depth + 1, parameter.use_mode)?;
            }
            Ok(parallel_lower::Child {
                function,
                arguments: count,
                result: admitted.result,
                result_owned: admitted.result_owned,
                types,
                implementations,
                read_scope_arguments,
            })
        })();
        self.active.remove(&expression);
        result
    }

    fn operation(
        &mut self,
        operation: ExpressionOperation,
        consumer_generation: u16,
        depth: usize,
        use_mode: ParameterUse,
        prospective: Option<BindingId>,
    ) -> Result<(), Diagnostic> {
        match operation {
            ExpressionOperation::Region { operation } => {
                use super::unit::RegionInstruction as I;
                use crate::platform::kernel::RegionExpression as E;
                match operation {
                    E::With {
                        binding,
                        capacity,
                        body,
                    } => {
                        self.expression(capacity, depth)?;
                        let record = self.binding(binding, BindingKind::Region)?;
                        let ty = record.declared_type.ok_or_else(|| {
                            compiler_corrupt(
                                "compiler_region",
                                "region binding has no capability type",
                            )
                        })?;
                        let capability_type = self.unit.tables.ty(ty)?;
                        let reference = LocalValueReference::LexicalBinding(binding);
                        let local = self.bind(reference)?;
                        self.push(CompiledInstruction::Region(I::Begin {
                            local,
                            capability_type,
                        }))?;
                        self.expression_with_use(body, depth, use_mode)?;
                        self.push(CompiledInstruction::Region(I::End { local }))?;
                        self.locals.remove(&reference);
                    }
                    E::Offset { pointer } => {
                        self.expression(pointer, depth)?;
                        self.push(CompiledInstruction::Region(I::Offset))?;
                    }
                    operation @ (E::MmapSave { .. } | E::MmapLoad { .. }) => {
                        for child in operation.expressions().into_iter().flatten() {
                            self.expression(*child, depth)?;
                        }
                        let capability = *operation.capability().ok_or_else(|| {
                            compiler_corrupt("compiler_region", "missing persistence custody")
                        })?;
                        let (local, _) = self.local_slot(capability)?;
                        let (requirement, target) = *operation.store().ok_or_else(|| {
                            compiler_corrupt("compiler_region", "missing persistence authority")
                        })?;
                        let requirement = match requirement {
                            crate::platform::kernel::RequirementOperand::Concrete(requirement) => {
                                CompiledTransactionRequirement::Concrete(
                                    self.unit.tables.requirement(requirement)?,
                                )
                            }
                            crate::platform::kernel::RequirementOperand::Parameter(parameter) => {
                                CompiledTransactionRequirement::Parameter(parameter)
                            }
                        };
                        let store = (requirement, self.unit.tables.operation(target)?);
                        let instruction = match operation {
                            E::MmapSave { .. } => I::MmapSave { local, store },
                            E::MmapLoad { frozen_type, .. } => I::MmapLoad {
                                local,
                                store,
                                frozen_type: self.unit.tables.ty(frozen_type)?,
                            },
                            _ => {
                                return Err(compiler_corrupt(
                                    "compiler_region",
                                    "invalid persistence operation",
                                ));
                            }
                        };
                        self.push(CompiledInstruction::Region(instruction))?;
                    }
                    operation => {
                        for child in operation.expressions().into_iter().flatten() {
                            self.expression(*child, depth)?;
                        }
                        let capability = *operation.capability().ok_or_else(|| {
                            compiler_corrupt("compiler_region", "region operand is missing custody")
                        })?;
                        let (local, _) = self.local_slot(capability)?;
                        let ty = self.unit.tables.ty(*operation.ty().ok_or_else(|| {
                            compiler_corrupt(
                                "compiler_region",
                                "region operand is missing its type",
                            )
                        })?)?;
                        let operation = match operation {
                            E::Alloc { .. } => I::Alloc {
                                local,
                                pointer_type: ty,
                            },
                            E::Read { .. } => I::Read {
                                local,
                                pointer_type: ty,
                            },
                            E::Write { .. } => I::Write {
                                local,
                                pointer_type: ty,
                            },
                            E::Freeze { .. } => I::Freeze {
                                local,
                                frozen_type: ty,
                            },
                            E::Resolve { .. } => I::Resolve {
                                local,
                                pointer_type: ty,
                            },
                            _ => {
                                return Err(compiler_corrupt(
                                    "compiler_region",
                                    "invalid region operation",
                                ));
                            }
                        };
                        self.push(CompiledInstruction::Region(operation))?;
                    }
                }
            }
            ExpressionOperation::CreateReadView {
                declaration,
                type_arguments,
                read_scope,
                source,
                fields,
            } => {
                let (source_local, source_type) = self.local_slot(source)?;
                let caller = self.function_scope.map(|(reference, _)| reference);
                let occurrence =
                    crate::platform::kernel::owned_associated::OwnedTypeOccurrence::authored(
                        caller,
                        consumer_generation,
                    );
                let source_is_view = self.unit.with_admission_read(|read, admission| {
                    let view = crate::platform::kernel::memory::view_at(
                        read,
                        source_type,
                        occurrence,
                        admission,
                    )?;
                    if !view
                        && !crate::platform::kernel::memory::direct_at(
                            read,
                            source_type,
                            occurrence,
                            admission,
                        )?
                    {
                        return Err(compiler_corrupt(
                            "compiler_read_view_source",
                            "constructor source has neither Owned nor positive ReadView category",
                        ));
                    }
                    Ok(view)
                })?;
                self.application_scopes(std::slice::from_ref(&read_scope))?;
                if source_is_view {
                    let parent = self.view_scope(source_type, consumer_generation)?;
                    self.prove_reborrow_scope(parent, read_scope)?;
                }
                let object = TypeObject::new(TypeForm::ReadViewApplied {
                    declaration,
                    arguments: type_arguments,
                    read_scope,
                })?;
                let ty = self.unit.retain_derived_type(None, &object)?;
                let layout = self.unit.read_view_layout(ty, consumer_generation, true)?;
                if layout.fields.len() != fields.len() {
                    return Err(compiler_corrupt(
                        "compiler_read_view_field",
                        "constructor must initialize the complete exact inline field inventory",
                    ));
                }
                self.unit
                    .derived_type_admission
                    .parallel
                    .reserve::<(FieldId, ReadViewField)>(layout.fields.len())?;
                self.unit
                    .derived_type_admission
                    .parallel
                    .reserve::<u32>(fields.len())?;
                self.unit
                    .derived_type_admission
                    .parallel
                    .reserve::<u32>(fields.len())?;
                let mut expected = BTreeMap::new();
                for field in layout.fields {
                    self.unit.canonical.code_step()?;
                    if expected.insert(field.id, field).is_some() {
                        return Err(compiler_corrupt(
                            "compiler_read_view_field",
                            "canonical view layout repeats an inline field identity",
                        ));
                    }
                }
                let mut relocations = Vec::with_capacity(fields.len());
                for initializer in &fields {
                    self.unit.canonical.code_step()?;
                    if !expected.contains_key(&initializer.field) {
                        return Err(compiler_corrupt(
                            "compiler_read_view_field",
                            "constructor names a foreign inline field",
                        ));
                    }
                    relocations.push(self.unit.tables.read_view_field(ReadViewFieldReference {
                        declaration,
                        field: initializer.field,
                    })?);
                }
                let view_type = self.unit.tables.ty(ty)?;
                // The explicit source is protected even for a zero-field view.
                self.push(CompiledInstruction::BeginReadView {
                    view_type,
                    source_local,
                    fields: relocations.clone(),
                })?;
                for initializer in fields {
                    self.unit.canonical.code_step()?;
                    let field = expected.remove(&initializer.field).ok_or_else(|| {
                        compiler_corrupt(
                            "compiler_read_view_field",
                            "constructor repeats an exact inline field",
                        )
                    })?;
                    let use_mode = match field.kind {
                        ReadViewFieldKind::Ordinary => ParameterUse::Unrestricted,
                        ReadViewFieldKind::ReadReference {
                            read_scope: field_scope,
                        } => {
                            if field_scope != read_scope {
                                return Err(compiler_corrupt(
                                    "compiler_read_view_scope",
                                    "reference initializer changes its exact instantiated scope",
                                ));
                            }
                            ParameterUse::Borrow
                        }
                    };
                    self.expression_with_use(initializer.value, depth, use_mode)?;
                }
                if !expected.is_empty() {
                    return Err(compiler_corrupt(
                        "compiler_read_view_field",
                        "constructor omits an exact inline field",
                    ));
                }
                self.push(CompiledInstruction::FinishReadView {
                    view_type,
                    fields: relocations,
                })?;
            }
            ExpressionOperation::BorrowReadView {
                value,
                binding,
                body,
            } => {
                let (source_local, ty) = self.local_slot(value)?;
                self.view_scope(ty, consumer_generation)?;
                let view_type = self.unit.tables.ty(ty)?;
                let (reference, binding_local, binding_type) =
                    self.scoped_read_binding(binding, ty, BindingKind::ReadViewBorrow)?;
                self.push(CompiledInstruction::BorrowReadView {
                    view_type,
                    source_local,
                    binding_local,
                    binding_type,
                })?;
                let loan_scope = self.begin_loan_scope(binding)?;
                self.expression_with_use(body, depth, use_mode)?;
                self.end_loan_scope(loan_scope)?;
                self.push(CompiledInstruction::EndReadViewBorrow { binding_local })?;
                self.unbind(reference);
            }
            ExpressionOperation::ReborrowReadView { value, read_scope } => {
                let (source_local, ty) = self.local_slot(value)?;
                let parent = self.view_scope(ty, consumer_generation)?;
                self.prove_reborrow_scope(parent, read_scope)?;
                let source = self
                    .unit
                    .with_expression_read(|read| read.type_object(ty))?
                    .ok_or_else(|| {
                        compiler_corrupt(
                            "compiler_read_view_type",
                            "reborrow source omits its exact type",
                        )
                    })?;
                let form = match source.form {
                    TypeForm::RegionCap {
                        region,
                        mode: crate::platform::kernel::RegionMode::Shared { .. },
                    } => TypeForm::RegionCap {
                        region,
                        mode: crate::platform::kernel::RegionMode::Shared { scope: read_scope },
                    },
                    TypeForm::ReadViewApplied {
                        declaration,
                        arguments,
                        ..
                    } => TypeForm::ReadViewApplied {
                        declaration,
                        arguments,
                        read_scope,
                    },
                    TypeForm::AssociatedReadViewMember {
                        contract, member, ..
                    } => TypeForm::AssociatedReadViewMember {
                        contract,
                        member,
                        read_scope,
                    },
                    TypeForm::AssociatedReadViewProjection {
                        scope,
                        parameter,
                        contract,
                        member,
                        ..
                    } => TypeForm::AssociatedReadViewProjection {
                        scope,
                        parameter,
                        contract,
                        member,
                        read_scope,
                    },
                    _ => {
                        return Err(compiler_corrupt(
                            "compiler_read_view_type",
                            "reborrow source has no positive ReadView shape",
                        ));
                    }
                };
                let result = self
                    .unit
                    .retain_derived_type(None, &TypeObject::new(form)?)?;
                self.view_scope(result, consumer_generation)?;
                let view_type = self.unit.tables.ty(result)?;
                self.push(CompiledInstruction::ReborrowReadView {
                    view_type,
                    source_local,
                })?;
            }
            ExpressionOperation::MoveReadView { value }
            | ExpressionOperation::DropReadView { value } => {
                let LocalValueReference::LexicalBinding(binding) = value else {
                    return Err(compiler_corrupt(
                        "compiler_read_view_source",
                        "view move and drop require an owning lexical descriptor",
                    ));
                };
                self.binding(binding, BindingKind::ReadView)?;
                let (source_local, ty) = self.local_slot(value)?;
                self.view_scope(ty, consumer_generation)?;
                let view_type = self.unit.tables.ty(ty)?;
                self.push(
                    if matches!(operation, ExpressionOperation::MoveReadView { .. }) {
                        CompiledInstruction::MoveReadView {
                            view_type,
                            source_local,
                        }
                    } else {
                        CompiledInstruction::DropReadView {
                            view_type,
                            source_local,
                        }
                    },
                )?;
            }
            ExpressionOperation::ReadViewDataField { value, field } => {
                let (source_local, view_type, field, selected) =
                    self.view_field(value, field, consumer_generation)?;
                if selected.kind != ReadViewFieldKind::Ordinary {
                    return Err(compiler_corrupt(
                        "compiler_read_view_field",
                        "data projection names a read-reference field",
                    ));
                }
                self.push(CompiledInstruction::ReadViewDataField {
                    view_type,
                    source_local,
                    field,
                })?;
            }
            ExpressionOperation::BorrowReadViewField {
                value,
                field,
                binding,
                body,
            } => {
                let (source_local, view_type, field, selected) =
                    self.view_field(value, field, consumer_generation)?;
                if !matches!(selected.kind, ReadViewFieldKind::ReadReference { .. }) {
                    return Err(compiler_corrupt(
                        "compiler_read_view_field",
                        "reference borrowing names an ordinary data field",
                    ));
                }
                let caller = self.function_scope.map(|(reference, _)| reference);
                let occurrence =
                    crate::platform::kernel::owned_associated::OwnedTypeOccurrence::authored(
                        caller,
                        consumer_generation,
                    );
                let view = self.unit.with_admission_read(|read, admission| {
                    crate::platform::kernel::memory::view_at(
                        read,
                        selected.ty,
                        occurrence,
                        admission,
                    )
                })?;
                let kind = if view {
                    BindingKind::ReadViewBorrow
                } else {
                    BindingKind::OwnedBorrow
                };
                let (reference, binding_local, binding_type) =
                    self.scoped_read_binding(binding, selected.ty, kind)?;
                self.push(CompiledInstruction::BorrowReadViewField {
                    view_type,
                    source_local,
                    field,
                    binding_local,
                    binding_type,
                })?;
                let loan_scope = self.begin_loan_scope(binding)?;
                self.expression_with_use(body, depth, use_mode)?;
                self.end_loan_scope(loan_scope)?;
                self.push(CompiledInstruction::EndReadViewBorrow { binding_local })?;
                self.unbind(reference);
            }
            ExpressionOperation::SequenceEmpty { sequence_type } => {
                self.sequence_item_type(sequence_type)?;
                let sequence_type = self.unit.tables.ty(sequence_type)?;
                self.push(CompiledInstruction::SequenceEmpty { sequence_type })?;
            }
            ExpressionOperation::SequenceLength {
                sequence_type,
                source,
            } => {
                self.sequence_item_type(sequence_type)?;
                let source_local = self.borrow_source_local(source, sequence_type, depth)?;
                let sequence_type = self.unit.tables.ty(sequence_type)?;
                self.push(CompiledInstruction::SequenceLength {
                    sequence_type,
                    source_local,
                })?;
            }
            ExpressionOperation::SequencePush {
                sequence_type,
                value,
                source,
            } => {
                let item = self.sequence_item_type(sequence_type)?;
                // Ordinary expressions evaluate before custody moves. Owned elements remain
                // exact consuming locals and use the existing checked load instruction.
                self.sequence_value(value, item, depth)?;
                let source_local = self.borrow_source_local(source, sequence_type, depth)?;
                let sequence_type = self.unit.tables.ty(sequence_type)?;
                self.push(CompiledInstruction::SequencePush {
                    sequence_type,
                    source_local,
                })?;
            }
            ExpressionOperation::SequenceGet {
                sequence_type,
                source,
                index,
            } => {
                let item = self.sequence_item_type(sequence_type)?;
                if self.unit.owned_local_type(item)? {
                    return Err(compiler_corrupt(
                        "compiler_sequence_get_item",
                        "sequence get requires an ordinary element type",
                    ));
                }
                self.expression(index, depth)?;
                let source_local = self.borrow_source_local(source, sequence_type, depth)?;
                let sequence_type = self.unit.tables.ty(sequence_type)?;
                self.push(CompiledInstruction::SequenceGet {
                    sequence_type,
                    source_local,
                })?;
            }
            ExpressionOperation::SequenceReplace {
                sequence_type,
                result_type,
                index,
                value,
                source,
            } => {
                let item = self.sequence_item_type(sequence_type)?;
                let expected = encode_type_object(&TypeObject::new(TypeForm::OwnedProduct {
                    fields: vec![
                        StructuralTypeField {
                            name: crate::platform::kernel::Name::new("rest")?,
                            ty: sequence_type,
                        },
                        StructuralTypeField {
                            name: crate::platform::kernel::Name::new("value")?,
                            ty: item,
                        },
                    ],
                })?)?
                .0;
                if expected != result_type {
                    return Err(compiler_corrupt(
                        "compiler_sequence_result_type",
                        "sequence replacement result differs from its exact rest/value envelope",
                    ));
                }
                self.expression(index, depth)?;
                self.sequence_value(value, item, depth)?;
                let source_local = self.borrow_source_local(source, sequence_type, depth)?;
                let sequence_type = self.unit.tables.ty(sequence_type)?;
                let result_type = self.unit.tables.ty(result_type)?;
                self.push(CompiledInstruction::SequenceReplace {
                    sequence_type,
                    result_type,
                    source_local,
                })?;
            }
            ExpressionOperation::SequencePop {
                sequence_type,
                result_type,
                source,
            } => {
                let item = self.sequence_item_type(sequence_type)?;
                let result_payload =
                    encode_type_object(&TypeObject::new(TypeForm::OwnedProduct {
                        fields: vec![
                            StructuralTypeField {
                                name: crate::platform::kernel::Name::new("rest")?,
                                ty: sequence_type,
                            },
                            StructuralTypeField {
                                name: crate::platform::kernel::Name::new("value")?,
                                ty: item,
                            },
                        ],
                    })?)?
                    .0;
                let expected = encode_type_object(&TypeObject::new(TypeForm::OwnedChoice {
                    cases: vec![
                        StructuralTypeField {
                            name: crate::platform::kernel::Name::new("empty")?,
                            ty: sequence_type,
                        },
                        StructuralTypeField {
                            name: crate::platform::kernel::Name::new("item")?,
                            ty: result_payload,
                        },
                    ],
                })?)?
                .0;
                if expected != result_type {
                    return Err(compiler_corrupt(
                        "compiler_sequence_result_type",
                        "sequence pop result differs from its exact empty/item custody envelope",
                    ));
                }
                let source_local = self.borrow_source_local(source, sequence_type, depth)?;
                let sequence_type = self.unit.tables.ty(sequence_type)?;
                let result_type = self.unit.tables.ty(result_type)?;
                self.push(CompiledInstruction::SequencePop {
                    sequence_type,
                    result_type,
                    source_local,
                })?;
            }
            ExpressionOperation::BorrowCall {
                call,
                binding,
                body,
            } => {
                let operation = match self
                    .unit
                    .required_owner(OwnerKey::Expression(call), "borrowed invocation")?
                {
                    OwnerRecord::Expression(record) => record.operation,
                    _ => {
                        return Err(compiler_corrupt(
                            "compiler_borrow_call",
                            "borrowed invocation is missing",
                        ));
                    }
                };
                let arguments = match operation {
                    ExpressionOperation::Call { arguments, .. }
                    | ExpressionOperation::ImplementationCall { arguments, .. }
                    | ExpressionOperation::MethodCall { arguments, .. } => arguments,
                    _ => {
                        return Err(compiler_corrupt(
                            "compiler_borrow_call",
                            "borrowed invocation requires an exact named call",
                        ));
                    }
                };
                let scope = self.unit.scope;
                let caller = self.function_scope.map(|(reference, _)| reference);
                let own_scope = self.admitted_lexical_scope(binding)?;
                self.unit
                    .derived_type_admission
                    .parallel
                    .reserve::<BindingId>(
                        self.live_loans
                            .len()
                            .saturating_add(usize::from(own_scope.is_some())),
                    )?;
                let mut invocation_loans = self.live_loans.clone();
                if own_scope.is_some() && !invocation_loans.insert(binding) {
                    return Err(compiler_corrupt(
                        "compiler_read_view_scope",
                        "borrowed invocation repeats an already live exact scope",
                    ));
                }
                let invocation = self.unit.with_admission_read(|read, admission| {
                    crate::platform::kernel::memory::borrow_invocation_at_with_scope_authority(
                        read, call, scope,
                        crate::platform::kernel::owned_associated::read_view_admission::ScopeAuthority {
                            binder: caller.map(ReadScopeBinder::Declaration), lexical: caller, live_loans: &invocation_loans,
                        }, admission,
                    )
                })?;
                if invocation.result.relation.source().copied() != Some(invocation.source) {
                    return Err(compiler_corrupt(
                        "compiler_borrow_call",
                        "borrowed application changed its selected source",
                    ));
                }
                let position = arguments
                    .iter()
                    .position(|argument| *argument == invocation.source)
                    .ok_or_else(|| {
                        compiler_corrupt(
                            "compiler_borrow_call",
                            "selected source is outside the exact argument inventory",
                        )
                    })?;
                let source_position = u32_count("borrowed result source position", position)?;
                for (digest, object) in invocation.types {
                    self.unit.canonical.code_step()?;
                    self.unit.retain_derived_type(Some(digest), &object)?;
                }
                let source = match self.unit.required_owner(
                    OwnerKey::Expression(invocation.source),
                    "borrowed source argument",
                )? {
                    OwnerRecord::Expression(record) => match record.operation {
                        ExpressionOperation::Local { value } => value,
                        _ => {
                            return Err(compiler_corrupt(
                                "compiler_borrow_source",
                                "borrowed result source must be an exact local",
                            ));
                        }
                    },
                    _ => {
                        return Err(compiler_corrupt(
                            "compiler_borrow_source",
                            "borrowed result source is missing",
                        ));
                    }
                };
                let (source_local, source_type) = self.local_slot(source)?;
                if source_type != invocation.source_type {
                    return Err(compiler_corrupt(
                        "compiler_borrow_source_type",
                        "selected local differs from the complete applied source type",
                    ));
                }
                let view_result =
                    matches!(invocation.result.relation, ResultRelation::ViewFrom { .. });
                let kind = if view_result {
                    BindingKind::ReadView
                } else {
                    BindingKind::OwnedBorrow
                };
                let destination = self.binding(binding, kind)?;
                if destination.value.is_some()
                    || destination.declared_type != Some(invocation.result.ty)
                {
                    return Err(compiler_corrupt(
                        "compiler_borrow_binding",
                        "borrowed result requires its exact applied annotation and no initializer",
                    ));
                }
                if let Some(read_scope) = own_scope {
                    self.push(CompiledInstruction::BeginReadViewScope { read_scope })?;
                }
                self.expression_with_prospective_scope(
                    call,
                    depth,
                    ParameterUse::Unrestricted,
                    own_scope.map(|_| binding),
                )?;
                let terminal = self.instructions.pop().ok_or_else(|| {
                    compiler_corrupt("compiler_borrow_call", "missing call instruction")
                })?;
                if !matches!(
                    terminal,
                    CompiledInstruction::Call { .. }
                        | CompiledInstruction::ImplementationCall { .. }
                        | CompiledInstruction::MethodCall { .. }
                ) {
                    return Err(compiler_corrupt(
                        "compiler_borrow_call",
                        "borrowed invocation has no exact call terminal",
                    ));
                }
                let (reference, binding_local, binding_type) =
                    self.scoped_read_binding(binding, invocation.result.ty, kind)?;
                self.push(CompiledInstruction::BeginBorrowCall {
                    source_local,
                    source_position,
                })?;
                self.push(terminal)?;
                self.push(if view_result {
                    CompiledInstruction::AdoptViewResult {
                        source_local,
                        binding_local,
                        binding_type,
                    }
                } else {
                    CompiledInstruction::AdoptBorrowResult {
                        source_local,
                        binding_local,
                        binding_type,
                    }
                })?;
                self.expression_with_use(body, depth, use_mode)?;
                if view_result {
                    self.clear_owning_local(binding_local)?;
                }
                self.end_loan_scope(own_scope)?;
                if !view_result {
                    self.push(CompiledInstruction::EndOwnedBorrow { binding_local })?;
                }
                self.unbind(reference);
            }
            ExpressionOperation::BorrowOwnedItem {
                sequence_type,
                source,
                index,
                binding,
                body,
            } => {
                let item = self.sequence_item_type(sequence_type)?;
                self.expression(index, depth)?;
                let source_local = self.borrow_source_local(source, sequence_type, depth)?;
                let (reference, binding_local, binding_type) =
                    self.borrow_binding(binding, item)?;
                let sequence_type = self.unit.tables.ty(sequence_type)?;
                self.push(CompiledInstruction::BorrowOwnedItem {
                    sequence_type,
                    source_local,
                    binding_local,
                    binding_type,
                })?;
                let loan_scope = self.begin_loan_scope(binding)?;
                self.expression_with_use(body, depth, use_mode)?;
                self.end_loan_scope(loan_scope)?;
                self.push(CompiledInstruction::EndOwnedBorrow { binding_local })?;
                self.unbind(reference);
            }
            ExpressionOperation::BorrowOwnedRecordField {
                record_type,
                source,
                field,
                binding,
                body,
            } => {
                let fields = self.unit.owned_record_fields(record_type, true)?;
                let (slot, (_, ty)) = fields
                    .iter()
                    .enumerate()
                    .find(|(_, (candidate, _))| *candidate == field)
                    .ok_or_else(|| {
                        compiler_corrupt(
                            "compiler_owned_record_field",
                            "borrow names no exact nominal owned field",
                        )
                    })?;
                let source_local = self.borrow_source_local(source, record_type, depth)?;
                let (reference, binding_local, binding_type) = self.borrow_binding(binding, *ty)?;
                let product_type = self.unit.tables.ty(record_type)?;
                self.push(CompiledInstruction::BorrowOwnedField {
                    product_type,
                    source_local,
                    field: u32_count("borrowed owned record field", slot)?,
                    binding_local,
                    binding_type,
                })?;
                let loan_scope = self.begin_loan_scope(binding)?;
                self.expression_with_use(body, depth, use_mode)?;
                self.end_loan_scope(loan_scope)?;
                self.push(CompiledInstruction::EndOwnedBorrow { binding_local })?;
                self.unbind(reference);
            }
            ExpressionOperation::BorrowOwnedField {
                product_type,
                source,
                field,
                binding,
                body,
            } => {
                let source_local = self.borrow_source_local(source, product_type, depth)?;
                let read = self.unit.canonical.code_type(product_type)?;
                self.unit.work.canonical.add(read.work);
                let object = read.value.ok_or_else(|| {
                    compiler_corrupt("compiler_product_type", "missing borrowed product type")
                })?;
                let TypeForm::OwnedProduct { fields } = object.form else {
                    return Err(compiler_corrupt(
                        "compiler_product_type",
                        "field borrowing requires a product type",
                    ));
                };
                let mut selected = None;
                for (index, candidate) in fields.into_iter().enumerate() {
                    self.unit.canonical.code_step()?;
                    if candidate.name == field {
                        selected =
                            Some((u32_count("borrowed product field", index)?, candidate.ty));
                    }
                }
                let (field, ty) = selected.ok_or_else(|| {
                    compiler_corrupt("compiler_product_field", "unknown borrowed product field")
                })?;
                let (reference, binding_local, binding_type) = self.borrow_binding(binding, ty)?;
                let product_type = self.unit.tables.ty(product_type)?;
                self.push(CompiledInstruction::BorrowOwnedField {
                    product_type,
                    source_local,
                    field,
                    binding_local,
                    binding_type,
                })?;
                let loan_scope = self.begin_loan_scope(binding)?;
                self.expression_with_use(body, depth, use_mode)?;
                self.end_loan_scope(loan_scope)?;
                self.push(CompiledInstruction::EndOwnedBorrow { binding_local })?;
                self.unbind(reference);
            }
            ExpressionOperation::MatchBorrowedOwned {
                choice_type,
                source,
                arms,
            } => {
                let source_local = self.borrow_source_local(source, choice_type, depth)?;
                let read = self.unit.canonical.code_type(choice_type)?;
                self.unit.work.canonical.add(read.work);
                let object = read.value.ok_or_else(|| {
                    compiler_corrupt("compiler_choice_type", "missing borrowed choice type")
                })?;
                let TypeForm::OwnedChoice { cases: expected } = object.form else {
                    return Err(compiler_corrupt(
                        "compiler_choice_type",
                        "borrowed matching requires a choice type",
                    ));
                };
                if expected.len() != arms.len() || arms.is_empty() {
                    return Err(compiler_corrupt(
                        "compiler_choice_case",
                        "borrowed matching must cover the complete choice",
                    ));
                }
                let choice_type = self.unit.tables.ty(choice_type)?;
                let switch = self.push(CompiledInstruction::MatchBorrowedOwned {
                    choice_type,
                    source_local,
                    cases: Vec::new(),
                })?;
                let mut cases = Vec::with_capacity(arms.len());
                let mut exits = Vec::with_capacity(arms.len());
                for (arm, expected) in arms.into_iter().zip(expected) {
                    self.unit.canonical.code_step()?;
                    if arm.name != expected.name {
                        return Err(compiler_corrupt(
                            "compiler_choice_case",
                            "borrowed choice arms differ from exact case order",
                        ));
                    }
                    let target = self.next_instruction()?;
                    let (reference, binding_local, binding_type) =
                        self.borrow_binding(arm.binding, expected.ty)?;
                    let loan_scope = self.begin_loan_scope(arm.binding)?;
                    self.expression_with_use(arm.body, depth, use_mode)?;
                    self.end_loan_scope(loan_scope)?;
                    self.push(CompiledInstruction::EndOwnedBorrow { binding_local })?;
                    self.unbind(reference);
                    cases.push(super::unit::CompiledBorrowedOwnedChoiceJump {
                        target,
                        binding_local,
                        binding_type,
                    });
                    exits.push(self.push(CompiledInstruction::Jump(u32::MAX))?);
                }
                let end = self.next_instruction()?;
                for exit in exits {
                    self.instructions[exit as usize] = CompiledInstruction::Jump(end);
                }
                self.instructions[switch as usize] = CompiledInstruction::MatchBorrowedOwned {
                    choice_type,
                    source_local,
                    cases,
                };
            }
            ExpressionOperation::Parallel { left, right } => {
                let left = self.parallel_call(left, depth, consumer_generation)?;
                let right = self.parallel_call(right, depth, consumer_generation)?;
                let result_type = self.unit.parallel_result_type(
                    left.result,
                    right.result,
                    left.result_owned || right.result_owned,
                )?;
                self.push(CompiledInstruction::Parallel {
                    left: left.function,
                    left_arguments: left.arguments,
                    left_types: left.types,
                    left_implementations: left.implementations,
                    left_read_scope_arguments: left.read_scope_arguments,
                    right: right.function,
                    right_arguments: right.arguments,
                    right_types: right.types,
                    right_implementations: right.implementations,
                    right_read_scope_arguments: right.read_scope_arguments,
                    result_type,
                })?;
            }
            ExpressionOperation::ChooseOwned {
                choice_type,
                case,
                value,
            } => {
                let read = self.unit.canonical.code_type(choice_type)?;
                self.unit.work.canonical.add(read.work);
                let object = read.value.ok_or_else(|| {
                    compiler_corrupt("compiler_choice_type", "missing choice type")
                })?;
                let TypeForm::OwnedChoice { cases } = object.form else {
                    return Err(compiler_corrupt(
                        "compiler_choice_type",
                        "owned choice operand is not a choice",
                    ));
                };
                let case = cases
                    .iter()
                    .position(|candidate| candidate.name == case)
                    .ok_or_else(|| {
                        compiler_corrupt("compiler_choice_case", "unknown owned choice case")
                    })?;
                self.expression(value, depth)?;
                let choice_type = self.unit.tables.ty(choice_type)?;
                self.push(CompiledInstruction::ChooseOwned {
                    choice_type,
                    case: u32_count("choice case", case)?,
                })?;
            }
            ExpressionOperation::MatchOwned {
                choice_type,
                source,
                arms,
            } => {
                self.expression_with_use(source, depth, ParameterUse::Consume)?;
                let choice_type = self.unit.tables.ty(choice_type)?;
                let switch = self.push(CompiledInstruction::MatchOwned {
                    choice_type,
                    cases: Vec::new(),
                })?;
                let mut cases = Vec::with_capacity(arms.len());
                let mut exits = Vec::with_capacity(arms.len());
                for arm in arms {
                    let target = self.next_instruction()?;
                    self.binding(arm.binding, BindingKind::OwnedChoicePayload)?;
                    let reference = LocalValueReference::LexicalBinding(arm.binding);
                    let binding_local = self.bind(reference)?;
                    self.expression_with_use(arm.body, depth, use_mode)?;
                    self.push(CompiledInstruction::Unit)?;
                    self.push(CompiledInstruction::StoreLocal(binding_local))?;
                    self.unbind(reference);
                    cases.push(super::unit::CompiledOwnedChoiceJump {
                        target,
                        binding_local,
                    });
                    exits.push(self.push(CompiledInstruction::Jump(u32::MAX))?);
                }
                let end = self.next_instruction()?;
                for exit in exits {
                    self.instructions[exit as usize] = CompiledInstruction::Jump(end);
                }
                self.instructions[switch as usize] =
                    CompiledInstruction::MatchOwned { choice_type, cases };
            }
            ExpressionOperation::PackOwnedRecord {
                record_type,
                fields,
            } => {
                let expected = self.unit.owned_record_fields(record_type, true)?;
                let mut order = Vec::with_capacity(fields.len());
                let mut seen = BTreeSet::new();
                for field in fields {
                    let slot = expected
                        .iter()
                        .position(|(candidate, _)| *candidate == field.field)
                        .ok_or_else(|| {
                            compiler_corrupt(
                                "compiler_owned_record_field",
                                "pack names no exact nominal owned field",
                            )
                        })?;
                    if !seen.insert(slot) {
                        return Err(compiler_corrupt(
                            "compiler_owned_record_fields",
                            "pack repeats one exact nominal owned field",
                        ));
                    }
                    self.expression(field.value, depth)?;
                    order.push(u32_count("owned record field", slot)?);
                }
                if seen.len() != expected.len() {
                    return Err(compiler_corrupt(
                        "compiler_owned_record_fields",
                        "pack must initialize the complete exact nominal owned layout",
                    ));
                }
                let product_type = self.unit.tables.ty(record_type)?;
                self.push(CompiledInstruction::PackOwned {
                    product_type,
                    fields: order,
                })?;
            }
            ExpressionOperation::UnpackOwnedRecord {
                record_type,
                source,
                fields,
                body,
            } => {
                let expected = self.unit.owned_record_fields(record_type, true)?;
                self.expression_with_use(source, depth, ParameterUse::Consume)?;
                let mut locals = vec![None; expected.len()];
                let mut scoped = Vec::with_capacity(fields.len());
                for field in fields {
                    let slot = expected
                        .iter()
                        .position(|(candidate, _)| *candidate == field.field)
                        .ok_or_else(|| {
                            compiler_corrupt(
                                "compiler_owned_record_field",
                                "unpack names no exact nominal owned field",
                            )
                        })?;
                    self.binding(field.binding, BindingKind::OwnedUnpack)?;
                    let reference = LocalValueReference::LexicalBinding(field.binding);
                    if locals[slot].replace(self.bind(reference)?).is_some() {
                        return Err(compiler_corrupt(
                            "compiler_owned_record_fields",
                            "unpack repeats one exact nominal owned field",
                        ));
                    }
                    scoped.push(reference);
                }
                let locals = locals
                    .into_iter()
                    .collect::<Option<Vec<_>>>()
                    .ok_or_else(|| {
                        compiler_corrupt(
                            "compiler_owned_record_fields",
                            "unpack must bind the complete exact nominal owned layout",
                        )
                    })?;
                let product_type = self.unit.tables.ty(record_type)?;
                self.push(CompiledInstruction::UnpackOwned {
                    product_type,
                    locals: locals.clone(),
                })?;
                self.expression_with_use(body, depth, use_mode)?;
                for local in locals {
                    self.push(CompiledInstruction::Unit)?;
                    self.push(CompiledInstruction::StoreLocal(local))?;
                }
                self.unbind_all(&scoped);
            }
            ExpressionOperation::PackOwned {
                product_type,
                fields,
            } => {
                let read = self.unit.canonical.code_type(product_type)?;
                self.unit.work.canonical.add(read.work);
                let object = read.value.ok_or_else(|| {
                    compiler_corrupt("compiler_product_type", "missing product type")
                })?;
                let TypeForm::OwnedProduct { fields: expected } = object.form else {
                    return Err(compiler_corrupt(
                        "compiler_product_type",
                        "pack requires product type",
                    ));
                };
                let mut order = Vec::new();
                for field in fields {
                    let position = expected
                        .iter()
                        .position(|f| f.name == field.name)
                        .ok_or_else(|| {
                            compiler_corrupt("compiler_product_field", "unknown product field")
                        })?;
                    self.expression(field.value, depth)?;
                    order.push(position as u32);
                }
                let product_type = self.unit.tables.ty(product_type)?;
                self.push(CompiledInstruction::PackOwned {
                    product_type,
                    fields: order,
                })?;
            }
            ExpressionOperation::UnpackOwned {
                product_type,
                source,
                fields,
                body,
            } => {
                self.expression_with_use(source, depth, ParameterUse::Consume)?;
                let mut locals = Vec::new();
                let mut scoped = Vec::new();
                for field in fields {
                    self.binding(field.binding, BindingKind::OwnedUnpack)?;
                    let reference = LocalValueReference::LexicalBinding(field.binding);
                    locals.push(self.bind(reference)?);
                    scoped.push(reference);
                }
                let product_type = self.unit.tables.ty(product_type)?;
                self.push(CompiledInstruction::UnpackOwned {
                    product_type,
                    locals: locals.clone(),
                })?;
                self.expression_with_use(body, depth, use_mode)?;
                for local in locals {
                    self.push(CompiledInstruction::Unit)?;
                    self.push(CompiledInstruction::StoreLocal(local))?;
                }
                self.unbind_all(&scoped);
            }
            ExpressionOperation::Unit {} => {
                self.push(CompiledInstruction::Unit)?;
            }
            ExpressionOperation::Bool { value } => {
                self.push(CompiledInstruction::Bool(value))?;
            }
            ExpressionOperation::I64 { value } => {
                self.push(CompiledInstruction::I64(value))?;
            }
            ExpressionOperation::F64 { value } => {
                self.push(CompiledInstruction::F64(value))?;
            }
            ExpressionOperation::Text { value } => {
                let text = self.unit.tables.text(compiled_text(value))?;
                self.push(CompiledInstruction::Text(text))?;
            }
            ExpressionOperation::StaticText { value } => {
                let text = self.unit.tables.text(compiled_text(value))?;
                self.push(CompiledInstruction::StaticText(text))?;
            }
            ExpressionOperation::Local { value } => {
                let local = self.locals.get(&value).copied().ok_or_else(|| {
                    compiler_corrupt(
                        "compiler_local_missing",
                        "validated exact local reference is outside the compiled lexical scope",
                    )
                })?;
                if self.local_is_view(value)? && use_mode != ParameterUse::Borrow {
                    return Err(compiler_corrupt(
                        "compiler_read_view_move",
                        "ordinary local evaluation cannot copy or implicitly move a view descriptor",
                    ));
                }
                let memory = self.local_is_memory(value)?;
                let use_mode = if memory && use_mode == ParameterUse::Unrestricted {
                    ParameterUse::Consume
                } else {
                    use_mode
                };
                self.push(CompiledInstruction::LoadLocal { local, use_mode })?;
            }
            ExpressionOperation::Constant { declaration } => {
                let function = self.unit.tables.declaration(declaration)?;
                self.push(CompiledInstruction::Call {
                    requirement_arguments: Vec::new(),
                    effect_arguments: Vec::new(),
                    function,
                    type_arguments: Vec::new(),
                    read_scope_arguments: Vec::new(),
                    arguments: 0,
                })?;
            }
            ExpressionOperation::If {
                condition,
                when_true,
                when_false,
            } => {
                self.expression(condition, depth)?;
                let conditional = self.push(CompiledInstruction::JumpIfFalse(u32::MAX))?;
                self.expression_with_use(when_true, depth, use_mode)?;
                let jump = self.push(CompiledInstruction::Jump(u32::MAX))?;
                let false_target = self.next_instruction()?;
                self.expression_with_use(when_false, depth, use_mode)?;
                let end = self.next_instruction()?;
                self.instructions[conditional as usize] =
                    CompiledInstruction::JumpIfFalse(false_target);
                self.instructions[jump as usize] = CompiledInstruction::Jump(end);
            }
            ExpressionOperation::Let { bindings, body } => {
                self.unit
                    .derived_type_admission
                    .parallel
                    .reserve::<(BindingId, ReadScopeArgument)>(bindings.len())?;
                let mut scoped = Vec::with_capacity(bindings.len());
                let mut view_scopes = Vec::new();
                for binding in bindings {
                    let record = match self
                        .unit
                        .required_owner(OwnerKey::Binding(binding), "let binding")?
                    {
                        OwnerRecord::Binding(record)
                            if matches!(record.kind, BindingKind::Let | BindingKind::ReadView) =>
                        {
                            record
                        }
                        _ => {
                            return Err(compiler_corrupt(
                                "compiler_binding_kind",
                                "let requires an ordinary or owning read-view binding",
                            ));
                        }
                    };
                    let view_scope_entry = if record.kind == BindingKind::ReadView {
                        let read_scope =
                            self.admitted_lexical_scope(binding)?.ok_or_else(|| {
                                compiler_corrupt(
                                    "compiler_read_view_scope",
                                    "owning view binding requires one exact current named function",
                                )
                            })?;
                        let ty = record.declared_type.ok_or_else(|| {
                            compiler_corrupt(
                                "compiler_read_view_type",
                                "owning view binding requires its exact annotation",
                            )
                        })?;
                        let declared_scope =
                            self.unit.with_expression_read(|read| {
                                let object = read.type_object(ty)?.ok_or_else(|| {
                                    compiler_corrupt(
                                        "compiler_read_view_type",
                                        "missing owning view annotation",
                                    )
                                })?;
                                object.form.read_scope().ok_or_else(|| compiler_corrupt(
                                    "compiler_read_view_type",
                                    "owning binding annotation has no positive ReadView category",
                                ))
                            })?;
                        let prospective = declared_scope == read_scope;
                        if prospective {
                            self.enter_read_scope(binding)?;
                            self.push(CompiledInstruction::BeginReadViewScope { read_scope })?;
                        }
                        self.view_scope(ty, consumer_generation)?;
                        Some((read_scope, prospective))
                    } else {
                        None
                    };
                    if let Some(ty) = record.declared_type {
                        self.unit.tables.ty(ty)?;
                    }
                    let value = record.value.ok_or_else(|| {
                        compiler_corrupt(
                            "compiler_let_value",
                            "let binding has no canonical value expression",
                        )
                    })?;
                    self.expression(value, depth)?;
                    let reference = LocalValueReference::LexicalBinding(binding);
                    let local = self.bind(reference)?;
                    scoped.push(reference);
                    self.push(CompiledInstruction::StoreLocal(local))?;
                    if let Some((read_scope, prospective)) = view_scope_entry {
                        if !prospective {
                            self.enter_read_scope(binding)?;
                            self.push(CompiledInstruction::BeginReadViewScope { read_scope })?;
                        }
                        view_scopes.push((binding, read_scope));
                    }
                }
                self.expression_with_use(body, depth, use_mode)?;
                // Later view bindings retain earlier owning sources. Tear down
                // mixed Lets in reverse lexical order; ordinary-only Lets keep
                // their existing canonical instruction order.
                let reverse_cleanup = !view_scopes.is_empty();
                for offset in 0..scoped.len() {
                    let index = if reverse_cleanup {
                        scoped.len() - 1 - offset
                    } else {
                        offset
                    };
                    let reference = &scoped[index];
                    let LocalValueReference::LexicalBinding(binding) = reference else {
                        continue;
                    };
                    let record = match self
                        .unit
                        .required_owner(OwnerKey::Binding(*binding), "let cleanup binding")?
                    {
                        OwnerRecord::Binding(record) => record,
                        _ => {
                            return Err(compiler_corrupt(
                                "compiler_binding_kind",
                                "let cleanup lost its exact binding",
                            ));
                        }
                    };
                    let local = self.locals[reference];
                    if record.kind == BindingKind::ReadView {
                        let (_, read_scope) = view_scopes
                            .pop()
                            .filter(|(scope_binding, _)| scope_binding == binding)
                            .ok_or_else(|| {
                                compiler_corrupt(
                                    "compiler_read_view_scope",
                                    "let cleanup lost its exact view scope",
                                )
                            })?;
                        self.clear_owning_local(local)?;
                        self.push(CompiledInstruction::EndReadViewScope { read_scope })?;
                    } else if let Some(ty) = record.declared_type
                        && self.unit.owned_local_type(ty)?
                    {
                        self.push(CompiledInstruction::Unit)?;
                        self.push(CompiledInstruction::StoreLocal(local))?;
                    }
                }
                self.unbind_all(&scoped);
            }
            ExpressionOperation::Sequence { items } => {
                let count = items.len();
                for (index, item) in items.into_iter().enumerate() {
                    self.expression_with_use(
                        item,
                        depth,
                        if index + 1 == count {
                            use_mode
                        } else {
                            ParameterUse::Unrestricted
                        },
                    )?;
                    if index + 1 != count {
                        self.push(CompiledInstruction::Drop)?;
                    }
                }
            }
            ExpressionOperation::ImplementationCall {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
                implementations,
                arguments,
                read_scope_arguments,
            } => {
                self.application_scopes_with_prospective(&read_scope_arguments, prospective)?;
                self.application_requirements(&requirement_arguments, &effect_arguments)?;
                let uses = self.unit.function_parameter_uses(function)?;
                if uses.len() != arguments.len() {
                    return Err(compiler_corrupt(
                        "compiler_call_argument_count",
                        "implementation call arity",
                    ));
                }
                for operand in &implementations {
                    self.unit.implementation_operand(operand)?;
                }
                let function = self.unit.tables.declaration(function)?;
                let type_arguments = type_arguments
                    .into_iter()
                    .map(|ty| self.unit.tables.ty(ty))
                    .collect::<Result<Vec<_>, _>>()?;
                let count = u32_count("implementation call arguments", arguments.len())?;
                for (argument, use_mode) in arguments.into_iter().zip(uses) {
                    self.expression_with_use(argument, depth, use_mode)?;
                }
                self.push(CompiledInstruction::ImplementationCall {
                    requirement_arguments,
                    effect_arguments,
                    function,
                    type_arguments,
                    implementations,
                    arguments: count,
                    read_scope_arguments,
                })?;
            }
            ExpressionOperation::MethodCall {
                witness,
                contract,
                method,
                arguments,
                read_scope_arguments,
            } => {
                self.application_scopes_with_prospective(&read_scope_arguments, prospective)?;
                self.unit.implementation_operand(&witness)?;
                self.unit.tables.declaration(contract)?;
                let signature = self.unit.owned_method(contract, method)?;
                if signature.parameters.len() != arguments.len() {
                    return Err(compiler_corrupt(
                        "compiler_call_argument_count",
                        "owned method arity",
                    ));
                }
                let count = u32_count("method call arguments", arguments.len())?;
                for (argument, parameter) in arguments.into_iter().zip(signature.parameters) {
                    self.expression_with_use(argument, depth, parameter.use_mode)?;
                }
                self.push(CompiledInstruction::MethodCall {
                    witness,
                    contract,
                    method,
                    arguments: count,
                    read_scope_arguments,
                })?;
            }
            ExpressionOperation::Call {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
                arguments,
                read_scope_arguments,
            } => {
                self.application_scopes_with_prospective(&read_scope_arguments, prospective)?;
                self.application_requirements(&requirement_arguments, &effect_arguments)?;
                let parameter_uses = self.unit.function_parameter_uses(function)?;
                if parameter_uses.len() != arguments.len() {
                    return Err(compiler_corrupt(
                        "compiler_call_argument_count",
                        "call arguments disagree with the exact function signature",
                    ));
                }
                let function = self.unit.tables.declaration(function)?;
                let type_arguments = type_arguments
                    .into_iter()
                    .map(|ty| self.unit.tables.ty(ty))
                    .collect::<Result<Vec<_>, _>>()?;
                let argument_count = u32_count("call arguments", arguments.len())?;
                for (argument, use_mode) in arguments.into_iter().zip(parameter_uses) {
                    self.expression_with_use(argument, depth, use_mode)?;
                }
                self.push(CompiledInstruction::Call {
                    requirement_arguments,
                    effect_arguments,
                    function,
                    type_arguments,
                    arguments: argument_count,
                    read_scope_arguments,
                })?;
            }
            ExpressionOperation::FunctionValue {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
                read_scope_arguments,
            } => {
                self.unit.require_value_callable(function)?;
                self.application_scopes_with_prospective(&read_scope_arguments, prospective)?;
                self.application_requirements(&requirement_arguments, &effect_arguments)?;
                let function = self.unit.tables.declaration(function)?;
                let type_arguments = type_arguments
                    .into_iter()
                    .map(|ty| self.unit.tables.ty(ty))
                    .collect::<Result<Vec<_>, _>>()?;
                self.push(CompiledInstruction::FunctionValue {
                    requirement_arguments,
                    effect_arguments,
                    function,
                    type_arguments,
                    read_scope_arguments,
                })?;
            }
            ExpressionOperation::Invoke { callee, arguments } => {
                let argument_count = u32_count("invoke arguments", arguments.len())?;
                self.expression(callee, depth)?;
                for argument in arguments {
                    self.expression(argument, depth)?;
                }
                self.push(CompiledInstruction::Invoke {
                    arguments: argument_count,
                })?;
            }
            ExpressionOperation::Bind { callee, arguments } => {
                let argument_count = u32_count("bind arguments", arguments.len())?;
                self.expression(callee, depth)?;
                self.push(CompiledInstruction::BeginBind {
                    arguments: argument_count,
                })?;
                for (index, argument) in arguments.into_iter().enumerate() {
                    self.expression(argument, depth)?;
                    self.push(CompiledInstruction::Capture {
                        index: u32_count("capture index", index)?,
                    })?;
                }
                self.push(CompiledInstruction::Bind {
                    arguments: argument_count,
                })?;
            }
            ExpressionOperation::Record {
                nominal_type,
                type_arguments,
                fields,
            } => {
                let type_arguments = type_arguments
                    .into_iter()
                    .map(|ty| self.unit.tables.ty(ty))
                    .collect::<Result<_, _>>()?;
                let nominal_type = nominal_type
                    .map(|declaration| self.unit.tables.declaration(declaration))
                    .transpose()?;
                let mut selectors = Vec::with_capacity(fields.len());
                for field in &fields {
                    selectors.push(self.field_selector(field.selector.clone())?);
                }
                for field in fields {
                    self.expression(field.value, depth)?;
                }
                self.push(CompiledInstruction::Record {
                    nominal_type,
                    type_arguments,
                    fields: selectors,
                })?;
            }
            ExpressionOperation::Variant {
                case,
                type_arguments,
                payload,
            } => {
                let type_arguments = type_arguments
                    .into_iter()
                    .map(|ty| self.unit.tables.ty(ty))
                    .collect::<Result<_, _>>()?;
                let resource_payload = self.unit.case_payload_is_resource(case)?;
                let case = self.unit.tables.case(case)?;
                if let Some(payload) = payload {
                    self.expression_with_use(
                        payload,
                        depth,
                        if resource_payload {
                            ParameterUse::Consume
                        } else {
                            ParameterUse::Unrestricted
                        },
                    )?;
                }
                self.push(CompiledInstruction::Variant {
                    type_arguments,
                    case,
                    has_payload: payload.is_some(),
                })?;
            }
            ExpressionOperation::Field { value, selector } => {
                let use_mode = match self
                    .unit
                    .required_owner(OwnerKey::Expression(value), "field source expression")?
                {
                    OwnerRecord::Expression(ExpressionRecord {
                        operation: ExpressionOperation::Local { value: local },
                        ..
                    }) if self.local_is_memory(local)? => ParameterUse::Borrow,
                    _ => ParameterUse::Unrestricted,
                };
                let selector = self.field_selector(selector)?;
                self.expression_with_use(value, depth, use_mode)?;
                self.push(CompiledInstruction::Field(selector))?;
            }
            ExpressionOperation::List { item_type, items } => {
                let item_type = self.unit.tables.ty(item_type)?;
                let item_count = u32_count("list items", items.len())?;
                for item in items {
                    self.expression(item, depth)?;
                }
                self.push(CompiledInstruction::List {
                    item_type,
                    items: item_count,
                })?;
            }
            ExpressionOperation::Map {
                key_type,
                value_type,
                entries,
            } => {
                let key_type = self.unit.tables.ty(key_type)?;
                let value_type = self.unit.tables.ty(value_type)?;
                let entry_count = u32_count("map entries", entries.len())?;
                for entry in entries {
                    self.expression(entry.key, depth)?;
                    self.expression(entry.value, depth)?;
                }
                self.push(CompiledInstruction::Map {
                    key_type,
                    value_type,
                    entries: entry_count,
                })?;
            }
            ExpressionOperation::Match { value, arms } => {
                let resource_match = arms.iter().try_fold(false, |observed, arm| {
                    self.unit
                        .case_payload_is_resource(arm.case)
                        .map(|resource| observed || resource)
                })?;
                self.expression_with_use(
                    value,
                    depth,
                    if resource_match {
                        ParameterUse::Consume
                    } else {
                        ParameterUse::Unrestricted
                    },
                )?;
                let switch = self.push(CompiledInstruction::SwitchVariant(Vec::new()))?;
                let mut compiled_arms = Vec::with_capacity(arms.len());
                let mut exits = Vec::with_capacity(arms.len());
                for arm in arms {
                    let case = self.unit.tables.case(arm.case)?;
                    let target = self.next_instruction()?;
                    let (binding_local, scoped) = if let Some(binding) = arm.payload_binding {
                        self.binding(binding, BindingKind::MatchPayload)?;
                        let reference = LocalValueReference::MatchPayload(binding);
                        (Some(self.bind(reference)?), Some(reference))
                    } else {
                        (None, None)
                    };
                    self.expression_with_use(arm.body, depth, use_mode)?;
                    if let Some(scoped) = scoped {
                        self.locals.remove(&scoped);
                    }
                    compiled_arms.push(CompiledVariantJump {
                        case,
                        target,
                        binding_local,
                    });
                    exits.push(self.push(CompiledInstruction::Jump(u32::MAX))?);
                }
                let end = self.next_instruction()?;
                for exit in exits {
                    self.instructions[exit as usize] = CompiledInstruction::Jump(end);
                }
                self.instructions[switch as usize] =
                    CompiledInstruction::SwitchVariant(compiled_arms);
            }
            ExpressionOperation::CapabilityCall {
                requirement,
                operation,
                arguments,
            } => {
                let parameter_uses = self.unit.operation_parameter_uses(operation)?;
                if parameter_uses.len() != arguments.len() {
                    return Err(compiler_corrupt(
                        "compiler_capability_argument_count",
                        "capability arguments disagree with the exact operation parameters",
                    ));
                }
                let operation = self.unit.tables.operation(operation)?;
                let argument_count = u32_count("capability arguments", arguments.len())?;
                for (argument, use_mode) in arguments.into_iter().zip(parameter_uses) {
                    self.expression_with_use(argument, depth, use_mode)?;
                }
                let instruction = match requirement {
                    crate::platform::kernel::RequirementOperand::Concrete(reference) => {
                        CompiledInstruction::Perform {
                            requirement: self.unit.tables.requirement(reference)?,
                            operation,
                            arguments: argument_count,
                        }
                    }
                    crate::platform::kernel::RequirementOperand::Parameter(parameter) => {
                        CompiledInstruction::PerformParameter {
                            parameter,
                            operation,
                            arguments: argument_count,
                        }
                    }
                };
                self.push(instruction)?;
            }
            ExpressionOperation::Transaction {
                requirement,
                binding,
                body,
            } => {
                self.binding(binding, BindingKind::Transaction)?;
                let reference = LocalValueReference::TransactionBinding(binding);
                let local = self.bind(reference)?;
                let instruction = match requirement {
                    crate::platform::kernel::RequirementOperand::Concrete(reference) => {
                        CompiledInstruction::BeginTransaction {
                            requirement: self.unit.tables.requirement(reference)?,
                            binding: local,
                        }
                    }
                    crate::platform::kernel::RequirementOperand::Parameter(parameter) => {
                        CompiledInstruction::BeginParameterTransaction {
                            parameter,
                            binding: local,
                        }
                    }
                };
                self.push(instruction)?;
                self.expression_with_use(body, depth, use_mode)?;
                let instruction = match requirement {
                    crate::platform::kernel::RequirementOperand::Concrete(reference) => {
                        CompiledInstruction::CommitTransaction {
                            requirement: self.unit.tables.requirement(reference)?,
                            binding: local,
                        }
                    }
                    crate::platform::kernel::RequirementOperand::Parameter(parameter) => {
                        CompiledInstruction::CommitParameterTransaction {
                            parameter,
                            binding: local,
                        }
                    }
                };
                self.push(instruction)?;
                self.unbind(reference);
            }
            ExpressionOperation::TransactionOutcome {
                requirement,
                binding,
                body,
                outcome,
                type_argument,
            } => {
                self.binding(binding, BindingKind::Transaction)?;
                let reference = LocalValueReference::TransactionBinding(binding);
                let local = self.bind(reference)?;
                let requirement = match requirement {
                    crate::platform::kernel::RequirementOperand::Concrete(reference) => {
                        CompiledTransactionRequirement::Concrete(
                            self.unit.tables.requirement(reference)?,
                        )
                    }
                    crate::platform::kernel::RequirementOperand::Parameter(parameter) => {
                        CompiledTransactionRequirement::Parameter(parameter)
                    }
                };
                let outcome = CompiledTransactionOutcome {
                    outcome: self.unit.tables.declaration(outcome.outcome)?,
                    abort_reason: self.unit.tables.declaration(outcome.abort_reason)?,
                    committed: self.unit.tables.case(outcome.committed)?,
                    aborted: self.unit.tables.case(outcome.aborted)?,
                    condition_failed: self.unit.tables.case(outcome.condition_failed)?,
                    conflict: self.unit.tables.case(outcome.conflict)?,
                    type_argument: self.unit.tables.ty(type_argument)?,
                };
                self.push(CompiledInstruction::BeginTransactionOutcome {
                    requirement,
                    binding: local,
                    outcome,
                })?;
                self.expression_with_use(body, depth, use_mode)?;
                self.push(CompiledInstruction::CommitTransactionOutcome {
                    requirement,
                    binding: local,
                    outcome,
                })?;
                self.unbind(reference);
            }
        }
        Ok(())
    }

    fn field_selector(
        &mut self,
        selector: FieldSelector,
    ) -> Result<CompiledFieldSelector, Diagnostic> {
        match selector {
            FieldSelector::Nominal(field) => self
                .unit
                .tables
                .field(field)
                .map(CompiledFieldSelector::Nominal),
            FieldSelector::Structural(name) => self
                .unit
                .tables
                .structural_name(name)
                .map(CompiledFieldSelector::Structural),
        }
    }

    fn sequence_item_type(
        &mut self,
        sequence_type: TypeObjectDigest,
    ) -> Result<TypeObjectDigest, Diagnostic> {
        let read = self.unit.canonical.code_type(sequence_type)?;
        self.unit.work.canonical.add(read.work);
        let Some(TypeObject {
            form: TypeForm::OwnedSequence { item },
            ..
        }) = read.value
        else {
            return Err(compiler_corrupt(
                "compiler_sequence_type",
                "sequence operation requires its exact sequence type",
            ));
        };
        self.unit.tables.ty(item)?;
        Ok(item)
    }

    fn sequence_value(
        &mut self,
        expression: ExpressionId,
        item: TypeObjectDigest,
        depth: usize,
    ) -> Result<(), Diagnostic> {
        if self.unit.owned_local_type(item)? {
            let local = self.borrow_source_local(expression, item, depth)?;
            self.push(CompiledInstruction::LoadLocal {
                local,
                use_mode: ParameterUse::Consume,
            })
            .map(|_| ())
        } else {
            self.expression(expression, depth)
        }
    }

    /// Traverse the exact canonical source child while preserving custody in its local slot.
    fn borrow_source_local(
        &mut self,
        expression: ExpressionId,
        expected: TypeObjectDigest,
        depth: usize,
    ) -> Result<u32, Diagnostic> {
        let record = self.begin_expression(expression, depth)?;
        let result = (|| {
            let ExpressionOperation::Local { value } = record.operation else {
                return Err(compiler_corrupt(
                    "compiler_borrow_source",
                    "lexical read source must be an exact live local",
                ));
            };
            let local = self.locals.get(&value).copied().ok_or_else(|| {
                compiler_corrupt(
                    "compiler_borrow_source",
                    "lexical read source is outside the compiled scope",
                )
            })?;
            let key = match value {
                LocalValueReference::FunctionParameter(parameter) => OwnerKey::Parameter(parameter),
                LocalValueReference::LexicalBinding(binding) => OwnerKey::Binding(binding),
                _ => {
                    return Err(compiler_corrupt(
                        "compiler_borrow_source",
                        "lexical read source has no owning or borrowed local custody",
                    ));
                }
            };
            let actual = match self
                .unit
                .required_owner(key, "borrowed source local type")?
            {
                OwnerRecord::Parameter(record) => Some(record.ty),
                OwnerRecord::Binding(record) => record.declared_type,
                _ => None,
            };
            if actual != Some(expected) {
                return Err(compiler_corrupt(
                    "compiler_borrow_source_type",
                    "lexical read source differs from its exact carrier type",
                ));
            }
            Ok(local)
        })();
        self.active.remove(&expression);
        result
    }

    fn borrow_binding(
        &mut self,
        binding: BindingId,
        expected: TypeObjectDigest,
    ) -> Result<(LocalValueReference, u32, u32), Diagnostic> {
        self.scoped_read_binding(binding, expected, BindingKind::OwnedBorrow)
    }

    fn scoped_read_binding(
        &mut self,
        binding: BindingId,
        expected: TypeObjectDigest,
        kind: BindingKind,
    ) -> Result<(LocalValueReference, u32, u32), Diagnostic> {
        let record = self.binding(binding, kind)?;
        if record.value.is_some() || record.declared_type != Some(expected) {
            return Err(compiler_corrupt(
                "compiler_borrow_binding",
                "scoped read binding requires the exact child annotation and no initializer",
            ));
        }
        let reference = LocalValueReference::LexicalBinding(binding);
        let local = self.bind(reference)?;
        self.enter_read_scope(binding)?;
        let ty = self.unit.tables.ty(expected)?;
        Ok((reference, local, ty))
    }

    fn binding(
        &mut self,
        binding: BindingId,
        expected: BindingKind,
    ) -> Result<BindingRecord, Diagnostic> {
        match self.unit.required_owner(
            OwnerKey::Binding(binding),
            "expression references a missing binding",
        )? {
            OwnerRecord::Binding(record) if record.kind == expected => Ok(record),
            OwnerRecord::Binding(_) => Err(compiler_corrupt(
                "compiler_binding_kind",
                "expression binding has the wrong exact binding kind",
            )),
            _ => Err(compiler_corrupt(
                "compiler_binding_owner_kind",
                "binding identity names another owner kind",
            )),
        }
    }

    fn bind(&mut self, reference: LocalValueReference) -> Result<u32, Diagnostic> {
        self.unit.canonical.code_step()?;
        if self.locals.contains_key(&reference) {
            return Err(compiler_corrupt(
                "compiler_local_duplicate",
                "one exact local identity is bound twice in one lexical scope",
            ));
        }
        let local = self.next_local;
        self.next_local = self.next_local.checked_add(1).ok_or_else(|| {
            compiler_error(
                DiagnosticClass::Resource,
                "compiler_local_count",
                "compiled local count overflows its dense index domain",
            )
        })?;
        if self.next_local as usize > MAXIMUM_COMPILER_UNIT_ITEMS {
            return Err(compiler_error(
                DiagnosticClass::Resource,
                "compiler_local_count",
                "compiled local count exceeds the compiler-unit work bound",
            ));
        }
        self.locals.insert(reference, local);
        Ok(local)
    }

    fn unbind_all(&mut self, references: &[LocalValueReference]) {
        for reference in references {
            self.unbind(*reference);
        }
    }

    fn unbind(&mut self, reference: LocalValueReference) {
        self.locals.remove(&reference);
        if let LocalValueReference::LexicalBinding(binding) = reference {
            self.live_loans.remove(&binding);
            self.read_scope_order.retain(|active| *active != binding);
        }
    }

    fn enter_read_scope(&mut self, binding: BindingId) -> Result<(), Diagnostic> {
        self.unit.canonical.code_step()?;
        self.unit
            .derived_type_admission
            .parallel
            .reserve::<BindingId>(2)?;
        if !self.live_loans.insert(binding) {
            return Err(compiler_corrupt(
                "compiler_read_view_scope",
                "one exact loan scope is introduced twice",
            ));
        }
        self.read_scope_order.push(binding);
        Ok(())
    }

    fn admitted_lexical_scope(
        &mut self,
        binding: BindingId,
    ) -> Result<Option<ReadScopeArgument>, Diagnostic> {
        let Some((callable, generation)) = self.function_scope else {
            return Ok(None);
        };
        if generation != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION {
            return Ok(None);
        }
        self.unit.with_admission_read(|read, admission| {
            crate::platform::kernel::owned_associated::read_view_admission::lexical_owner(
                read,
                callable,
                binding,
                crate::platform::kernel::owned_associated::AssociatedTypeContext::Lexical(callable),
                admission,
            )
        })?;
        self.unit.tables.declaration(callable)?;
        Ok(Some(ReadScopeArgument::Lexical { callable, binding }))
    }

    fn begin_loan_scope(
        &mut self,
        binding: BindingId,
    ) -> Result<Option<ReadScopeArgument>, Diagnostic> {
        let scope = self.admitted_lexical_scope(binding)?;
        if let Some(read_scope) = scope {
            self.push(CompiledInstruction::BeginReadViewScope { read_scope })?;
        }
        Ok(scope)
    }

    fn end_loan_scope(&mut self, scope: Option<ReadScopeArgument>) -> Result<(), Diagnostic> {
        if let Some(read_scope) = scope {
            self.push(CompiledInstruction::EndReadViewScope { read_scope })?;
        }
        Ok(())
    }

    fn clear_owning_local(&mut self, local: u32) -> Result<(), Diagnostic> {
        self.push(CompiledInstruction::Unit)?;
        self.push(CompiledInstruction::StoreLocal(local))?;
        Ok(())
    }

    fn push(&mut self, instruction: CompiledInstruction) -> Result<u32, Diagnostic> {
        self.unit.canonical.code_step()?;
        if self.instructions.len() == MAXIMUM_COMPILER_UNIT_ITEMS {
            return Err(compiler_error(
                DiagnosticClass::Resource,
                "compiler_instruction_count",
                "compiled instruction count exceeds the compiler-unit bound",
            ));
        }
        let index = u32_count("compiled instruction index", self.instructions.len())?;
        self.instructions.push(instruction);
        self.unit.work.instructions_emitted = self.unit.work.instructions_emitted.saturating_add(1);
        Ok(index)
    }

    fn next_instruction(&self) -> Result<u32, Diagnostic> {
        u32_count("compiled instruction target", self.instructions.len())
    }
}

#[derive(Default)]
struct TablesBuilder {
    declarations: InternTable<DeclarationReference>,
    fields: InternTable<FieldReference>,
    cases: InternTable<CaseReference>,
    requirements: InternTable<RequirementReference>,
    operations: InternTable<OperationReference>,
    ports: InternTable<PortReference>,
    types: InternTable<TypeObjectDigest>,
    structural_names: InternTable<crate::platform::kernel::Name>,
    read_view_fields: InternTable<ReadViewFieldReference>,
    texts: InternTable<CompiledText>,
}

impl TablesBuilder {
    fn from_tables(tables: &CompilationTables) -> Self {
        Self {
            declarations: InternTable::from_values(&tables.declarations),
            fields: InternTable::from_values(&tables.fields),
            cases: InternTable::from_values(&tables.cases),
            requirements: InternTable::from_values(&tables.requirements),
            operations: InternTable::from_values(&tables.operations),
            ports: InternTable::from_values(&tables.ports),
            types: InternTable::from_values(&tables.types),
            structural_names: InternTable::from_values(&tables.structural_names),
            read_view_fields: InternTable::from_values(&tables.read_view_fields),
            texts: InternTable::from_values(&tables.texts),
        }
    }
    fn declaration(&mut self, value: DeclarationReference) -> Result<u32, Diagnostic> {
        self.declarations.intern(value, "declaration relocations")
    }

    fn field(&mut self, value: FieldReference) -> Result<u32, Diagnostic> {
        self.fields.intern(value, "field relocations")
    }

    fn case(&mut self, value: CaseReference) -> Result<u32, Diagnostic> {
        self.cases.intern(value, "case relocations")
    }

    fn read_view_field(&mut self, value: ReadViewFieldReference) -> Result<u32, Diagnostic> {
        self.read_view_fields
            .intern(value, "inline read-view field relocations")
    }

    fn requirement(&mut self, value: RequirementReference) -> Result<u32, Diagnostic> {
        self.requirements.intern(value, "requirement relocations")
    }

    fn operation(&mut self, value: OperationReference) -> Result<u32, Diagnostic> {
        self.operations.intern(value, "operation relocations")
    }

    fn port(&mut self, value: PortReference) -> Result<u32, Diagnostic> {
        self.ports.intern(value, "port relocations")
    }

    fn ty(&mut self, value: TypeObjectDigest) -> Result<u32, Diagnostic> {
        self.types.intern(value, "type relocations")
    }

    fn structural_name(&mut self, value: crate::platform::kernel::Name) -> Result<u32, Diagnostic> {
        self.structural_names
            .intern(value, "structural field names")
    }

    fn text(&mut self, value: CompiledText) -> Result<u32, Diagnostic> {
        self.texts.intern(value, "text constants")
    }

    fn finish(self) -> CompilationTables {
        CompilationTables {
            declarations: self.declarations.values,
            fields: self.fields.values,
            cases: self.cases.values,
            requirements: self.requirements.values,
            operations: self.operations.values,
            ports: self.ports.values,
            types: self.types.values,
            structural_names: self.structural_names.values,
            read_view_fields: self.read_view_fields.values,
            texts: self.texts.values,
        }
    }
}

struct InternTable<T> {
    indexes: BTreeMap<T, u32>,
    values: Vec<T>,
}

impl<T> Default for InternTable<T> {
    fn default() -> Self {
        Self {
            indexes: BTreeMap::new(),
            values: Vec::new(),
        }
    }
}

impl<T: Clone + Ord> InternTable<T> {
    fn from_values(values: &[T]) -> Self {
        Self {
            indexes: values
                .iter()
                .cloned()
                .enumerate()
                .map(|(index, value)| (value, index as u32))
                .collect(),
            values: values.to_vec(),
        }
    }
    fn intern(&mut self, value: T, label: &'static str) -> Result<u32, Diagnostic> {
        if let Some(index) = self.indexes.get(&value) {
            return Ok(*index);
        }
        if self.values.len() == MAXIMUM_COMPILER_UNIT_ITEMS {
            return Err(compiler_error(
                DiagnosticClass::Resource,
                "compiler_relocation_count",
                format!("{label} exceed the compiler-unit bound"),
            ));
        }
        let index = u32_count(label, self.values.len())?;
        self.values.push(value.clone());
        self.indexes.insert(value, index);
        Ok(index)
    }
}

fn compiled_text(value: TextValue) -> CompiledText {
    match value {
        TextValue::Inline { text } => CompiledText::Inline(text),
        TextValue::Blob { digest, bytes } => CompiledText::Blob { digest, bytes },
    }
}

fn u32_count(label: &'static str, count: usize) -> Result<u32, Diagnostic> {
    u32::try_from(count).map_err(|_| {
        compiler_error(
            DiagnosticClass::Resource,
            "compiler_dense_index",
            format!("{label} does not fit the dense compiler index domain"),
        )
    })
}

fn compiler_corrupt(code: &'static str, message: impl Into<String>) -> Diagnostic {
    compiler_error(DiagnosticClass::Corrupt, code, message)
}

fn compiler_error(
    class: DiagnosticClass,
    code: &'static str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod read_view_lower_tests {
    use super::*;
    use crate::platform::kernel::{
        DeclarationVisibility, FunctionDeclaration, Name, OwnerHeader, OwnerKind, ParameterRecord,
        ReadScopeReference, ReadViewDeclaration, ReadViewFieldInitializer,
    };
    use crate::platform::semantic_id::{ModuleId, ReadScopeParameterId};

    struct Source {
        package: PackageId,
        owners: BTreeMap<OwnerKey, OwnerRecord>,
        types: BTreeMap<TypeObjectDigest, TypeObject>,
        function: DeclarationId,
        parameter: ParameterId,
        root: ExpressionId,
        view: DeclarationReference,
        fields: [FieldId; 2],
        buffer: TypeObjectDigest,
    }

    impl CodeRead for Source {
        fn code_dependency(&self, package: PackageId) -> Result<CanonicalRead<bool>, Diagnostic> {
            Ok(observed(package == self.package))
        }
        fn code_owner(
            &self,
            owner: OwnerKey,
        ) -> Result<CanonicalRead<Option<OwnerRecord>>, Diagnostic> {
            Ok(observed(self.owners.get(&owner).cloned()))
        }
        fn code_type(
            &self,
            ty: TypeObjectDigest,
        ) -> Result<CanonicalRead<Option<TypeObject>>, Diagnostic> {
            Ok(observed(self.types.get(&ty).cloned()))
        }
        fn code_interface(
            &self,
            _: PackageId,
            _: OwnerKey,
        ) -> Result<CanonicalRead<Option<PackageInterfaceRecord>>, Diagnostic> {
            Ok(observed(None))
        }
    }

    /// Supplies the local point reads used by the production declaration compiler. These
    /// hand-built lowering fixtures have no accepted root, repository or witness binding.
    struct DeclarationRead<'a>(&'a Source);

    impl CanonicalBaseRead for DeclarationRead<'_> {
        fn semantic_root(&self) -> &crate::platform::kernel::SemanticRoot {
            panic!("lowering fixture has no persisted semantic root")
        }

        fn repository_id(&self) -> crate::platform::semantic_id::RepositoryId {
            panic!("lowering fixture has no accepted repository binding")
        }

        fn package_id(&self) -> PackageId {
            self.0.package
        }

        fn exact_revision(&self) -> Option<crate::platform::semantic_id::RevisionId> {
            None
        }

        fn owner_count(&self) -> u64 {
            self.0.owners.len() as u64
        }

        fn dependency_count(&self) -> u64 {
            0
        }

        fn retirement_count(&self) -> u64 {
            0
        }

        fn read_owner(
            &self,
            owner: OwnerKey,
        ) -> Result<CanonicalRead<Option<OwnerRecord>>, Diagnostic> {
            self.0.code_owner(owner)
        }

        fn read_type_object(
            &self,
            ty: TypeObjectDigest,
        ) -> Result<CanonicalRead<Option<TypeObject>>, Diagnostic> {
            self.0.code_type(ty)
        }

        fn read_package_interface_owner(
            &self,
            _: &crate::platform::kernel::DependencyRecord,
            _: OwnerKey,
        ) -> Result<CanonicalRead<Option<PackageInterfaceRecord>>, Diagnostic> {
            Ok(observed(None))
        }

        fn read_dependency(
            &self,
            _: PackageId,
        ) -> Result<CanonicalRead<Option<crate::platform::kernel::DependencyRecord>>, Diagnostic>
        {
            Ok(observed(None))
        }

        fn read_dependencies(
            &self,
        ) -> Result<CanonicalRead<Vec<crate::platform::kernel::DependencyRecord>>, Diagnostic>
        {
            Ok(observed(Vec::new()))
        }

        fn read_retirement(
            &self,
            _: OwnerKey,
        ) -> Result<CanonicalRead<Option<crate::platform::kernel::RetirementRecord>>, Diagnostic>
        {
            Ok(observed(None))
        }
    }

    fn observed<T>(value: T) -> CanonicalRead<T> {
        CanonicalRead {
            value,
            work: CanonicalReadWork::default(),
        }
    }

    impl Source {
        fn ty(&mut self, form: TypeForm) -> TypeObjectDigest {
            let object = TypeObject::new(form).unwrap();
            let digest = encode_type_object(&object).unwrap().0;
            self.types.insert(digest, object);
            digest
        }

        fn expression(&mut self, id: ExpressionId, operation: ExpressionOperation) {
            self.owners.insert(
                OwnerKey::Expression(id),
                OwnerRecord::Expression(ExpressionRecord::new(id, operation).unwrap()),
            );
        }

        fn function_record(&self) -> DeclarationRecord {
            let OwnerRecord::Declaration(record) =
                &self.owners[&OwnerKey::Declaration(self.function)]
            else {
                panic!("function owner")
            };
            record.clone()
        }

        fn lower(&self) -> Result<(CompiledCode, CompilationTables), Diagnostic> {
            let read = DeclarationRead(self);
            let mut builder = UnitBuilder {
                canonical: &read,
                package: self.package,
                scope: Some(self.function),
                tables: TablesBuilder::default(),
                derived_type_objects: BTreeMap::new(),
                derived_type_admission: DerivedTypeAdmission::default(),
                work: CompilationWork::default(),
            };
            let CompilationPayload::Function { code, .. } =
                builder.compile_declaration(self.function, self.function_record())?
            else {
                panic!("function payload")
            };
            builder.complete_owned_record_type_closure()?;
            Ok((code, builder.tables.finish()))
        }
    }

    fn source(count: usize, authored_order: &[usize]) -> Source {
        let package = PackageId::from_bytes([1; 16]).unwrap();
        let function = DeclarationId::from_bytes([2; 16]).unwrap();
        let parameter = ParameterId::from_bytes([3; 16]).unwrap();
        let root = ExpressionId::from_bytes([4; 16]).unwrap();
        let view = DeclarationReference {
            package,
            declaration: DeclarationId::from_bytes([5; 16]).unwrap(),
        };
        let module = ModuleId::from_bytes([6; 16]).unwrap();
        let formal = ReadScopeParameter {
            id: ReadScopeParameterId::from_bytes([7; 16]).unwrap(),
            name: Name::new("source_scope").unwrap(),
        };
        let scope = ReadScopeArgument::Parameter(ReadScopeReference {
            binder: ReadScopeBinder::Declaration(DeclarationReference {
                package,
                declaration: function,
            }),
            parameter: formal.id,
        });
        let mut source = Source {
            package,
            owners: BTreeMap::new(),
            types: BTreeMap::new(),
            function,
            parameter,
            root,
            view,
            fields: [
                FieldId::from_bytes([11; 16]).unwrap(),
                FieldId::from_bytes([12; 16]).unwrap(),
            ],
            buffer: TypeObjectDigest::from_bytes([1; 32]),
        };
        let scalar = source.ty(TypeForm::I64);
        source.buffer = source.ty(TypeForm::ByteBuffer);
        let result = source.ty(TypeForm::ReadViewApplied {
            declaration: view,
            arguments: Vec::new(),
            read_scope: scope,
        });
        let fields = (0..count)
            .map(|index| ReadViewField {
                id: source.fields[index],
                name: Name::new(if index == 0 { "first" } else { "second" }).unwrap(),
                ty: scalar,
                kind: ReadViewFieldKind::Ordinary,
            })
            .collect();
        source.owners.insert(
            OwnerKey::Declaration(view.declaration),
            OwnerRecord::Declaration(DeclarationRecord {
                header: OwnerHeader::new(
                    OwnerKey::Declaration(view.declaration),
                    OwnerKind::ReadView,
                ),
                module,
                name: Name::new("Descriptor").unwrap(),
                visibility: DeclarationVisibility::Private,
                payload: DeclarationPayload::ReadView(ReadViewDeclaration {
                    type_parameters: Vec::new(),
                    scope_parameter: ReadScopeParameter {
                        id: ReadScopeParameterId::from_bytes([8; 16]).unwrap(),
                        name: Name::new("view_scope").unwrap(),
                    },
                    fields,
                }),
            }),
        );
        source.owners.insert(
            OwnerKey::Parameter(parameter),
            OwnerRecord::Parameter(ParameterRecord {
                header: OwnerHeader::new(OwnerKey::Parameter(parameter), OwnerKind::Parameter),
                parent: ParameterParent::Function(function),
                name: Name::new("source").unwrap(),
                ty: source.buffer,
                use_mode: ParameterUse::Borrow,
                resource_requirement: None,
            }),
        );
        source.owners.insert(
            OwnerKey::Declaration(function),
            OwnerRecord::Declaration(DeclarationRecord {
                header: OwnerHeader::new(OwnerKey::Declaration(function), OwnerKind::PureFunction),
                module,
                name: Name::new("build").unwrap(),
                visibility: DeclarationVisibility::Private,
                payload: DeclarationPayload::Function(FunctionDeclaration {
                    implementation_parameters: Vec::new(),
                    requirement_parameters: Vec::new(),
                    effect_parameters: Vec::new(),
                    type_parameters: Vec::new(),
                    parameters: vec![parameter],
                    result: CallableResult {
                        ty: result,
                        relation: ResultRelation::ViewFrom {
                            source: parameter,
                            scope,
                        },
                    },
                    read_scope_parameters: vec![formal],
                    read_scope_sources: vec![ReadScopeSource {
                        source: parameter,
                        scope,
                    }],
                    effect: FunctionEffect::Pure,
                    body: root,
                }),
            }),
        );
        let initializer_ids = [
            ExpressionId::from_bytes([21; 16]).unwrap(),
            ExpressionId::from_bytes([22; 16]).unwrap(),
        ];
        for (id, value) in initializer_ids.into_iter().zip([17, -29]) {
            source.expression(id, ExpressionOperation::I64 { value });
        }
        source.expression(
            root,
            ExpressionOperation::CreateReadView {
                declaration: view,
                type_arguments: Vec::new(),
                read_scope: scope,
                source: LocalValueReference::FunctionParameter(parameter),
                fields: authored_order
                    .iter()
                    .map(|index| ReadViewFieldInitializer {
                        field: source.fields[*index],
                        value: initializer_ids[*index],
                    })
                    .collect(),
            },
        );
        source
    }

    #[test]
    fn zero_field_view_retains_explicit_source_guard_and_view_return() {
        let source = source(0, &[]);
        let (code, tables) = source.lower().unwrap();
        let view_type = tables
            .types
            .iter()
            .position(|ty| {
                matches!(
                    source.types.get(ty).map(|object| &object.form),
                    Some(TypeForm::ReadViewApplied { .. })
                )
            })
            .unwrap() as u32;
        assert_eq!(code.parameter_count, 1);
        assert_eq!(
            code.instructions,
            vec![
                CompiledInstruction::BeginReadView {
                    view_type,
                    source_local: 0,
                    fields: Vec::new()
                },
                CompiledInstruction::FinishReadView {
                    view_type,
                    fields: Vec::new()
                },
                CompiledInstruction::ReturnView,
            ]
        );
        assert!(tables.read_view_fields.is_empty());
    }

    #[test]
    fn view_constructor_protects_source_before_authored_field_order() {
        let source = source(2, &[1, 0]);
        let (code, tables) = source.lower().unwrap();
        assert_eq!(
            tables.read_view_fields,
            source.fields.map(|field| ReadViewFieldReference {
                declaration: source.view,
                field
            })
        );
        assert!(
            matches!(&code.instructions[0], CompiledInstruction::BeginReadView { source_local: 0, fields, .. } if fields == &[1, 0])
        );
        assert_eq!(code.instructions[1], CompiledInstruction::I64(-29));
        assert_eq!(code.instructions[2], CompiledInstruction::I64(17));
        assert!(
            matches!(&code.instructions[3], CompiledInstruction::FinishReadView { fields, .. } if fields == &[1, 0])
        );
        assert_eq!(code.instructions[4], CompiledInstruction::ReturnView);
    }

    #[test]
    fn view_constructor_refuses_missing_repeated_and_foreign_inline_fields() {
        assert_eq!(
            source(2, &[0]).lower().unwrap_err().code,
            "compiler_read_view_field"
        );
        // Build a locally valid record before faulting this private raw reader.
        // The checked constructor correctly refuses duplicate fields earlier;
        // this control independently reaches the compiler inventory boundary.
        let mut repeated = source(2, &[0, 1]);
        let OwnerRecord::Expression(record) = repeated
            .owners
            .get_mut(&OwnerKey::Expression(repeated.root))
            .unwrap()
        else {
            panic!("constructor owner")
        };
        let ExpressionOperation::CreateReadView { fields, .. } = &mut record.operation else {
            panic!("constructor operation")
        };
        fields[1].field = fields[0].field;
        assert_eq!(
            repeated.lower().unwrap_err().code,
            "compiler_read_view_field"
        );
        let mut foreign = source(2, &[0, 1]);
        let OwnerRecord::Expression(record) = foreign
            .owners
            .get_mut(&OwnerKey::Expression(foreign.root))
            .unwrap()
        else {
            panic!("constructor owner")
        };
        let ExpressionOperation::CreateReadView { fields, .. } = &mut record.operation else {
            panic!("constructor operation")
        };
        fields[1].field = FieldId::from_bytes([99; 16]).unwrap();
        assert_eq!(
            foreign.lower().unwrap_err().code,
            "compiler_read_view_field"
        );
    }

    #[test]
    fn inline_field_relocations_keep_equal_field_ids_in_distinct_declarations() {
        let source = source(0, &[]);
        let mut tables = TablesBuilder::default();
        let first = ReadViewFieldReference {
            declaration: source.view,
            field: source.fields[0],
        };
        let second = ReadViewFieldReference {
            declaration: DeclarationReference {
                package: source.package,
                declaration: source.function,
            },
            field: source.fields[0],
        };
        assert_eq!(tables.read_view_field(first).unwrap(), 0);
        assert_eq!(tables.read_view_field(second).unwrap(), 1);
        assert_eq!(tables.read_view_field(first).unwrap(), 0);
        assert!(tables.fields.values.is_empty());
        assert_eq!(tables.finish().read_view_fields, vec![first, second]);
    }

    fn borrowed_call_source() -> Source {
        let generation = crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION;
        let mut source = source(0, &[]);
        let callee = DeclarationId::from_bytes([40; 16]).unwrap();
        let parameters = [
            ParameterId::from_bytes([41; 16]).unwrap(),
            ParameterId::from_bytes([42; 16]).unwrap(),
        ];
        let callee_root = ExpressionId::from_bytes([43; 16]).unwrap();
        let call = ExpressionId::from_bytes([44; 16]).unwrap();
        let arguments = [
            ExpressionId::from_bytes([45; 16]).unwrap(),
            ExpressionId::from_bytes([46; 16]).unwrap(),
        ];
        let binding = BindingId::from_bytes([47; 16]).unwrap();
        let body = ExpressionId::from_bytes([48; 16]).unwrap();
        let header = |owner, kind| {
            let mut header = OwnerHeader::new(owner, kind);
            header.contract_version = generation;
            header
        };
        let mut declaration = source.function_record();
        declaration.header.contract_version = generation;
        let DeclarationPayload::Function(function) = &mut declaration.payload else {
            panic!("function")
        };
        function.result = CallableResult {
            ty: source.buffer,
            relation: ResultRelation::ReadFrom {
                source: source.parameter,
            },
        };
        function.read_scope_parameters.clear();
        function.read_scope_sources.clear();
        source.owners.insert(
            OwnerKey::Declaration(source.function),
            OwnerRecord::Declaration(declaration.clone()),
        );
        let OwnerRecord::Parameter(parameter) = source
            .owners
            .get_mut(&OwnerKey::Parameter(source.parameter))
            .unwrap()
        else {
            panic!("parameter")
        };
        parameter.header.contract_version = generation;
        for (index, parameter) in parameters.into_iter().enumerate() {
            source.owners.insert(
                OwnerKey::Parameter(parameter),
                OwnerRecord::Parameter(ParameterRecord {
                    header: header(OwnerKey::Parameter(parameter), OwnerKind::Parameter),
                    parent: ParameterParent::Function(callee),
                    name: Name::new(if index == 0 { "unselected" } else { "selected" }).unwrap(),
                    ty: source.buffer,
                    use_mode: ParameterUse::Borrow,
                    resource_requirement: None,
                }),
            );
        }
        declaration.header = header(OwnerKey::Declaration(callee), OwnerKind::PureFunction);
        declaration.name = Name::new("second_source").unwrap();
        let DeclarationPayload::Function(function) = &mut declaration.payload else {
            panic!("callee")
        };
        function.parameters = parameters.to_vec();
        function.result = CallableResult {
            ty: source.buffer,
            relation: ResultRelation::ReadFrom {
                source: parameters[1],
            },
        };
        function.body = callee_root;
        source.owners.insert(
            OwnerKey::Declaration(callee),
            OwnerRecord::Declaration(declaration),
        );
        source.expression(
            callee_root,
            ExpressionOperation::Local {
                value: LocalValueReference::FunctionParameter(parameters[1]),
            },
        );
        for argument in arguments {
            source.expression(
                argument,
                ExpressionOperation::Local {
                    value: LocalValueReference::FunctionParameter(source.parameter),
                },
            );
        }
        source.expression(
            call,
            ExpressionOperation::Call {
                function: DeclarationReference {
                    package: source.package,
                    declaration: callee,
                },
                type_arguments: Vec::new(),
                read_scope_arguments: Vec::new(),
                effect_arguments: Vec::new(),
                requirement_arguments: Vec::new(),
                arguments: arguments.to_vec(),
            },
        );
        source.owners.insert(
            OwnerKey::Binding(binding),
            OwnerRecord::Binding(BindingRecord {
                header: header(OwnerKey::Binding(binding), OwnerKind::Binding),
                name: Name::new("loan").unwrap(),
                kind: BindingKind::OwnedBorrow,
                value: None,
                declared_type: Some(source.buffer),
            }),
        );
        source.expression(
            body,
            ExpressionOperation::Local {
                value: LocalValueReference::LexicalBinding(binding),
            },
        );
        source.expression(
            source.root,
            ExpressionOperation::BorrowCall {
                call,
                binding,
                body,
            },
        );
        for record in source.owners.values_mut() {
            if let OwnerRecord::Expression(record) = record {
                record.contract_version = generation;
            }
        }
        source
    }

    #[test]
    fn borrowed_call_keeps_selected_ordered_source_despite_equal_types_and_backing_local() {
        let source = borrowed_call_source();
        let (code, _) = source.lower().unwrap();
        assert_eq!(
            code.instructions
                .iter()
                .filter(|instruction| matches!(
                    instruction,
                    CompiledInstruction::LoadLocal {
                        local: 0,
                        use_mode: ParameterUse::Borrow
                    }
                ))
                .count(),
            2
        );
        assert!(code.instructions.iter().any(|instruction| matches!(
            instruction,
            CompiledInstruction::BeginBorrowCall {
                source_local: 0,
                source_position: 1
            }
        )));
        assert!(matches!(
            code.instructions.first(),
            Some(CompiledInstruction::BeginReadViewScope { .. })
        ));
        assert!(matches!(
            code.instructions[code.instructions.len() - 3],
            CompiledInstruction::EndReadViewScope { .. }
        ));
        assert_eq!(
            code.instructions.last(),
            Some(&CompiledInstruction::ReturnBorrowed)
        );
    }

    #[test]
    fn borrowed_call_keeps_complete_scoped_adoption_and_cleanup_order() {
        let source = borrowed_call_source();
        let (code, _) = source.lower().unwrap();
        assert_eq!(code.instructions.len(), 10);
        let Some(CompiledInstruction::BeginReadViewScope { read_scope }) =
            code.instructions.first()
        else {
            panic!("borrowed call begins its lexical read scope")
        };
        assert!(matches!(
            code.instructions[3],
            CompiledInstruction::BeginBorrowCall {
                source_local: 0,
                source_position: 1
            }
        ));
        assert!(matches!(
            code.instructions[5],
            CompiledInstruction::AdoptBorrowResult {
                source_local: 0,
                binding_local: 1,
                ..
            }
        ));
        assert_eq!(
            code.instructions[7],
            CompiledInstruction::EndReadViewScope {
                read_scope: *read_scope
            }
        );
        assert_eq!(
            code.instructions[8],
            CompiledInstruction::EndOwnedBorrow { binding_local: 1 }
        );
        assert_eq!(code.instructions[9], CompiledInstruction::ReturnBorrowed);
    }

    #[test]
    fn current_function_bounds_ordinary_loan_syntax_and_rejects_foreign_owners() {
        let current = crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION;
        let source = borrowed_call_source();
        let (code, _) = source.lower().unwrap();
        assert!(matches!(
            code.instructions.first(),
            Some(CompiledInstruction::BeginReadViewScope { .. })
        ));
        assert!(code.instructions.iter().any(|instruction| matches!(
            instruction,
            CompiledInstruction::EndReadViewScope { .. }
        )));
        for owner in [
            OwnerKey::Expression(source.root),
            OwnerKey::Binding(BindingId::from_bytes([47; 16]).unwrap()),
            OwnerKey::Parameter(source.parameter),
        ] {
            let record = &source.owners[&owner];
            assert_eq!(record.header().contract_version, current);
            crate::platform::kernel::encode_owner(record).unwrap();
            let mut foreign = record.clone();
            foreign.set_encoding_for_edit(current ^ 1);
            assert!(crate::platform::kernel::encode_owner(&foreign).is_err());
        }
    }

    #[test]
    fn current_constant_owned_loan_has_no_invented_callable_scope() {
        let mut source = source(0, &[]);
        source.owners.clear();
        source.types.clear();
        let scalar = source.ty(TypeForm::I64);
        let inner = source.ty(TypeForm::OwnedProduct {
            fields: vec![StructuralTypeField {
                name: Name::new("tag").unwrap(),
                ty: scalar,
            }],
        });
        let outer = source.ty(TypeForm::OwnedProduct {
            fields: vec![StructuralTypeField {
                name: Name::new("child").unwrap(),
                ty: inner,
            }],
        });
        let owning = BindingId::from_bytes([80; 16]).unwrap();
        let loan = BindingId::from_bytes([81; 16]).unwrap();
        let empty = ExpressionId::from_bytes([82; 16]).unwrap();
        let packed = ExpressionId::from_bytes([83; 16]).unwrap();
        let local = ExpressionId::from_bytes([84; 16]).unwrap();
        let body = ExpressionId::from_bytes([85; 16]).unwrap();
        let borrowed = ExpressionId::from_bytes([86; 16]).unwrap();
        let tag = ExpressionId::from_bytes([87; 16]).unwrap();
        source.owners.insert(
            OwnerKey::Binding(owning),
            OwnerRecord::Binding(BindingRecord {
                header: OwnerHeader::new(OwnerKey::Binding(owning), OwnerKind::Binding),
                name: Name::new("owner").unwrap(),
                kind: BindingKind::Let,
                value: Some(packed),
                declared_type: Some(outer),
            }),
        );
        source.owners.insert(
            OwnerKey::Binding(loan),
            OwnerRecord::Binding(BindingRecord {
                header: OwnerHeader::new(OwnerKey::Binding(loan), OwnerKind::Binding),
                name: Name::new("child").unwrap(),
                kind: BindingKind::OwnedBorrow,
                value: None,
                declared_type: Some(inner),
            }),
        );
        source.expression(tag, ExpressionOperation::I64 { value: 0 });
        source.expression(
            empty,
            ExpressionOperation::PackOwned {
                product_type: inner,
                fields: vec![crate::platform::kernel::OwnedProductExpressionField {
                    name: Name::new("tag").unwrap(),
                    value: tag,
                }],
            },
        );
        source.expression(
            packed,
            ExpressionOperation::PackOwned {
                product_type: outer,
                fields: vec![crate::platform::kernel::OwnedProductExpressionField {
                    name: Name::new("child").unwrap(),
                    value: empty,
                }],
            },
        );
        source.expression(
            local,
            ExpressionOperation::Local {
                value: LocalValueReference::LexicalBinding(owning),
            },
        );
        source.expression(body, ExpressionOperation::I64 { value: 73 });
        source.expression(
            borrowed,
            ExpressionOperation::BorrowOwnedField {
                product_type: outer,
                source: local,
                field: Name::new("child").unwrap(),
                binding: loan,
                body,
            },
        );
        source.expression(
            source.root,
            ExpressionOperation::Let {
                bindings: vec![owning],
                body: borrowed,
            },
        );
        let record = DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(source.function), OwnerKind::Constant),
            module: ModuleId::from_bytes([6; 16]).unwrap(),
            name: Name::new("ordinary_constant").unwrap(),
            visibility: DeclarationVisibility::Private,
            payload: DeclarationPayload::Constant {
                ty: scalar,
                value: source.root,
            },
        };
        source.owners.insert(
            OwnerKey::Declaration(source.function),
            OwnerRecord::Declaration(record.clone()),
        );
        let read = DeclarationRead(&source);
        let mut builder = UnitBuilder {
            canonical: &read,
            package: source.package,
            scope: Some(source.function),
            tables: TablesBuilder::default(),
            derived_type_objects: BTreeMap::new(),
            derived_type_admission: DerivedTypeAdmission::default(),
            work: CompilationWork::default(),
        };
        let CompilationPayload::Constant { code, .. } = builder
            .compile_declaration(source.function, record)
            .unwrap()
        else {
            panic!("constant")
        };
        assert_eq!(code.parameter_count, 0);
        assert!(code.instructions.iter().any(|instruction| matches!(
            instruction,
            CompiledInstruction::BorrowOwnedField { .. }
        )));
        assert!(
            code.instructions.iter().any(|instruction| matches!(
                instruction,
                CompiledInstruction::EndOwnedBorrow { .. }
            ))
        );
        assert!(!code.instructions.iter().any(|instruction| matches!(
            instruction,
            CompiledInstruction::BeginReadViewScope { .. }
                | CompiledInstruction::EndReadViewScope { .. }
        )));
        assert_eq!(code.instructions.last(), Some(&CompiledInstruction::Return));
    }

    #[test]
    fn forwarded_view_result_adopts_an_owning_descriptor_and_closes_only_its_own_scope() {
        let mut source = borrowed_call_source();
        let callee = DeclarationId::from_bytes([40; 16]).unwrap();
        let selected = ParameterId::from_bytes([42; 16]).unwrap();
        let callee_root = ExpressionId::from_bytes([43; 16]).unwrap();
        let call = ExpressionId::from_bytes([44; 16]).unwrap();
        let binding = BindingId::from_bytes([47; 16]).unwrap();
        let body = ExpressionId::from_bytes([48; 16]).unwrap();
        let caller = DeclarationReference {
            package: source.package,
            declaration: source.function,
        };
        let caller_formal = ReadScopeParameter {
            id: ReadScopeParameterId::from_bytes([7; 16]).unwrap(),
            name: Name::new("source_scope").unwrap(),
        };
        let caller_scope = ReadScopeArgument::Parameter(ReadScopeReference {
            binder: ReadScopeBinder::Declaration(caller),
            parameter: caller_formal.id,
        });
        let callee_formal = ReadScopeParameter {
            id: ReadScopeParameterId::from_bytes([55; 16]).unwrap(),
            name: Name::new("returned_scope").unwrap(),
        };
        let callee_scope = ReadScopeArgument::Parameter(ReadScopeReference {
            binder: ReadScopeBinder::Declaration(DeclarationReference {
                package: source.package,
                declaration: callee,
            }),
            parameter: callee_formal.id,
        });
        let caller_type = source.ty(TypeForm::ReadViewApplied {
            declaration: source.view,
            arguments: Vec::new(),
            read_scope: caller_scope,
        });
        let callee_type = source.ty(TypeForm::ReadViewApplied {
            declaration: source.view,
            arguments: Vec::new(),
            read_scope: callee_scope,
        });
        for (declaration, formal, scope, ty, selected) in [
            (
                source.function,
                caller_formal,
                caller_scope,
                caller_type,
                source.parameter,
            ),
            (callee, callee_formal, callee_scope, callee_type, selected),
        ] {
            let OwnerRecord::Declaration(record) = source
                .owners
                .get_mut(&OwnerKey::Declaration(declaration))
                .unwrap()
            else {
                panic!("function")
            };
            let DeclarationPayload::Function(function) = &mut record.payload else {
                panic!("function payload")
            };
            function.result = CallableResult {
                ty,
                relation: ResultRelation::ViewFrom {
                    source: selected,
                    scope,
                },
            };
            function.read_scope_parameters = vec![formal];
            function.read_scope_sources = vec![ReadScopeSource {
                source: selected,
                scope,
            }];
        }
        source.expression(
            callee_root,
            ExpressionOperation::CreateReadView {
                declaration: source.view,
                type_arguments: Vec::new(),
                read_scope: callee_scope,
                source: LocalValueReference::FunctionParameter(selected),
                fields: Vec::new(),
            },
        );
        let OwnerRecord::Expression(record) =
            source.owners.get_mut(&OwnerKey::Expression(call)).unwrap()
        else {
            panic!("call")
        };
        let ExpressionOperation::Call {
            read_scope_arguments,
            ..
        } = &mut record.operation
        else {
            panic!("named call")
        };
        *read_scope_arguments = vec![caller_scope];
        let OwnerRecord::Binding(record) =
            source.owners.get_mut(&OwnerKey::Binding(binding)).unwrap()
        else {
            panic!("result binding")
        };
        record.kind = BindingKind::ReadView;
        record.declared_type = Some(caller_type);
        source.expression(
            body,
            ExpressionOperation::MoveReadView {
                value: LocalValueReference::LexicalBinding(binding),
            },
        );
        let (code, _) = source.lower().unwrap();
        assert!(code.instructions.iter().any(|instruction| matches!(
            instruction,
            CompiledInstruction::AdoptViewResult {
                source_local: 0,
                binding_local: 1,
                ..
            }
        )));
        assert!(!code.instructions.iter().any(|instruction| matches!(
            instruction,
            CompiledInstruction::AdoptBorrowResult { .. }
                | CompiledInstruction::EndOwnedBorrow { .. }
        )));
        assert!(code.instructions.iter().any(|instruction| matches!(instruction,
            CompiledInstruction::Call { read_scope_arguments, .. } if read_scope_arguments == &[caller_scope])));
        assert_eq!(
            code.instructions[code.instructions.len() - 2],
            CompiledInstruction::EndReadViewScope {
                read_scope: ReadScopeArgument::Lexical {
                    callable: caller,
                    binding
                },
            }
        );
        assert_eq!(
            code.instructions.last(),
            Some(&CompiledInstruction::ReturnView)
        );
    }

    #[test]
    fn owning_view_let_bounds_descriptor_lifetime_without_changing_source_scope() {
        let mut source = source(0, &[]);
        let binding = BindingId::from_bytes([74; 16]).unwrap();
        let root = ExpressionId::from_bytes([70; 16]).unwrap();
        let result = ExpressionId::from_bytes([71; 16]).unwrap();
        let drop = ExpressionId::from_bytes([72; 16]).unwrap();
        let body = ExpressionId::from_bytes([73; 16]).unwrap();
        let scalar = source.ty(TypeForm::I64);
        let mut function_record = source.function_record();
        let DeclarationPayload::Function(function) = &mut function_record.payload else {
            panic!("function")
        };
        let view_type = function.result.ty;
        let ResultRelation::ViewFrom {
            scope: source_scope,
            ..
        } = function.result.relation
        else {
            panic!("source result")
        };
        function.result = CallableResult {
            ty: scalar,
            relation: ResultRelation::Value,
        };
        function.body = root;
        source.owners.insert(
            OwnerKey::Declaration(source.function),
            OwnerRecord::Declaration(function_record),
        );
        source.owners.insert(
            OwnerKey::Binding(binding),
            OwnerRecord::Binding(BindingRecord {
                header: OwnerHeader::new(OwnerKey::Binding(binding), OwnerKind::Binding),
                name: Name::new("descriptor").unwrap(),
                kind: BindingKind::ReadView,
                value: Some(source.root),
                declared_type: Some(view_type),
            }),
        );
        source.expression(
            drop,
            ExpressionOperation::DropReadView {
                value: LocalValueReference::LexicalBinding(binding),
            },
        );
        source.expression(result, ExpressionOperation::I64 { value: 73 });
        source.expression(
            body,
            ExpressionOperation::Sequence {
                items: vec![drop, result],
            },
        );
        source.expression(
            root,
            ExpressionOperation::Let {
                bindings: vec![binding],
                body,
            },
        );
        source.root = root;
        let (code, tables) = source.lower().unwrap();
        let descriptor_scope = ReadScopeArgument::Lexical {
            callable: DeclarationReference {
                package: source.package,
                declaration: source.function,
            },
            binding,
        };
        assert_eq!(
            code.instructions.get(3),
            Some(&CompiledInstruction::BeginReadViewScope {
                read_scope: descriptor_scope
            })
        );
        assert!(matches!(
            code.instructions[0],
            CompiledInstruction::BeginReadView {
                source_local: 0,
                ..
            }
        ));
        assert_eq!(
            code.instructions[code.instructions.len() - 2],
            CompiledInstruction::EndReadViewScope {
                read_scope: descriptor_scope
            }
        );
        assert_eq!(code.instructions.last(), Some(&CompiledInstruction::Return));
        assert!(
            code.instructions
                .iter()
                .any(|instruction| matches!(instruction, CompiledInstruction::I64(73)))
        );
        assert_eq!(
            code.instructions
                .iter()
                .filter(|instruction| matches!(
                    instruction,
                    CompiledInstruction::DropReadView {
                        source_local: 1,
                        ..
                    }
                ))
                .count(),
            1
        );
        assert!(tables.types.contains(&view_type));
        let TypeForm::ReadViewApplied { read_scope, .. } = &source.types[&view_type].form else {
            panic!("view type")
        };
        assert_eq!(*read_scope, source_scope);
        assert_ne!(*read_scope, descriptor_scope);
    }
}
