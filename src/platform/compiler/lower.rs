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
    BindingKind, BindingRecord, CaseReference, DeclarationPayload, DeclarationRecord,
    DeclarationReference, ExactOwnerKey, ExpressionOperation, ExpressionRecord, FieldReference,
    FieldSelector, FunctionEffect, LocalValueReference, OperationReference, OwnerKey, OwnerRecord,
    PackageId, PackageInterfaceRecord, ParameterParent, ParameterUse, PortImplementation,
    PortReference, RelationEndpoint, RelationKind, RequirementReference, StructuralTypeField,
    TextValue, TypeForm, TypeObject, TypeObjectDigest, encode_type_object,
};
use crate::platform::package::RunnerKind;
use crate::platform::semantic_id::{
    BindingId, CaseId, DeclarationId, ExpressionId, FieldId, HttpRouteId, OperationId, ParameterId,
    PortId, RequirementId, TypeParameterId,
};
use crate::platform::session::{CanonicalSessionRead, validate_session_function_type};
use crate::platform::storage::object::ObjectKey;
use std::collections::{BTreeMap, BTreeSet};

struct SignatureGenerics<'a> {
    types: &'a [crate::platform::semantic_id::TypeParameterId],
    effects: &'a [crate::platform::semantic_id::EffectParameterId],
    requirements: &'a [crate::platform::semantic_id::RequirementParameterId],
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
        crate::platform::kernel::contract::MAXIMUM_HTTP_ROUTES_PER_TARGET.saturating_add(1),
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
    work: CompilationWork,
}

#[path = "parallel_lower.rs"]
mod parallel_lower;

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
}

