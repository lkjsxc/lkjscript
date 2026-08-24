//! Exact point-read lowering from normalized Graph 5 records into one compiler unit.

use super::unit::{
    BYTECODE_CONTRACT_VERSION, COMPILER_UNIT_CONTRACT_VERSION, CompilationPayload,
    CompilationSource, CompilationTables, CompilationUnit, CompilationUnitKey, CompiledCaseLayout,
    CompiledCode, CompiledFieldLayout, CompiledFieldSelector, CompiledInstruction,
    CompiledOperationLayout, CompiledParameter, CompiledPort, CompiledPortImplementation,
    CompiledRequirement, CompiledSignature, CompiledText, CompiledVariantJump,
    MAXIMUM_COMPILER_UNIT_ITEMS, OptimizationPolicy,
};
use crate::platform::change::{
    CanonicalBaseRead, CanonicalReadWork, WitnessBaseRead, WitnessReadWork,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    BindingKind, BindingRecord, CaseReference, DeclarationPayload, DeclarationRecord,
    DeclarationReference, ExpressionOperation, ExpressionRecord, FieldReference, FieldSelector,
    FunctionEffect, LocalValueReference, OperationReference, OwnerKey, OwnerRecord, PackageId,
    ParameterParent, PortImplementation, PortReference, RequirementReference, TextValue,
    TypeObjectDigest,
};
use crate::platform::semantic_id::{
    BindingId, CaseId, DeclarationId, ExpressionId, FieldId, OperationId, ParameterId, PortId,
    RequirementId,
};
use crate::platform::storage::object::ObjectKey;
use std::collections::{BTreeMap, BTreeSet};

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
        tables: TablesBuilder::default(),
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
    let payload = builder.compile_payload(owner, record)?;
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

struct UnitBuilder<'a, B: ?Sized> {
    canonical: &'a B,
    package: PackageId,
    tables: TablesBuilder,
    work: CompilationWork,
}