impl<B: CanonicalBaseRead + ?Sized> CodeRead for B {
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
    let mut builder = UnitBuilder {
        canonical: read,
        package: unit.source.package,
        scope: match unit.source.owner {
            OwnerKey::Declaration(declaration) => Some(declaration),
            _ => None,
        },
        tables: TablesBuilder::default(),
        derived_type_objects: BTreeMap::new(),
        work: CompilationWork::default(),
    };
    let mut reconstruct = |code: &CompiledCode| -> Result<(), Diagnostic> {
        for instruction in &code.instructions {
            read.validation_checkpoint()?;
            *remaining = remaining.checked_sub(1).ok_or_else(|| {
                compiler_error(
                    DiagnosticClass::Resource,
                    "compiler_parallel_result_type_limit",
                    "parallel type reconstruction exceeds the existing validation work bound",
                )
            })?;
            let CompiledInstruction::Parallel {
                left,
                left_types,
                right,
                right_types,
                result_type,
                ..
            } = instruction
            else {
                continue;
            };
            let target = |index: u32| {
                unit.tables
                    .declarations
                    .get(index as usize)
                    .copied()
                    .ok_or_else(|| {
                        compiler_corrupt(
                            "compiler_parallel_function",
                            "missing parallel child relocation",
                        )
                    })
            };
            let types = |indexes: &[u32]| {
                indexes
                    .iter()
                    .map(|i| {
                        unit.tables.types.get(*i as usize).copied().ok_or_else(|| {
                            compiler_corrupt(
                                "compiler_parallel_type",
                                "missing child type relocation",
                            )
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()
            };
            let left = builder.parallel_function_result(target(*left)?, &types(left_types)?)?;
            let right = builder.parallel_function_result(target(*right)?, &types(right_types)?)?;
            let expected = builder.parallel_result_type(left, right)?;
            if unit.tables.types.get(*result_type as usize)
                != builder.tables.types.values.get(expected as usize)
            {
                return Err(compiler_corrupt(
                    "compiler_parallel_result_type",
                    "parallel result operand differs from exact canonical child signatures",
                ));
            }
        }
        Ok(())
    };
    match &unit.payload {
        CompilationPayload::Function { code, .. } | CompilationPayload::Constant { code, .. } => {
            reconstruct(code)?
        }
        CompilationPayload::Test {
            actual, expected, ..
        } => {
            reconstruct(actual)?;
            reconstruct(expected)?;
        }
        CompilationPayload::Component { ports, .. } => {
            for port in ports {
                if let CompiledPortImplementation::Expression(code) = &port.implementation {
                    reconstruct(code)?;
                }
            }
        }
        _ => {}
    }
    Ok((builder.derived_type_objects, builder.work.canonical))
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
                self.tables.declaration(i.contract)?;
                self.tables.ty(i.self_type)?;
                for ty in &i.type_arguments {
                    self.tables.ty(*ty)?;
                }
                for m in &i.methods {
                    self.tables.declaration(m.function)?;
                    for ty in &m.type_arguments {
                        self.tables.ty(*ty)?;
                    }
                }
                Ok(CompilationPayload::OwnedImplementation(i))
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
                    },
                    &external.parameters,
                    external.result,
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
                    },
                    &function.parameters,
                    function.result,
                    &function.effect,
                )?;
                signature.implementation_parameters = function.implementation_parameters;
                signature.result_borrow = function.result_borrow;
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
        result: TypeObjectDigest,
        effect: &FunctionEffect,
    ) -> Result<CompiledSignature, Diagnostic> {
        let SignatureGenerics {
            types: type_parameters,
            effects: effect_parameters,
            requirements: requirement_parameters,
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
            result: self.tables.ty(result)?,
            result_borrow: None,
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
    fn implementation_operand(
        &mut self,
        operand: &crate::platform::kernel::ImplementationOperand,
    ) -> Result<(), Diagnostic> {
        let reference = match operand {
            crate::platform::kernel::ImplementationOperand::Concrete {
                implementation,
                type_arguments,
            } => {
                for ty in type_arguments {
                    self.tables.ty(*ty)?;
                }
                *implementation
            }
            crate::platform::kernel::ImplementationOperand::Parameter { function, .. } => *function,
        };
        self.tables.declaration(reference)?;
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
        let read = self.canonical.code_type(ty)?;
        self.work.canonical.add(read.work);
        Ok(match read.value.map(|t| t.form) {
            Some(
                TypeForm::ByteBuffer
                | TypeForm::OwnedI64Cell
                | TypeForm::OwnedProduct { .. }
                | TypeForm::OwnedChoice { .. }
                | TypeForm::OwnedSequence { .. },
            ) => true,
            Some(TypeForm::TypeParameter { parameter }) => matches!(
                self.required_owner(OwnerKey::TypeParameter(parameter), "owned local parameter")?,
                OwnerRecord::TypeParameter(p) if p.constraints.has_owned()
            ),
            _ => false,
        })
    }
    fn compile_code(
        &mut self,
        root: ExpressionId,
        parameters: &[ParameterId],
    ) -> Result<CompiledCode, Diagnostic> {
        let mut compiler = CodeCompiler::new(self, parameters)?;
        let borrowed = match compiler.unit.scope {
            Some(declaration) => matches!(
                compiler.unit.required_owner(OwnerKey::Declaration(declaration), "compiled return owner")?,
                OwnerRecord::Declaration(record) if matches!(&record.payload,
                    DeclarationPayload::Function(function) if function.result_borrow.is_some())
            ),
            None => false,
        };
        compiler.expression_with_use(
            root,
            0,
            if borrowed {
                ParameterUse::Borrow
            } else {
                ParameterUse::Unrestricted
            },
        )?;
        compiler.push(if borrowed {
            CompiledInstruction::ReturnBorrowed
        } else {
            CompiledInstruction::Return
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

    fn function_borrow_position(
        &mut self,
        function: DeclarationReference,
    ) -> Result<u32, Diagnostic> {
        let (parameters, source) = if function.package == self.package {
            match self.required_owner(
                OwnerKey::Declaration(function.declaration),
                "borrowed call function",
            )? {
                OwnerRecord::Declaration(record) => match record.payload {
                    DeclarationPayload::Function(signature) => {
                        (signature.parameters, signature.result_borrow)
                    }
                    _ => {
                        return Err(compiler_corrupt(
                            "compiler_borrow_call",
                            "borrowed calls require graph functions",
                        ));
                    }
                },
                _ => {
                    return Err(compiler_corrupt(
                        "compiler_borrow_call",
                        "missing borrowed call declaration",
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
                    ) => (signature.parameters, signature.result_borrow),
                    _ => {
                        return Err(compiler_corrupt(
                            "compiler_borrow_call",
                            "borrowed calls require graph functions",
                        ));
                    }
                },
                _ => {
                    return Err(compiler_corrupt(
                        "compiler_borrow_call",
                        "missing borrowed call declaration",
                    ));
                }
            }
        };
        let position = source
            .and_then(|source| parameters.iter().position(|p| *p == source))
            .ok_or_else(|| {
                compiler_corrupt(
                    "compiler_borrow_call",
                    "borrowed call has no exact selected parameter",
                )
            })?;
        u32_count("borrowed result source position", position)
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
                            && signature.effect_parameters.is_empty() =>
                    {
                        (signature.parameters, signature.result)
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
                        && signature.effect_parameters.is_empty() =>
                    {
                        (signature.parameters, signature.result)
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
    ) -> Result<u32, Diagnostic> {
        let mut owned = false;
        for ty in [left, right] {
            self.canonical.code_step()?;
            let object = self.parallel_type(ty)?;
            owned |= match object.form {
                TypeForm::ByteBuffer
                | TypeForm::OwnedI64Cell
                | TypeForm::OwnedProduct { .. }
                | TypeForm::OwnedChoice { .. }
                | TypeForm::OwnedSequence { .. } => true,
                TypeForm::TypeParameter { parameter } => {
                    let OwnerRecord::TypeParameter(record) = self.required_owner(
                        OwnerKey::TypeParameter(parameter),
                        "parallel result type parameter",
                    )?
                    else {
                        return Err(compiler_corrupt(
                            "compiler_parallel_result_type",
                            "parallel result parameter has another canonical owner kind",
                        ));
                    };
                    if Some(record.declaration) != self.scope {
                        return Err(compiler_corrupt(
                            "compiler_parallel_result_type",
                            "parallel result parameter is outside its exact caller scope",
                        ));
                    }
                    record.constraints.has_owned()
                }
                _ => false,
            };
        }
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
        let (digest, bytes) = encode_type_object(&object)?;
        self.derived_type_objects.insert(
            ObjectKey::from_digest(
                crate::platform::storage::object::ObjectDomain::Type,
                digest.bytes(),
            ),
            bytes,
        );
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
}

impl<'a, 'b, B: CodeRead + ?Sized> CodeCompiler<'a, 'b, B> {
    fn new(
        unit: &'a mut UnitBuilder<'b, B>,
        parameters: &[ParameterId],
    ) -> Result<Self, Diagnostic> {
        let mut compiler = Self {
            unit,
            instructions: Vec::new(),
            locals: BTreeMap::new(),
            next_local: 0,
            active: BTreeSet::new(),
            compiled: BTreeSet::new(),
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

    fn expression_with_use(
        &mut self,
        expression: ExpressionId,
        depth: usize,
        use_mode: ParameterUse,
    ) -> Result<(), Diagnostic> {
        let operation = self.begin_expression(expression, depth)?;
        let result = self.operation(operation, depth + 1, use_mode);
        self.active.remove(&expression);
        result
    }

    fn begin_expression(
        &mut self,
        expression: ExpressionId,
        depth: usize,
    ) -> Result<ExpressionOperation, Diagnostic> {
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
        let OwnerRecord::Expression(ExpressionRecord { id, operation, .. }) = record else {
            self.active.remove(&expression);
            return Err(compiler_corrupt(
                "compiler_expression_kind",
                "expression identity names another canonical owner kind",
            ));
        };
        if id != expression {
            self.active.remove(&expression);
            return Err(compiler_corrupt(
                "compiler_expression_identity",
                "expression record identity changed during exact lowering",
            ));
        }
        Ok(operation)
    }

    /// Traverse the canonical child Call exactly once, but evaluate only its
    /// arguments in the parent. The task body belongs to the joined child.
    fn parallel_call(
        &mut self,
        expression: ExpressionId,
        depth: usize,
    ) -> Result<parallel_lower::Child, Diagnostic> {
        let operation = self.begin_expression(expression, depth)?;
        let result = (|| {
            let (function, arguments, type_arguments, implementations) = match operation {
                ExpressionOperation::Call {
                    function,
                    arguments,
                    type_arguments,
                    effect_arguments,
                    requirement_arguments,
                } if effect_arguments.is_empty() && requirement_arguments.is_empty() => {
                    (function, arguments, type_arguments, Vec::new())
                }
                ExpressionOperation::ImplementationCall {
                    function,
                    arguments,
                    type_arguments,
                    implementations,
                    effect_arguments,
                    requirement_arguments,
                } if effect_arguments.is_empty() && requirement_arguments.is_empty() => {
                    (function, arguments, type_arguments, implementations)
                }
                _ => {
                    return Err(compiler_corrupt(
                        "compiler_parallel_call",
                        "parallel child must be a closed named call",
                    ));
                }
            };
            let uses = self.unit.function_parameter_uses(function)?;
            let result = self
                .unit
                .parallel_function_result(function, &type_arguments)?;
            let types = type_arguments
                .into_iter()
                .map(|ty| self.unit.tables.ty(ty))
                .collect::<Result<Vec<_>, _>>()?;
            for operand in &implementations {
                self.unit.implementation_operand(operand)?;
            }
            if uses.len() != arguments.len() {
                return Err(compiler_corrupt(
                    "compiler_parallel_call",
                    "parallel child arity differs from its exact signature",
                ));
            }
            let function = self.unit.tables.declaration(function)?;
            let count = u32_count("parallel child arguments", arguments.len())?;
            for (argument, use_mode) in arguments.into_iter().zip(uses) {
                self.expression_with_use(argument, depth + 1, use_mode)?;
            }
            Ok(parallel_lower::Child {
                function,
                arguments: count,
                result,
                types,
                implementations,
            })
        })();
        self.active.remove(&expression);
        result
    }

    fn operation(
        &mut self,
        operation: ExpressionOperation,
        depth: usize,
        use_mode: ParameterUse,
    ) -> Result<(), Diagnostic> {
        match operation {
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
                // The source graph admits exact locals. Preserve element-before-sequence order.
                let value_local = self.borrow_source_local(value, item, depth)?;
                let source_local = self.borrow_source_local(source, sequence_type, depth)?;
                let sequence_type = self.unit.tables.ty(sequence_type)?;
                self.push(CompiledInstruction::SequencePush {
                    sequence_type,
                    value_local,
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
                let (source_position, arguments) = match operation {
                    ExpressionOperation::Call {
                        function,
                        arguments,
                        ..
                    }
                    | ExpressionOperation::ImplementationCall {
                        function,
                        arguments,
                        ..
                    } => (self.unit.function_borrow_position(function)?, arguments),
                    ExpressionOperation::MethodCall {
                        contract,
                        method,
                        arguments,
                        ..
                    } => {
                        let signature = self.unit.owned_method(contract, method)?;
                        (
                            signature.result_borrow.ok_or_else(|| {
                                compiler_corrupt(
                                    "compiler_borrow_call",
                                    "method has no borrowed result",
                                )
                            })?,
                            arguments,
                        )
                    }
                    _ => {
                        return Err(compiler_corrupt(
                            "compiler_borrow_call",
                            "borrowed invocation requires an exact call",
                        ));
                    }
                };
                let source = *arguments.get(source_position as usize).ok_or_else(|| {
                    compiler_corrupt("compiler_borrow_call", "selected source argument is absent")
                })?;
                let value = match self
                    .unit
                    .required_owner(OwnerKey::Expression(source), "borrowed source argument")?
                {
                    OwnerRecord::Expression(record) => match record.operation {
                        ExpressionOperation::Local { value } => value,
                        _ => {
                            return Err(compiler_corrupt(
                                "compiler_borrow_source",
                                "borrowed result source must be a local",
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
                let source_local = self.locals.get(&value).copied().ok_or_else(|| {
                    compiler_corrupt(
                        "compiler_borrow_source",
                        "borrowed result source is outside scope",
                    )
                })?;
                let record = self.binding(binding, BindingKind::OwnedBorrow)?;
                let ty = record.declared_type.ok_or_else(|| {
                    compiler_corrupt(
                        "compiler_borrow_binding",
                        "borrowed call binding requires an exact type",
                    )
                })?;
                // Preserve complete authored argument evaluation before entering the handoff.
                self.expression(call, depth)?;
                let invocation = self.instructions.pop().ok_or_else(|| {
                    compiler_corrupt("compiler_borrow_call", "missing call instruction")
                })?;
                if !matches!(
                    invocation,
                    CompiledInstruction::Call { .. }
                        | CompiledInstruction::ImplementationCall { .. }
                        | CompiledInstruction::MethodCall { .. }
                ) {
                    return Err(compiler_corrupt(
                        "compiler_borrow_call",
                        "borrowed invocation has no exact call terminal",
                    ));
                }
                let (reference, binding_local, binding_type) = self.borrow_binding(binding, ty)?;
                self.push(CompiledInstruction::BeginBorrowCall {
                    source_local,
                    source_position,
                })?;
                self.push(invocation)?;
                self.push(CompiledInstruction::AdoptBorrowResult {
                    source_local,
                    binding_local,
                    binding_type,
                })?;
                self.expression_with_use(body, depth, use_mode)?;
                self.push(CompiledInstruction::EndOwnedBorrow { binding_local })?;
                self.locals.remove(&reference);
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
                self.expression_with_use(body, depth, use_mode)?;
                self.push(CompiledInstruction::EndOwnedBorrow { binding_local })?;
                self.locals.remove(&reference);
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
                self.expression_with_use(body, depth, use_mode)?;
                self.push(CompiledInstruction::EndOwnedBorrow { binding_local })?;
                self.locals.remove(&reference);
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
                    self.expression_with_use(arm.body, depth, use_mode)?;
                    self.push(CompiledInstruction::EndOwnedBorrow { binding_local })?;
                    self.locals.remove(&reference);
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
                let left = self.parallel_call(left, depth)?;
                let right = self.parallel_call(right, depth)?;
                let result_type = self.unit.parallel_result_type(left.result, right.result)?;
                self.push(CompiledInstruction::Parallel {
                    left: left.function,
                    left_arguments: left.arguments,
                    left_types: left.types,
                    left_implementations: left.implementations,
                    right: right.function,
                    right_arguments: right.arguments,
                    right_types: right.types,
                    right_implementations: right.implementations,
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
                    self.locals.remove(&reference);
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
                let mut scoped = Vec::with_capacity(bindings.len());
                for binding in bindings {
                    let record = self.binding(binding, BindingKind::Let)?;
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
                }
                self.expression_with_use(body, depth, use_mode)?;
                for reference in &scoped {
                    let LocalValueReference::LexicalBinding(binding) = reference else {
                        continue;
                    };
                    let record = self.binding(*binding, BindingKind::Let)?;
                    if let Some(ty) = record.declared_type
                        && self.unit.owned_local_type(ty)?
                    {
                        let local = self.locals[reference];
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
            } => {
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
                })?;
            }
            ExpressionOperation::MethodCall {
                witness,
                contract,
                method,
                arguments,
            } => {
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
                })?;
            }
            ExpressionOperation::Call {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
                arguments,
            } => {
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
                })?;
            }
            ExpressionOperation::FunctionValue {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
            } => {
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
                self.locals.remove(&reference);
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
                self.locals.remove(&reference);
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

    /// Traverse the exact canonical source child while preserving custody in its local slot.
    fn borrow_source_local(
        &mut self,
        expression: ExpressionId,
        expected: TypeObjectDigest,
        depth: usize,
    ) -> Result<u32, Diagnostic> {
        let operation = self.begin_expression(expression, depth)?;
        let result = (|| {
            let ExpressionOperation::Local { value } = operation else {
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
        let record = self.binding(binding, BindingKind::OwnedBorrow)?;
        if record.value.is_some() || record.declared_type != Some(expected) {
            return Err(compiler_corrupt(
                "compiler_borrow_binding",
                "scoped read binding requires the exact child annotation and no initializer",
            ));
        }
        let reference = LocalValueReference::LexicalBinding(binding);
        let local = self.bind(reference)?;
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
            self.locals.remove(reference);
        }
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