impl<B: CanonicalBaseRead + ?Sized> UnitBuilder<'_, B> {
    fn compile_payload(
        &mut self,
        selected: OwnerKey,
        record: OwnerRecord,
    ) -> Result<CompilationPayload, Diagnostic> {
        match (selected, record) {
            (OwnerKey::Declaration(declaration), OwnerRecord::Declaration(record)) => {
                self.compile_declaration(declaration, record)
            }
            (OwnerKey::Target(_), OwnerRecord::Target(record)) => {
                let component = self.tables.declaration(record.component)?;
                let port = self.tables.port(record.port)?;
                Ok(CompilationPayload::Target {
                    component,
                    port,
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
            DeclarationPayload::Record { fields } => {
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
                Ok(CompilationPayload::Record { fields: compiled })
            }
            DeclarationPayload::Variant { cases } => {
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
                Ok(CompilationPayload::Variant { cases: compiled })
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
                    &external.type_parameters,
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
                let signature = self.compile_signature(
                    declaration,
                    &function.type_parameters,
                    &function.parameters,
                    function.result,
                    &function.effect,
                )?;
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

    fn compile_signature(
        &mut self,
        declaration: DeclarationId,
        type_parameters: &[crate::platform::semantic_id::TypeParameterId],
        parameters: &[ParameterId],
        result: TypeObjectDigest,
        effect: &FunctionEffect,
    ) -> Result<CompiledSignature, Diagnostic> {
        for type_parameter in type_parameters {
            match self.required_owner(
                OwnerKey::TypeParameter(*type_parameter),
                "function signature references a missing type parameter",
            )? {
                OwnerRecord::TypeParameter(record) if record.declaration == declaration => {}
                OwnerRecord::TypeParameter(_) => {
                    return Err(compiler_corrupt(
                        "compiler_unit_type_parameter_parent",
                        "function type parameter belongs to another declaration",
                    ));
                }
                _ => {
                    return Err(compiler_corrupt(
                        "compiler_unit_type_parameter_kind",
                        "function type-parameter identity names another owner kind",
                    ));
                }
            }
        }
        let mut compiled_parameters = Vec::with_capacity(parameters.len());
        for parameter in parameters {
            let parameter_record = self.required_parameter(
                *parameter,
                ParameterParent::Function(declaration),
                "function parameter",
            )?;
            compiled_parameters.push(CompiledParameter {
                parameter: *parameter,
                ty: self.tables.ty(parameter_record.ty)?,
            });
        }
        let task_requirements = match effect {
            FunctionEffect::Pure => Vec::new(),
            FunctionEffect::Task { requirements } => requirements
                .iter()
                .map(|reference| self.tables.requirement(*reference))
                .collect::<Result<Vec<_>, _>>()?,
        };
        Ok(CompiledSignature {
            type_parameters: type_parameters.to_vec(),
            parameters: compiled_parameters,
            result: self.tables.ty(result)?,
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
                ty: self.tables.ty(parameter_record.ty)?,
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

    fn compile_code(
        &mut self,
        root: ExpressionId,
        parameters: &[ParameterId],
    ) -> Result<CompiledCode, Diagnostic> {
        let mut compiler = CodeCompiler::new(self, parameters)?;
        compiler.expression(root, 0)?;
        compiler.push(CompiledInstruction::Return)?;
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
        let read = self.canonical.read_owner(owner)?;
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
}

struct CodeCompiler<'a, 'b, B: ?Sized> {
    unit: &'a mut UnitBuilder<'b, B>,
    instructions: Vec<CompiledInstruction>,
    locals: BTreeMap<LocalValueReference, u32>,
    next_local: u32,
    active: BTreeSet<ExpressionId>,
    compiled: BTreeSet<ExpressionId>,
}

impl<'a, 'b, B: CanonicalBaseRead + ?Sized> CodeCompiler<'a, 'b, B> {
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
        let result = self.operation(operation, depth + 1);
        self.active.remove(&expression);
        result
    }

    fn operation(
        &mut self,
        operation: ExpressionOperation,
        depth: usize,
    ) -> Result<(), Diagnostic> {
        match operation {
            ExpressionOperation::Unit {} => {
                self.push(CompiledInstruction::Unit)?;
            }
            ExpressionOperation::Bool { value } => {
                self.push(CompiledInstruction::Bool(value))?;
            }
            ExpressionOperation::I64 { value } => {
                self.push(CompiledInstruction::I64(value))?;
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
                self.push(CompiledInstruction::LoadLocal(local))?;
            }
            ExpressionOperation::Constant { declaration } => {
                let function = self.unit.tables.declaration(declaration)?;
                self.push(CompiledInstruction::Call {
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
                self.expression(when_true, depth)?;
                let jump = self.push(CompiledInstruction::Jump(u32::MAX))?;
                let false_target = self.next_instruction()?;
                self.expression(when_false, depth)?;
                let end = self.next_instruction()?;
                self.instructions[conditional as usize] =
                    CompiledInstruction::JumpIfFalse(false_target);
                self.instructions[jump as usize] = CompiledInstruction::Jump(end);
            }
            ExpressionOperation::Let { bindings, body } => {
                let mut scoped = Vec::with_capacity(bindings.len());
                for binding in bindings {
                    let record = self.binding(binding, BindingKind::Let)?;
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
                self.expression(body, depth)?;
                self.unbind_all(&scoped);
            }
            ExpressionOperation::Sequence { items } => {
                let count = items.len();
                for (index, item) in items.into_iter().enumerate() {
                    self.expression(item, depth)?;
                    if index + 1 != count {
                        self.push(CompiledInstruction::Drop)?;
                    }
                }
            }
            ExpressionOperation::Call {
                function,
                type_arguments,
                arguments,
            } => {
                let function = self.unit.tables.declaration(function)?;
                let type_arguments = type_arguments
                    .into_iter()
                    .map(|ty| self.unit.tables.ty(ty))
                    .collect::<Result<Vec<_>, _>>()?;
                let argument_count = u32_count("call arguments", arguments.len())?;
                for argument in arguments {
                    self.expression(argument, depth)?;
                }
                self.push(CompiledInstruction::Call {
                    function,
                    type_arguments,
                    arguments: argument_count,
                })?;
            }
            ExpressionOperation::FunctionValue {
                function,
                type_arguments,
            } => {
                let function = self.unit.tables.declaration(function)?;
                let type_arguments = type_arguments
                    .into_iter()
                    .map(|ty| self.unit.tables.ty(ty))
                    .collect::<Result<Vec<_>, _>>()?;
                self.push(CompiledInstruction::FunctionValue {
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
            ExpressionOperation::Record {
                nominal_type,
                fields,
            } => {
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
                    fields: selectors,
                })?;
            }
            ExpressionOperation::Variant { case, payload } => {
                let case = self.unit.tables.case(case)?;
                if let Some(payload) = payload {
                    self.expression(payload, depth)?;
                }
                self.push(CompiledInstruction::Variant {
                    case,
                    has_payload: payload.is_some(),
                })?;
            }
            ExpressionOperation::Field { value, selector } => {
                let selector = self.field_selector(selector)?;
                self.expression(value, depth)?;
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
                self.expression(value, depth)?;
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
                    self.expression(arm.body, depth)?;
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
                let requirement = self.unit.tables.requirement(requirement)?;
                let operation = self.unit.tables.operation(operation)?;
                let argument_count = u32_count("capability arguments", arguments.len())?;
                for argument in arguments {
                    self.expression(argument, depth)?;
                }
                self.push(CompiledInstruction::Perform {
                    requirement,
                    operation,
                    arguments: argument_count,
                })?;
            }
            ExpressionOperation::Transaction {
                requirement,
                binding,
                body,
            } => {
                self.binding(binding, BindingKind::Transaction)?;
                let requirement = self.unit.tables.requirement(requirement)?;
                let reference = LocalValueReference::TransactionBinding(binding);
                let local = self.bind(reference)?;
                self.push(CompiledInstruction::BeginTransaction {
                    requirement,
                    binding: local,
                })?;
                self.expression(body, depth)?;
                self.push(CompiledInstruction::CommitTransaction {
                    requirement,
                    binding: local,
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
