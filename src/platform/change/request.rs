//! High-level semantic intent lowered to exact primitive owner edits.

mod codec;
mod creation;
mod deletion;
mod extraction;
mod precondition;

pub use creation::{
    AuthoredAnnotationValue, AuthoredBindingDefinition, AuthoredCase, AuthoredCaseReference,
    AuthoredDeclarationReference, AuthoredExpression, AuthoredExpressionOperation, AuthoredField,
    AuthoredFieldReference, AuthoredFieldSelector, AuthoredFunctionEffect, AuthoredLetBinding,
    AuthoredLocalReference, AuthoredMapExpressionEntry, AuthoredMatchExpressionArm,
    AuthoredOperation, AuthoredOperationReference, AuthoredParameter, AuthoredPort,
    AuthoredPortImplementation, AuthoredPortReference, AuthoredRecordExpressionField,
    AuthoredRequirement, AuthoredRequirementReference, AuthoredResourceLimit,
    AuthoredStructuralTypeField, AuthoredType, AuthoredTypeParameter,
    AuthoredTypeParameterReference,
};
pub use precondition::{AuthoredOwnerParent, AuthoredPrecondition};

use super::{
    AuthoredAllocation, CanonicalBaseRead, CanonicalReadWork, ChangeBudget, ChangeBudgetWork,
    PrimitiveEdit, WitnessBaseRead, WitnessReadWork,
};
use crate::platform::contract::registry::CHANGE_ALLOCATION_SEED_DOMAIN;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    ChangeDigest, DependencyObjectDigest, DependencyRecord, EncodedOwnerKey, IdentityKind,
    ModuleRecord, Name, NamespaceClass, OwnerHeader, OwnerKey, OwnerKind, OwnerRecord, PackageId,
    PackageRevisionDigest, RetirementRecord, TypeObject, TypeObjectDigest, TypeObjectInterner,
    encode_dependency, encode_owner,
};
use crate::platform::semantic_id::{
    AnnotationId, BindingId, CaseId, DeclarationId, DocumentationId, ExpressionId, FieldId,
    HttpRouteId, ModuleId, OperationId, ParameterId, PortId, RequirementId, RevisionId, TargetId,
    TypeParameterId,
};
use crate::platform::witness::NamespaceKey;
use std::collections::{BTreeMap, BTreeSet};

pub const MAXIMUM_AUTHORED_CHANGES: usize = 10_000;
pub const MAXIMUM_AUTHORED_CHANGE_BYTES: usize = 4 * 1_048_576;
const MAXIMUM_REQUEST_SYMBOL_BYTES: usize = 128;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoredChangeSet {
    pub base: RevisionId,
    pub preconditions: Vec<AuthoredPrecondition>,
    pub changes: Vec<AuthoredChange>,
    pub budget: ChangeBudget,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthoredChange {
    CreateModule {
        symbol: String,
        name: Name,
    },
    CreateFunction {
        symbol: String,
        module: ModuleSelector,
        name: Name,
        visibility: crate::platform::kernel::DeclarationVisibility,
        type_parameters: Vec<AuthoredTypeParameter>,
        parameters: Vec<AuthoredParameter>,
        result: AuthoredType,
        effect: AuthoredFunctionEffect,
        body: AuthoredExpression,
    },
    CreateRecord {
        symbol: String,
        module: ModuleSelector,
        name: Name,
        visibility: crate::platform::kernel::DeclarationVisibility,
        fields: Vec<AuthoredField>,
    },
    CreateVariant {
        symbol: String,
        module: ModuleSelector,
        name: Name,
        visibility: crate::platform::kernel::DeclarationVisibility,
        cases: Vec<AuthoredCase>,
    },
    CreateInterface {
        symbol: String,
        module: ModuleSelector,
        name: Name,
        visibility: crate::platform::kernel::DeclarationVisibility,
        operations: Vec<AuthoredOperation>,
    },
    CreateExternal {
        symbol: String,
        module: ModuleSelector,
        name: Name,
        visibility: crate::platform::kernel::DeclarationVisibility,
        type_parameters: Vec<AuthoredTypeParameter>,
        parameters: Vec<AuthoredParameter>,
        result: AuthoredType,
        implementation: crate::platform::kernel::ImplementationName,
    },
    CreateConstant {
        symbol: String,
        module: ModuleSelector,
        name: Name,
        visibility: crate::platform::kernel::DeclarationVisibility,
        ty: AuthoredType,
        value: AuthoredExpression,
    },
    CreateComponent {
        symbol: String,
        module: ModuleSelector,
        name: Name,
        visibility: crate::platform::kernel::DeclarationVisibility,
        requirements: Vec<AuthoredRequirement>,
        ports: Vec<AuthoredPort>,
    },
    CreateTest {
        symbol: String,
        module: ModuleSelector,
        name: Name,
        visibility: crate::platform::kernel::DeclarationVisibility,
        actual: AuthoredExpression,
        expected: AuthoredExpression,
    },
    CreateTarget {
        symbol: String,
        name: Name,
        component: AuthoredDeclarationReference,
        port: Option<AuthoredPortReference>,
        runner: crate::platform::package::RunnerKind,
    },
    CreateDocumentation {
        symbol: String,
        owner: OwnerSelector,
        class: crate::platform::kernel::DocumentationClass,
        text: String,
    },
    CreateAnnotation {
        symbol: String,
        owner: OwnerSelector,
        class: crate::platform::kernel::AnnotationClass,
        key: Name,
        value: AuthoredAnnotationValue,
    },
    AddField {
        record: DeclarationSelector,
        field: AuthoredField,
    },
    AddCase {
        variant: DeclarationSelector,
        case: AuthoredCase,
    },
    AddOperation {
        interface: DeclarationSelector,
        operation: AuthoredOperation,
    },
    AddTypeParameter {
        declaration: DeclarationSelector,
        parameter: AuthoredTypeParameter,
    },
    AddParameter {
        parent: ParameterParentSelector,
        parameter: AuthoredParameter,
    },
    AddRequirement {
        component: DeclarationSelector,
        requirement: AuthoredRequirement,
    },
    AddPort {
        component: DeclarationSelector,
        port: AuthoredPort,
    },
    AddHttpRoute {
        symbol: String,
        target: OwnerSelector,
        method: String,
        path: String,
        port: AuthoredPortReference,
    },
    SetDeclarationVisibility {
        declaration: DeclarationSelector,
        visibility: crate::platform::kernel::DeclarationVisibility,
    },
    SetFunctionContract {
        function: DeclarationSelector,
        result: AuthoredType,
        effect: AuthoredFunctionEffect,
    },
    SetExternalContract {
        external: DeclarationSelector,
        result: AuthoredType,
        implementation: crate::platform::kernel::ImplementationName,
    },
    SetFieldType {
        field: OwnerSelector,
        ty: AuthoredType,
    },
    SetCasePayload {
        case: OwnerSelector,
        payload: Option<AuthoredType>,
    },
    SetParameterType {
        parameter: OwnerSelector,
        ty: AuthoredType,
    },
    SetOperationContract {
        operation: OwnerSelector,
        result: AuthoredType,
        idempotency: crate::platform::kernel::Idempotency,
        external_visibility: crate::platform::kernel::ExternalVisibility,
    },
    SetRequirementContract {
        requirement: OwnerSelector,
        interface: AuthoredDeclarationReference,
        operations: Vec<AuthoredOperationReference>,
        limits: Vec<AuthoredResourceLimit>,
    },
    SetTarget {
        target: OwnerSelector,
        component: AuthoredDeclarationReference,
        port: Option<AuthoredPortReference>,
        runner: crate::platform::package::RunnerKind,
    },
    SetHttpRoute {
        route: OwnerSelector,
        method: String,
        path: String,
        port: AuthoredPortReference,
    },
    AddDependency {
        package: PackageId,
        semantic_revision: RevisionId,
        package_revision: PackageRevisionDigest,
    },
    ReplaceDependency {
        package: PackageId,
        semantic_revision: RevisionId,
        package_revision: PackageRevisionDigest,
    },
    DeleteDependency {
        package: PackageId,
    },
    DeleteOwner {
        owner: OwnerSelector,
        policy: AuthoredDeletePolicy,
    },
    RenameOwner {
        owner: OwnerSelector,
        name: Name,
    },
    MoveDeclaration {
        declaration: DeclarationSelector,
        module: ModuleSelector,
    },
    ReplaceFunctionBody {
        function: DeclarationSelector,
        body: AuthoredExpression,
    },
    ExtractFunction {
        symbol: String,
        function: DeclarationSelector,
        expression: ExpressionId,
        name: Name,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OwnerSelector {
    Exact { owner: OwnerKey },
    ModuleName { name: Name },
    DeclarationName { module: ModuleSelector, name: Name },
    Symbol { symbol: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModuleSelector {
    Id { module: ModuleId },
    Name { name: Name },
    Symbol { symbol: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeclarationSelector {
    Id {
        declaration: crate::platform::semantic_id::DeclarationId,
    },
    Qualified {
        module: ModuleSelector,
        name: Name,
    },
    Symbol {
        symbol: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParameterParentSelector {
    Declaration { declaration: DeclarationSelector },
    Operation { operation: OwnerSelector },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthoredDeletePolicy {
    Reject,
    OwnedClosure,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum SymbolKind {
    Module,
    Declaration,
    TypeParameter,
    Field,
    Case,
    Operation,
    FunctionParameter,
    OperationParameter,
    LexicalBinding,
    MatchPayloadBinding,
    TransactionBinding,
    Expression,
    Requirement,
    Port,
    Target,
    HttpRoute,
    Documentation,
    Annotation,
}

impl SymbolKind {
    const fn canonical_tag(self) -> u8 {
        match self {
            Self::Module => 1,
            Self::Declaration => 2,
            Self::TypeParameter => 3,
            Self::Field => 4,
            Self::Case => 5,
            Self::Operation => 6,
            Self::FunctionParameter => 7,
            Self::OperationParameter => 8,
            Self::LexicalBinding => 9,
            Self::MatchPayloadBinding => 10,
            Self::TransactionBinding => 11,
            Self::Expression => 12,
            Self::Requirement => 13,
            Self::Port => 14,
            Self::Target => 15,
            Self::HttpRoute => 18,
            Self::Documentation => 16,
            Self::Annotation => 17,
        }
    }

    const fn allocation_domain(self) -> u8 {
        match self {
            Self::Module => 1,
            Self::Declaration => 2,
            Self::TypeParameter => 3,
            Self::Field => 4,
            Self::Case => 5,
            Self::Operation => 6,
            Self::FunctionParameter | Self::OperationParameter => 7,
            Self::LexicalBinding | Self::MatchPayloadBinding | Self::TransactionBinding => 8,
            Self::Expression => 9,
            Self::Requirement => 10,
            Self::Port => 11,
            Self::Target => 12,
            Self::HttpRoute => 15,
            Self::Documentation => 13,
            Self::Annotation => 14,
        }
    }

    const fn identity_kind(self) -> IdentityKind {
        match self {
            Self::Module => IdentityKind::Module,
            Self::Declaration => IdentityKind::Declaration,
            Self::TypeParameter => IdentityKind::TypeParameter,
            Self::Field => IdentityKind::Field,
            Self::Case => IdentityKind::Case,
            Self::Operation => IdentityKind::Operation,
            Self::FunctionParameter | Self::OperationParameter => IdentityKind::Parameter,
            Self::LexicalBinding | Self::MatchPayloadBinding | Self::TransactionBinding => {
                IdentityKind::Binding
            }
            Self::Expression => IdentityKind::Expression,
            Self::Requirement => IdentityKind::Requirement,
            Self::Port => IdentityKind::Port,
            Self::Target => IdentityKind::Target,
            Self::HttpRoute => IdentityKind::HttpRoute,
            Self::Documentation => IdentityKind::Documentation,
            Self::Annotation => IdentityKind::Annotation,
        }
    }

    fn allocate(self, seed: &[u8], ordinal: u64) -> OwnerKey {
        match self {
            Self::Module => OwnerKey::Module(ModuleId::allocate(seed, ordinal)),
            Self::Declaration => OwnerKey::Declaration(DeclarationId::allocate(seed, ordinal)),
            Self::TypeParameter => {
                OwnerKey::TypeParameter(TypeParameterId::allocate(seed, ordinal))
            }
            Self::Field => OwnerKey::Field(FieldId::allocate(seed, ordinal)),
            Self::Case => OwnerKey::Case(CaseId::allocate(seed, ordinal)),
            Self::Operation => OwnerKey::Operation(OperationId::allocate(seed, ordinal)),
            Self::FunctionParameter | Self::OperationParameter => {
                OwnerKey::Parameter(ParameterId::allocate(seed, ordinal))
            }
            Self::LexicalBinding | Self::MatchPayloadBinding | Self::TransactionBinding => {
                OwnerKey::Binding(BindingId::allocate(seed, ordinal))
            }
            Self::Expression => OwnerKey::Expression(ExpressionId::allocate(seed, ordinal)),
            Self::Requirement => OwnerKey::Requirement(RequirementId::allocate(seed, ordinal)),
            Self::Port => OwnerKey::Port(PortId::allocate(seed, ordinal)),
            Self::Target => OwnerKey::Target(TargetId::allocate(seed, ordinal)),
            Self::HttpRoute => OwnerKey::HttpRoute(
                crate::platform::semantic_id::HttpRouteId::allocate(seed, ordinal),
            ),
            Self::Documentation => {
                OwnerKey::Documentation(DocumentationId::allocate(seed, ordinal))
            }
            Self::Annotation => OwnerKey::Annotation(AnnotationId::allocate(seed, ordinal)),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AuthoredLoweringWork {
    pub operations_lowered: u64,
    pub preconditions_checked: u64,
    pub allocated_identities: u64,
    pub type_nodes_interned: u64,
    pub ownership_steps: u64,
    pub relation_edges_read: u64,
    pub canonical: CanonicalReadWork,
    pub witness: WitnessReadWork,
}

impl AuthoredLoweringWork {
    pub(crate) fn budget_work(self) -> ChangeBudgetWork {
        ChangeBudgetWork {
            authored_operations: self.operations_lowered,
            preconditions_checked: self.preconditions_checked,
            allocated_identities: self.allocated_identities,
            authored_type_nodes: self.type_nodes_interned,
            canonical_reads: self.canonical,
            witness_reads: self.witness,
            impact_ownership_steps: self.ownership_steps,
            relation_edges: self.relation_edges_read,
            ..ChangeBudgetWork::default()
        }
    }
}

#[derive(Clone, Debug)]
pub struct AuthoredLowering {
    pub edits: Vec<PrimitiveEdit>,
    pub allocated: BTreeMap<String, OwnerKey>,
    pub allocations: Vec<AuthoredAllocation>,
    pub dependency_befores: BTreeMap<PackageId, DependencyRecord>,
    pub extraction: Option<super::FunctionExtractionEvidence>,
    pub work: AuthoredLoweringWork,
}

pub fn lower_authored_changes<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    base: &B,
    witness: &W,
    request: &AuthoredChangeSet,
) -> Result<AuthoredLowering, Diagnostic> {
    if base.exact_revision() != Some(request.base) {
        return Err(request_error(
            DiagnosticClass::Semantic,
            "change_authored_stale_base",
            "authored change base is not the exact pinned repository revision",
        ));
    }
    if !witness.witness_contract_is_current()
        || witness.witness_repository_id() != base.repository_id()
        || witness.witness_package_id() != base.package_id()
        || witness.witness_manifest().semantic_root
            != crate::platform::kernel::encode_root(base.semantic_root())?.0
    {
        return Err(request_error(
            DiagnosticClass::Corrupt,
            "change_authored_witness_base",
            "authored change inputs do not share one exact canonical and witness base",
        ));
    }
    if request.changes.is_empty() || request.changes.len() > MAXIMUM_AUTHORED_CHANGES {
        return Err(request_error(
            DiagnosticClass::Resource,
            "change_authored_count",
            format!("authored change requires 1 through {MAXIMUM_AUTHORED_CHANGES} operations"),
        ));
    }
    let budget = request
        .budget
        .validate_request_counts(request.changes.len(), request.preconditions.len())?;

    let (definitions, total_identity_count) =
        collect_symbol_definitions(request, budget.authored.maximum_allocated_identities)?;
    budget.check_allocated_identities(total_identity_count)?;
    let request_bytes = codec::encode_authored_intent(request, &definitions)?;
    let seed = allocation_seed(base, &request_bytes)?;
    let deletion_change = ChangeDigest::of(&request_bytes);
    let definition_count = definitions.len();
    let allocated = allocate_symbols(&seed, &definitions)?;
    let allocations = logical_allocations(&definitions, &allocated)?;
    let mut lowerer = AuthoredLowerer::new(
        base,
        witness,
        AuthoredLoweringInputs {
            allocation_seed: seed,
            deletion_change,
            allocated,
            definitions,
            allocations,
            budget,
        },
    )?;
    lowerer.work.operations_lowered = u64::try_from(request.changes.len()).unwrap_or(u64::MAX);
    lowerer.work.allocated_identities = u64::try_from(definition_count).unwrap_or(u64::MAX);
    precondition::evaluate(&mut lowerer, &request.preconditions)?;
    lowerer.check_budget("authored preconditions")?;
    let extraction_count = request
        .changes
        .iter()
        .filter(|change| matches!(change, AuthoredChange::ExtractFunction { .. }))
        .count();
    if extraction_count > 1 {
        return Err(request_error(
            DiagnosticClass::Semantic,
            "change_extract_multiple",
            "one authored request may contain at most one extract.function operation",
        ));
    }
    for change in &request.changes {
        if let AuthoredChange::ExtractFunction {
            symbol,
            function,
            expression,
            name,
        } = change
        {
            extraction::lower(&mut lowerer, symbol, function, *expression, name)?;
        }
    }
    lowerer.check_budget("function extraction lowering")?;
    for change in &request.changes {
        if let AuthoredChange::CreateModule { symbol, name } = change {
            let module = lowerer.module_symbol(symbol)?;
            lowerer.insert_created(OwnerRecord::Module(ModuleRecord {
                header: OwnerHeader::new(OwnerKey::Module(module), OwnerKind::Module),
                name: name.clone(),
            }))?;
        }
        lowerer.check_budget("module creation lowering")?;
    }
    for change in &request.changes {
        match change {
            AuthoredChange::CreateRecord {
                symbol,
                module,
                name,
                visibility,
                fields,
            } => creation::lower_record(&mut lowerer, symbol, module, name, *visibility, fields)?,
            AuthoredChange::CreateVariant {
                symbol,
                module,
                name,
                visibility,
                cases,
            } => creation::lower_variant(&mut lowerer, symbol, module, name, *visibility, cases)?,
            AuthoredChange::CreateInterface {
                symbol,
                module,
                name,
                visibility,
                operations,
            } => creation::lower_interface(
                &mut lowerer,
                symbol,
                module,
                name,
                *visibility,
                operations,
            )?,
            AuthoredChange::CreateExternal {
                symbol,
                module,
                name,
                visibility,
                type_parameters,
                parameters,
                result,
                implementation,
            } => creation::lower_external(
                &mut lowerer,
                symbol,
                module,
                name,
                *visibility,
                type_parameters,
                parameters,
                result,
                implementation,
            )?,
            AuthoredChange::CreateFunction {
                symbol,
                module,
                name,
                visibility,
                type_parameters,
                parameters,
                result,
                effect,
                body,
            } => creation::lower_function(
                &mut lowerer,
                symbol,
                module,
                name,
                *visibility,
                type_parameters,
                parameters,
                result,
                effect,
                body,
            )?,
            AuthoredChange::CreateConstant {
                symbol,
                module,
                name,
                visibility,
                ty,
                value,
            } => creation::lower_constant(
                &mut lowerer,
                symbol,
                module,
                name,
                *visibility,
                ty,
                value,
            )?,
            AuthoredChange::CreateComponent {
                symbol,
                module,
                name,
                visibility,
                requirements,
                ports,
            } => creation::lower_component(
                &mut lowerer,
                symbol,
                module,
                name,
                *visibility,
                requirements,
                ports,
            )?,
            AuthoredChange::CreateTest {
                symbol,
                module,
                name,
                visibility,
                actual,
                expected,
            } => creation::lower_test(
                &mut lowerer,
                symbol,
                module,
                name,
                *visibility,
                actual,
                expected,
            )?,
            AuthoredChange::CreateTarget {
                symbol,
                name,
                component,
                port,
                runner,
            } => creation::lower_target(&mut lowerer, symbol, name, component, port, *runner)?,
            AuthoredChange::CreateDocumentation {
                symbol,
                owner,
                class,
                text,
            } => creation::lower_documentation(&mut lowerer, symbol, owner, *class, text)?,
            AuthoredChange::CreateAnnotation {
                symbol,
                owner,
                class,
                key,
                value,
            } => creation::lower_annotation(&mut lowerer, symbol, owner, *class, key, value)?,
            _ => {}
        }
        lowerer.check_budget("owner creation lowering")?;
    }
    for change in &request.changes {
        match change {
            AuthoredChange::AddField { .. }
            | AuthoredChange::AddCase { .. }
            | AuthoredChange::AddOperation { .. }
            | AuthoredChange::AddTypeParameter { .. }
            | AuthoredChange::AddRequirement { .. }
            | AuthoredChange::AddPort { .. }
            | AuthoredChange::AddHttpRoute { .. } => {
                creation::lower_mutation(&mut lowerer, change)?;
            }
            _ => {}
        }
        lowerer.check_budget("member addition lowering")?;
    }
    for change in &request.changes {
        if matches!(change, AuthoredChange::AddParameter { .. }) {
            creation::lower_mutation(&mut lowerer, change)?;
        }
        lowerer.check_budget("parameter addition lowering")?;
    }
    for change in &request.changes {
        match change {
            AuthoredChange::AddDependency {
                package,
                semantic_revision,
                package_revision,
            } => lowerer.add_dependency(DependencyRecord {
                graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
                package: *package,
                semantic_revision: *semantic_revision,
                package_revision: *package_revision,
            })?,
            AuthoredChange::ReplaceDependency {
                package,
                semantic_revision,
                package_revision,
            } => lowerer.replace_dependency(DependencyRecord {
                graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
                package: *package,
                semantic_revision: *semantic_revision,
                package_revision: *package_revision,
            })?,
            AuthoredChange::DeleteDependency { package } => {
                lowerer.delete_dependency(*package)?;
            }
            _ => {}
        }
        lowerer.check_budget("dependency lowering")?;
    }
    for change in &request.changes {
        match change {
            AuthoredChange::CreateModule { .. }
            | AuthoredChange::CreateRecord { .. }
            | AuthoredChange::CreateVariant { .. }
            | AuthoredChange::CreateInterface { .. }
            | AuthoredChange::CreateExternal { .. }
            | AuthoredChange::CreateFunction { .. }
            | AuthoredChange::CreateConstant { .. }
            | AuthoredChange::CreateComponent { .. }
            | AuthoredChange::CreateTest { .. }
            | AuthoredChange::CreateTarget { .. }
            | AuthoredChange::CreateDocumentation { .. }
            | AuthoredChange::CreateAnnotation { .. }
            | AuthoredChange::AddField { .. }
            | AuthoredChange::AddCase { .. }
            | AuthoredChange::AddOperation { .. }
            | AuthoredChange::AddTypeParameter { .. }
            | AuthoredChange::AddParameter { .. }
            | AuthoredChange::AddRequirement { .. }
            | AuthoredChange::AddPort { .. }
            | AuthoredChange::AddHttpRoute { .. }
            | AuthoredChange::AddDependency { .. }
            | AuthoredChange::ReplaceDependency { .. }
            | AuthoredChange::DeleteDependency { .. } => {}
            AuthoredChange::ExtractFunction { .. } => {}
            AuthoredChange::SetDeclarationVisibility { .. }
            | AuthoredChange::SetFunctionContract { .. }
            | AuthoredChange::SetExternalContract { .. }
            | AuthoredChange::SetFieldType { .. }
            | AuthoredChange::SetCasePayload { .. }
            | AuthoredChange::SetParameterType { .. }
            | AuthoredChange::SetOperationContract { .. }
            | AuthoredChange::SetRequirementContract { .. }
            | AuthoredChange::SetTarget { .. }
            | AuthoredChange::SetHttpRoute { .. } => {
                creation::lower_mutation(&mut lowerer, change)?;
            }
            AuthoredChange::DeleteOwner { .. } => {}
            AuthoredChange::RenameOwner { owner, name } => {
                let owner = lowerer.resolve_owner(owner)?;
                rename_owner(lowerer.candidate_mut(owner)?, name.clone())?;
            }
            AuthoredChange::MoveDeclaration {
                declaration,
                module,
            } => {
                let declaration = lowerer.resolve_declaration(declaration)?;
                let module = lowerer.resolve_module(module)?;
                let OwnerRecord::Declaration(record) =
                    lowerer.candidate_mut(OwnerKey::Declaration(declaration))?
                else {
                    return Err(request_error(
                        DiagnosticClass::Corrupt,
                        "change_authored_declaration_record",
                        "resolved declaration identity is bound to a foreign owner record",
                    ));
                };
                record.module = module;
            }
            AuthoredChange::ReplaceFunctionBody { function, body } => {
                let function = lowerer.resolve_declaration(function)?;
                let previous_body = {
                    let OwnerRecord::Declaration(record) =
                        lowerer.candidate_mut(OwnerKey::Declaration(function))?
                    else {
                        return Err(request_error(
                            DiagnosticClass::Corrupt,
                            "change_authored_declaration_record",
                            "resolved declaration identity is bound to a foreign owner record",
                        ));
                    };
                    let crate::platform::kernel::DeclarationPayload::Function(function) =
                        &record.payload
                    else {
                        return Err(request_error(
                            DiagnosticClass::Semantic,
                            "change_authored_function_kind",
                            "function-body replacement selector does not name a function declaration",
                        ));
                    };
                    function.body
                };
                let body = lowerer.lower_expression(body)?;
                let OwnerRecord::Declaration(record) =
                    lowerer.candidate_mut(OwnerKey::Declaration(function))?
                else {
                    return Err(request_error(
                        DiagnosticClass::Corrupt,
                        "change_authored_declaration_record",
                        "resolved function identity is bound to a foreign owner record",
                    ));
                };
                let crate::platform::kernel::DeclarationPayload::Function(function) =
                    &mut record.payload
                else {
                    return Err(request_error(
                        DiagnosticClass::Semantic,
                        "change_authored_function_kind",
                        "function-body replacement selector does not name a function declaration",
                    ));
                };
                function.body = body;
                deletion::retire_replaced_expression_tree(&mut lowerer, previous_body)?;
            }
        }
        lowerer.check_budget("authored mutation lowering")?;
    }
    deletion::lower_deletions(
        &mut lowerer,
        request.changes.iter().filter_map(|change| match change {
            AuthoredChange::DeleteOwner { owner, policy } => Some((owner, policy)),
            _ => None,
        }),
    )?;
    lowerer.check_budget("authored deletion lowering")?;
    lowerer.finish()
}

fn collect_symbol_definitions(
    request: &AuthoredChangeSet,
    maximum: u64,
) -> Result<(BTreeMap<String, SymbolDefinition>, usize), Diagnostic> {
    let mut definitions = SymbolDefinitions::new(maximum);
    for change in &request.changes {
        match change {
            AuthoredChange::CreateModule { symbol, .. } => {
                define_symbol(&mut definitions, symbol, SymbolKind::Module)?;
            }
            AuthoredChange::CreateRecord { symbol, fields, .. } => {
                creation::collect_record_symbols(symbol, fields, &mut definitions)?
            }
            AuthoredChange::CreateVariant { symbol, cases, .. } => {
                creation::collect_variant_symbols(symbol, cases, &mut definitions)?
            }
            AuthoredChange::CreateInterface {
                symbol, operations, ..
            } => creation::collect_interface_symbols(symbol, operations, &mut definitions)?,
            AuthoredChange::CreateExternal {
                symbol,
                type_parameters,
                parameters,
                ..
            } => creation::collect_external_symbols(
                symbol,
                type_parameters,
                parameters,
                &mut definitions,
            )?,
            AuthoredChange::CreateFunction {
                symbol,
                type_parameters,
                parameters,
                body,
                ..
            } => creation::collect_function_symbols(
                symbol,
                type_parameters,
                parameters,
                body,
                &mut definitions,
            )?,
            AuthoredChange::CreateConstant { symbol, value, .. } => {
                creation::collect_constant_symbols(symbol, value, &mut definitions)?
            }
            AuthoredChange::CreateComponent {
                symbol,
                requirements,
                ports,
                ..
            } => {
                creation::collect_component_symbols(symbol, requirements, ports, &mut definitions)?
            }
            AuthoredChange::CreateTest {
                symbol,
                actual,
                expected,
                ..
            } => creation::collect_test_symbols(symbol, actual, expected, &mut definitions)?,
            AuthoredChange::CreateTarget { symbol, .. } => {
                creation::collect_target_symbols(symbol, &mut definitions)?
            }
            AuthoredChange::AddHttpRoute { .. } => {}
            AuthoredChange::CreateDocumentation { symbol, .. } => {
                creation::collect_documentation_symbols(symbol, &mut definitions)?
            }
            AuthoredChange::CreateAnnotation { symbol, .. } => {
                creation::collect_annotation_symbols(symbol, &mut definitions)?
            }
            AuthoredChange::ExtractFunction { symbol, .. } => {
                define_symbol(&mut definitions, symbol, SymbolKind::Declaration)?;
            }
            AuthoredChange::AddField { .. }
            | AuthoredChange::AddCase { .. }
            | AuthoredChange::AddOperation { .. }
            | AuthoredChange::AddTypeParameter { .. }
            | AuthoredChange::AddParameter { .. }
            | AuthoredChange::AddRequirement { .. }
            | AuthoredChange::AddPort { .. }
            | AuthoredChange::SetDeclarationVisibility { .. }
            | AuthoredChange::SetFunctionContract { .. }
            | AuthoredChange::SetExternalContract { .. }
            | AuthoredChange::SetFieldType { .. }
            | AuthoredChange::SetCasePayload { .. }
            | AuthoredChange::SetParameterType { .. }
            | AuthoredChange::SetOperationContract { .. }
            | AuthoredChange::SetRequirementContract { .. }
            | AuthoredChange::SetTarget { .. }
            | AuthoredChange::SetHttpRoute { .. } => {
                creation::collect_mutation_symbols(change, &mut definitions)?
            }
            AuthoredChange::AddDependency { .. }
            | AuthoredChange::ReplaceDependency { .. }
            | AuthoredChange::DeleteDependency { .. } => {}
            AuthoredChange::DeleteOwner { .. }
            | AuthoredChange::RenameOwner { .. }
            | AuthoredChange::MoveDeclaration { .. } => {}
            AuthoredChange::ReplaceFunctionBody { body, .. } => {
                creation::collect_expression_symbols(body, &mut definitions)?
            }
        }
    }
    let mut routes = request
        .changes
        .iter()
        .filter_map(|change| match change {
            AuthoredChange::AddHttpRoute { symbol, .. } => Some((symbol, change)),
            _ => None,
        })
        .map(|(symbol, change)| {
            codec::authored_http_route_sort_key(change, &definitions.entries)
                .map(|key| (key, symbol.as_str()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    routes.sort_unstable_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| left.1.as_bytes().cmp(right.1.as_bytes()))
    });
    for (_, symbol) in routes {
        define_symbol(&mut definitions, symbol, SymbolKind::HttpRoute)?;
    }
    Ok(definitions.into_entries())
}

pub(super) struct SymbolDefinitions {
    maximum: u64,
    entries: BTreeMap<String, SymbolDefinition>,
    next_ordinals: BTreeMap<u8, u64>,
    anonymous_identities: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct SymbolDefinition {
    kind: SymbolKind,
    ordinal: u64,
}

impl SymbolDefinitions {
    fn new(maximum: u64) -> Self {
        Self {
            maximum,
            entries: BTreeMap::new(),
            next_ordinals: BTreeMap::new(),
            anonymous_identities: 0,
        }
    }

    fn into_entries(self) -> (BTreeMap<String, SymbolDefinition>, usize) {
        let identity_count = self.identity_count();
        (self.entries, identity_count)
    }

    fn identity_count(&self) -> usize {
        self.entries.len().saturating_add(self.anonymous_identities)
    }

    pub(super) fn define_anonymous_identity(&mut self) -> Result<(), Diagnostic> {
        self.admit_one_identity()?;
        self.anonymous_identities = self.anonymous_identities.saturating_add(1);
        Ok(())
    }

    fn admit_one_identity(&self) -> Result<(), Diagnostic> {
        if u64::try_from(self.identity_count()).unwrap_or(u64::MAX) >= self.maximum {
            return Err(request_error(
                DiagnosticClass::Resource,
                "change_budget_allocated_identities",
                format!(
                    "request-local identity collection exceeds the declared {}-identity budget",
                    self.maximum
                ),
            ));
        }
        Ok(())
    }
}

pub(super) fn define_symbol(
    definitions: &mut SymbolDefinitions,
    symbol: &str,
    kind: SymbolKind,
) -> Result<(), Diagnostic> {
    validate_symbol(symbol)?;
    if definitions.entries.contains_key(symbol) {
        return Err(request_error(
            DiagnosticClass::Source,
            "change_authored_symbol_duplicate",
            format!("request-local symbol {symbol} is defined more than once"),
        ));
    }
    definitions.admit_one_identity()?;
    let next = definitions
        .next_ordinals
        .entry(kind.allocation_domain())
        .or_default();
    *next = next.checked_add(1).ok_or_else(|| {
        request_error(
            DiagnosticClass::Resource,
            "change_authored_allocation_ordinal",
            "request-local allocation ordinal was exhausted",
        )
    })?;
    definitions.entries.insert(
        symbol.to_owned(),
        SymbolDefinition {
            kind,
            ordinal: *next,
        },
    );
    Ok(())
}

fn allocate_symbols(
    seed: &[u8],
    definitions: &BTreeMap<String, SymbolDefinition>,
) -> Result<BTreeMap<String, OwnerKey>, Diagnostic> {
    let mut allocated = BTreeMap::new();
    for (symbol, definition) in definitions {
        allocated.insert(
            symbol.clone(),
            definition.kind.allocate(seed, definition.ordinal),
        );
    }
    Ok(allocated)
}

fn logical_allocations(
    definitions: &BTreeMap<String, SymbolDefinition>,
    allocated: &BTreeMap<String, OwnerKey>,
) -> Result<Vec<AuthoredAllocation>, Diagnostic> {
    let mut allocations = Vec::new();
    allocations.try_reserve_exact(definitions.len()).map_err(|_| {
        request_error(
            DiagnosticClass::Resource,
            "change_authored_allocation_records",
            "request-local allocation record reservation failed within the declared identity budget",
        )
    })?;
    for (symbol, definition) in definitions {
        let owner = allocated.get(symbol).copied().ok_or_else(|| {
            request_error(
                DiagnosticClass::Corrupt,
                "change_authored_allocation_projection",
                "request-local allocation projection lost a normalized symbol",
            )
        })?;
        allocations.push(AuthoredAllocation {
            domain: definition.kind.identity_kind(),
            ordinal: definition.ordinal,
            owner,
        });
    }
    sort_allocations(&mut allocations);
    Ok(allocations)
}

pub(crate) fn canonical_authored_intent_bytes(
    request: &AuthoredChangeSet,
) -> Result<Vec<u8>, Diagnostic> {
    let (definitions, _) = collect_symbol_definitions(
        request,
        request.budget.authored.maximum_allocated_identities,
    )?;
    codec::encode_authored_intent(request, &definitions)
}

pub(crate) fn canonical_authored_budget_bytes(budget: ChangeBudget) -> Result<Vec<u8>, Diagnostic> {
    codec::encode_budget(budget)
}

fn allocation_seed<B: CanonicalBaseRead + ?Sized>(
    base: &B,
    request_bytes: &[u8],
) -> Result<[u8; 32], Diagnostic> {
    let request_length = u64::try_from(request_bytes.len()).map_err(|_| {
        request_error(
            DiagnosticClass::Resource,
            "change_authored_length",
            "authored change byte length exceeds its canonical allocation domain",
        )
    })?;
    let mut hasher = blake3::Hasher::new_derive_key(CHANGE_ALLOCATION_SEED_DOMAIN);
    hasher.update(&base.repository_id().bytes());
    hasher.update(&request_length.to_be_bytes());
    hasher.update(request_bytes);
    Ok(*hasher.finalize().as_bytes())
}

#[derive(Clone)]
struct WorkingOwner {
    before: Option<crate::platform::kernel::OwnerObjectDigest>,
    original: Option<OwnerRecord>,
    record: OwnerRecord,
    deleted: bool,
}

#[derive(Clone)]
struct WorkingDependency {
    before: Option<DependencyObjectDigest>,
    original: Option<DependencyRecord>,
    record: Option<DependencyRecord>,
}

struct AuthoredLoweringInputs {
    allocation_seed: [u8; 32],
    deletion_change: ChangeDigest,
    allocated: BTreeMap<String, OwnerKey>,
    definitions: BTreeMap<String, SymbolDefinition>,
    allocations: Vec<AuthoredAllocation>,
    budget: ChangeBudget,
}

struct AuthoredLowerer<'a, B: ?Sized, W: ?Sized> {
    base: &'a B,
    witness: &'a W,
    allocation_seed: [u8; 32],
    deletion_change: ChangeDigest,
    allocated: BTreeMap<String, OwnerKey>,
    definitions: BTreeMap<String, SymbolDefinition>,
    allocations: Vec<AuthoredAllocation>,
    owners: BTreeMap<OwnerKey, WorkingOwner>,
    owner_edits: BTreeSet<OwnerKey>,
    dependencies: BTreeMap<PackageId, WorkingDependency>,
    dependency_edits: BTreeSet<PackageId>,
    retirements: BTreeMap<OwnerKey, RetirementRecord>,
    retirement_edits: BTreeSet<OwnerKey>,
    namespace: BTreeMap<NamespaceKey, Option<OwnerKey>>,
    ownership: BTreeMap<OwnerKey, Option<crate::platform::witness::OwnershipEntry>>,
    incoming_relations: BTreeMap<OwnerKey, Vec<crate::platform::kernel::RelationEdge>>,
    base_types: BTreeMap<TypeObjectDigest, Option<TypeObject>>,
    types: TypeObjectInterner,
    type_additions: BTreeSet<TypeObjectDigest>,
    next_anonymous_expression_ordinal: u64,
    next_generated_parameter_ordinal: u64,
    extraction: Option<super::FunctionExtractionEvidence>,
    extraction_protected: BTreeSet<OwnerKey>,
    admitting_extraction: bool,
    work: AuthoredLoweringWork,
    budget: ChangeBudget,
}

impl<'a, B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized> AuthoredLowerer<'a, B, W> {
    fn new(
        base: &'a B,
        witness: &'a W,
        inputs: AuthoredLoweringInputs,
    ) -> Result<Self, Diagnostic> {
        let expression_symbol_count = inputs
            .definitions
            .values()
            .filter(|definition| definition.kind == SymbolKind::Expression)
            .count();
        let next_anonymous_expression_ordinal = u64::try_from(expression_symbol_count)
            .ok()
            .and_then(|count| count.checked_add(1))
            .ok_or_else(|| {
                request_error(
                    DiagnosticClass::Resource,
                    "change_authored_expression_ordinal",
                    "expression allocation ordinal was exhausted",
                )
            })?;
        let next_generated_parameter_ordinal = inputs
            .definitions
            .values()
            .filter(|definition| {
                matches!(
                    definition.kind,
                    SymbolKind::FunctionParameter | SymbolKind::OperationParameter
                )
            })
            .map(|definition| definition.ordinal)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| {
                request_error(
                    DiagnosticClass::Resource,
                    "change_authored_parameter_ordinal",
                    "generated parameter allocation ordinal was exhausted",
                )
            })?;
        Ok(Self {
            base,
            witness,
            allocation_seed: inputs.allocation_seed,
            deletion_change: inputs.deletion_change,
            allocated: inputs.allocated,
            definitions: inputs.definitions,
            allocations: inputs.allocations,
            owners: BTreeMap::new(),
            owner_edits: BTreeSet::new(),
            dependencies: BTreeMap::new(),
            dependency_edits: BTreeSet::new(),
            retirements: BTreeMap::new(),
            retirement_edits: BTreeSet::new(),
            namespace: BTreeMap::new(),
            ownership: BTreeMap::new(),
            incoming_relations: BTreeMap::new(),
            base_types: BTreeMap::new(),
            types: TypeObjectInterner::with_maximum_objects(
                usize::try_from(inputs.budget.authored.maximum_type_nodes).unwrap_or(usize::MAX),
            ),
            type_additions: BTreeSet::new(),
            next_anonymous_expression_ordinal,
            next_generated_parameter_ordinal,
            extraction: None,
            extraction_protected: BTreeSet::new(),
            admitting_extraction: false,
            work: AuthoredLoweringWork::default(),
            budget: inputs.budget,
        })
    }

    fn check_budget(&self, phase: &str) -> Result<(), Diagnostic> {
        self.budget.check_observed(self.work.budget_work(), phase)
    }

    fn insert_created(&mut self, record: OwnerRecord) -> Result<(), Diagnostic> {
        let owner = record.owner();
        if self.owners.contains_key(&owner) {
            return Err(request_error(
                DiagnosticClass::Corrupt,
                "change_authored_allocation_collision",
                "one request-local identity was allocated more than once",
            ));
        }
        self.admit_owner_edit(owner)?;
        self.owners.insert(
            owner,
            WorkingOwner {
                before: None,
                original: None,
                record,
                deleted: false,
            },
        );
        Ok(())
    }

    fn resolve_owner(&mut self, selector: &OwnerSelector) -> Result<OwnerKey, Diagnostic> {
        let owner = match selector {
            OwnerSelector::Exact { owner } => *owner,
            OwnerSelector::ModuleName { name } => {
                OwnerKey::Module(self.resolve_module(&ModuleSelector::Name { name: name.clone() })?)
            }
            OwnerSelector::DeclarationName { module, name } => OwnerKey::Declaration(
                self.resolve_declaration(&DeclarationSelector::Qualified {
                    module: module.clone(),
                    name: name.clone(),
                })?,
            ),
            OwnerSelector::Symbol { symbol } => self.resolve_symbol(symbol)?,
        };
        self.require_owner(owner)?;
        Ok(owner)
    }

    fn resolve_module(&mut self, selector: &ModuleSelector) -> Result<ModuleId, Diagnostic> {
        let owner = match selector {
            ModuleSelector::Id { module } => OwnerKey::Module(*module),
            ModuleSelector::Name { name } => self.namespace_owner(NamespaceKey {
                parent: None,
                class: NamespaceClass::Module,
                name: name.clone(),
            })?,
            ModuleSelector::Symbol { symbol } => self.resolve_symbol(symbol)?,
        };
        self.require_owner(owner)?;
        match owner {
            OwnerKey::Module(module) => Ok(module),
            _ => Err(request_error(
                DiagnosticClass::Semantic,
                "change_authored_module_kind",
                "module selector resolved to a foreign owner domain",
            )),
        }
    }

    fn resolve_declaration(
        &mut self,
        selector: &DeclarationSelector,
    ) -> Result<crate::platform::semantic_id::DeclarationId, Diagnostic> {
        let owner = match selector {
            DeclarationSelector::Id { declaration } => OwnerKey::Declaration(*declaration),
            DeclarationSelector::Qualified { module, name } => {
                let module = self.resolve_module(module)?;
                self.namespace_owner(NamespaceKey {
                    parent: Some(OwnerKey::Module(module)),
                    class: NamespaceClass::Declaration,
                    name: name.clone(),
                })?
            }
            DeclarationSelector::Symbol { symbol } => {
                self.symbol_owner(symbol, SymbolKind::Declaration)?
            }
        };
        self.require_owner(owner)?;
        match owner {
            OwnerKey::Declaration(declaration) => Ok(declaration),
            _ => Err(request_error(
                DiagnosticClass::Semantic,
                "change_authored_declaration_kind",
                "declaration selector resolved to a foreign owner domain",
            )),
        }
    }

    fn resolve_symbol(&self, symbol: &str) -> Result<OwnerKey, Diagnostic> {
        validate_symbol(symbol)?;
        self.allocated.get(symbol).copied().ok_or_else(|| {
            request_error(
                DiagnosticClass::Source,
                "change_authored_symbol_missing",
                format!("request-local symbol {symbol} has no unique definition"),
            )
        })
    }

    fn symbol_kind(&self, symbol: &str) -> Result<SymbolKind, Diagnostic> {
        validate_symbol(symbol)?;
        self.definitions
            .get(symbol)
            .map(|definition| definition.kind)
            .ok_or_else(|| {
                request_error(
                    DiagnosticClass::Source,
                    "change_authored_symbol_missing",
                    format!("request-local symbol {symbol} has no unique definition"),
                )
            })
    }

    fn symbol_owner(&self, symbol: &str, expected: SymbolKind) -> Result<OwnerKey, Diagnostic> {
        let actual = self.symbol_kind(symbol)?;
        if actual != expected {
            return Err(request_error(
                DiagnosticClass::Semantic,
                "change_authored_symbol_kind",
                format!("request-local symbol {symbol} has kind {actual:?}, expected {expected:?}"),
            ));
        }
        self.resolve_symbol(symbol)
    }

    fn module_symbol(&self, symbol: &str) -> Result<ModuleId, Diagnostic> {
        match self.symbol_owner(symbol, SymbolKind::Module)? {
            OwnerKey::Module(value) => Ok(value),
            _ => Err(symbol_domain_corrupt(symbol)),
        }
    }

    fn declaration_symbol(&self, symbol: &str) -> Result<DeclarationId, Diagnostic> {
        match self.symbol_owner(symbol, SymbolKind::Declaration)? {
            OwnerKey::Declaration(value) => Ok(value),
            _ => Err(symbol_domain_corrupt(symbol)),
        }
    }

    fn type_parameter_symbol(&self, symbol: &str) -> Result<TypeParameterId, Diagnostic> {
        match self.symbol_owner(symbol, SymbolKind::TypeParameter)? {
            OwnerKey::TypeParameter(value) => Ok(value),
            _ => Err(symbol_domain_corrupt(symbol)),
        }
    }

    fn field_symbol(&self, symbol: &str) -> Result<FieldId, Diagnostic> {
        match self.symbol_owner(symbol, SymbolKind::Field)? {
            OwnerKey::Field(value) => Ok(value),
            _ => Err(symbol_domain_corrupt(symbol)),
        }
    }

    fn case_symbol(&self, symbol: &str) -> Result<CaseId, Diagnostic> {
        match self.symbol_owner(symbol, SymbolKind::Case)? {
            OwnerKey::Case(value) => Ok(value),
            _ => Err(symbol_domain_corrupt(symbol)),
        }
    }

    fn operation_symbol(&self, symbol: &str) -> Result<OperationId, Diagnostic> {
        match self.symbol_owner(symbol, SymbolKind::Operation)? {
            OwnerKey::Operation(value) => Ok(value),
            _ => Err(symbol_domain_corrupt(symbol)),
        }
    }

    fn function_parameter_symbol(&self, symbol: &str) -> Result<ParameterId, Diagnostic> {
        match self.symbol_owner(symbol, SymbolKind::FunctionParameter)? {
            OwnerKey::Parameter(value) => Ok(value),
            _ => Err(symbol_domain_corrupt(symbol)),
        }
    }

    fn operation_parameter_symbol(&self, symbol: &str) -> Result<ParameterId, Diagnostic> {
        match self.symbol_owner(symbol, SymbolKind::OperationParameter)? {
            OwnerKey::Parameter(value) => Ok(value),
            _ => Err(symbol_domain_corrupt(symbol)),
        }
    }

    fn lexical_binding_symbol(&self, symbol: &str) -> Result<BindingId, Diagnostic> {
        match self.symbol_owner(symbol, SymbolKind::LexicalBinding)? {
            OwnerKey::Binding(value) => Ok(value),
            _ => Err(symbol_domain_corrupt(symbol)),
        }
    }

    fn match_payload_symbol(&self, symbol: &str) -> Result<BindingId, Diagnostic> {
        match self.symbol_owner(symbol, SymbolKind::MatchPayloadBinding)? {
            OwnerKey::Binding(value) => Ok(value),
            _ => Err(symbol_domain_corrupt(symbol)),
        }
    }

    fn transaction_binding_symbol(&self, symbol: &str) -> Result<BindingId, Diagnostic> {
        match self.symbol_owner(symbol, SymbolKind::TransactionBinding)? {
            OwnerKey::Binding(value) => Ok(value),
            _ => Err(symbol_domain_corrupt(symbol)),
        }
    }

    fn requirement_symbol(&self, symbol: &str) -> Result<RequirementId, Diagnostic> {
        match self.symbol_owner(symbol, SymbolKind::Requirement)? {
            OwnerKey::Requirement(value) => Ok(value),
            _ => Err(symbol_domain_corrupt(symbol)),
        }
    }

    fn port_symbol(&self, symbol: &str) -> Result<PortId, Diagnostic> {
        match self.symbol_owner(symbol, SymbolKind::Port)? {
            OwnerKey::Port(value) => Ok(value),
            _ => Err(symbol_domain_corrupt(symbol)),
        }
    }

    fn target_symbol(&self, symbol: &str) -> Result<TargetId, Diagnostic> {
        match self.symbol_owner(symbol, SymbolKind::Target)? {
            OwnerKey::Target(value) => Ok(value),
            _ => Err(symbol_domain_corrupt(symbol)),
        }
    }

    fn http_route_symbol(&self, symbol: &str) -> Result<HttpRouteId, Diagnostic> {
        match self.symbol_owner(symbol, SymbolKind::HttpRoute)? {
            OwnerKey::HttpRoute(value) => Ok(value),
            _ => Err(symbol_domain_corrupt(symbol)),
        }
    }

    fn documentation_symbol(&self, symbol: &str) -> Result<DocumentationId, Diagnostic> {
        match self.symbol_owner(symbol, SymbolKind::Documentation)? {
            OwnerKey::Documentation(value) => Ok(value),
            _ => Err(symbol_domain_corrupt(symbol)),
        }
    }

    fn annotation_symbol(&self, symbol: &str) -> Result<AnnotationId, Diagnostic> {
        match self.symbol_owner(symbol, SymbolKind::Annotation)? {
            OwnerKey::Annotation(value) => Ok(value),
            _ => Err(symbol_domain_corrupt(symbol)),
        }
    }

    fn resolve_creation_owner(&mut self, selector: &OwnerSelector) -> Result<OwnerKey, Diagnostic> {
        match selector {
            OwnerSelector::Symbol { symbol } => self.resolve_symbol(symbol),
            _ => self.resolve_owner(selector),
        }
    }

    fn resolve_creation_declaration(
        &mut self,
        selector: &DeclarationSelector,
    ) -> Result<DeclarationId, Diagnostic> {
        match selector {
            DeclarationSelector::Symbol { symbol } => self.declaration_symbol(symbol),
            _ => self.resolve_declaration(selector),
        }
    }

    fn lower_port_reference(
        &mut self,
        selector: &AuthoredPortReference,
    ) -> Result<crate::platform::kernel::PortReference, Diagnostic> {
        match selector {
            AuthoredPortReference::Exact { package, port } => {
                if *package == self.base.package_id() {
                    self.require_owner(OwnerKey::Port(*port))?;
                }
                Ok(crate::platform::kernel::PortReference {
                    package: *package,
                    port: *port,
                })
            }
            AuthoredPortReference::Symbol { symbol } => {
                Ok(crate::platform::kernel::PortReference {
                    package: self.base.package_id(),
                    port: self.port_symbol(symbol)?,
                })
            }
        }
    }

    fn expression_identity(&mut self, symbol: Option<&str>) -> Result<ExpressionId, Diagnostic> {
        if let Some(symbol) = symbol {
            return match self.symbol_owner(symbol, SymbolKind::Expression)? {
                OwnerKey::Expression(value) => Ok(value),
                _ => Err(symbol_domain_corrupt(symbol)),
            };
        }
        let allocated_identities =
            self.work
                .allocated_identities
                .checked_add(1)
                .ok_or_else(|| {
                    request_error(
                        DiagnosticClass::Resource,
                        "change_budget_allocated_identities",
                        "allocated identity observation overflowed",
                    )
                })?;
        self.budget.check_allocated_identities(
            usize::try_from(allocated_identities).unwrap_or(usize::MAX),
        )?;
        let ordinal = self.next_anonymous_expression_ordinal;
        self.next_anonymous_expression_ordinal = ordinal.checked_add(1).ok_or_else(|| {
            request_error(
                DiagnosticClass::Resource,
                "change_authored_expression_ordinal",
                "anonymous expression allocation ordinal was exhausted",
            )
        })?;
        self.work.allocated_identities = allocated_identities;
        let expression = ExpressionId::allocate(&self.allocation_seed, ordinal);
        self.allocations.push(AuthoredAllocation {
            domain: IdentityKind::Expression,
            ordinal,
            owner: OwnerKey::Expression(expression),
        });
        Ok(expression)
    }

    fn generated_parameter_identity(&mut self) -> Result<ParameterId, Diagnostic> {
        let allocated_identities =
            self.work
                .allocated_identities
                .checked_add(1)
                .ok_or_else(|| {
                    request_error(
                        DiagnosticClass::Resource,
                        "change_budget_allocated_identities",
                        "allocated identity observation overflowed",
                    )
                })?;
        self.budget.check_allocated_identities(
            usize::try_from(allocated_identities).unwrap_or(usize::MAX),
        )?;
        let ordinal = self.next_generated_parameter_ordinal;
        self.next_generated_parameter_ordinal = ordinal.checked_add(1).ok_or_else(|| {
            request_error(
                DiagnosticClass::Resource,
                "change_authored_parameter_ordinal",
                "generated parameter allocation ordinal was exhausted",
            )
        })?;
        self.work.allocated_identities = allocated_identities;
        let parameter = ParameterId::allocate(&self.allocation_seed, ordinal);
        self.allocations.push(AuthoredAllocation {
            domain: IdentityKind::Parameter,
            ordinal,
            owner: OwnerKey::Parameter(parameter),
        });
        Ok(parameter)
    }

    fn namespace_owner(&mut self, key: NamespaceKey) -> Result<OwnerKey, Diagnostic> {
        if !self.namespace.contains_key(&key) {
            let read = self.witness.read_namespace(&key)?;
            self.work.witness.add(read.work);
            self.namespace.insert(key.clone(), read.value);
        }
        self.namespace.get(&key).copied().flatten().ok_or_else(|| {
            request_error(
                DiagnosticClass::Semantic,
                "change_authored_selector_missing",
                format!("qualified selector has no owner at namespace key {key:?}"),
            )
        })
    }

    fn require_owner(&mut self, owner: OwnerKey) -> Result<(), Diagnostic> {
        if let Some(working) = self.owners.get(&owner) {
            return if working.deleted {
                Err(request_error(
                    DiagnosticClass::Semantic,
                    "change_authored_owner_deleted",
                    format!("owner {owner:?} was already selected for deletion in this request"),
                ))
            } else {
                Ok(())
            };
        }
        let read = self.base.read_owner(owner)?;
        self.work.canonical.add(read.work);
        let record = read.value.ok_or_else(|| {
            request_error(
                DiagnosticClass::Semantic,
                "change_authored_owner_missing",
                format!("selector names missing owner {owner:?}"),
            )
        })?;
        let (before, _) = encode_owner(&record)?;
        self.owners.insert(
            owner,
            WorkingOwner {
                before: Some(before),
                original: Some(record.clone()),
                record,
                deleted: false,
            },
        );
        Ok(())
    }

    fn candidate_mut(&mut self, owner: OwnerKey) -> Result<&mut OwnerRecord, Diagnostic> {
        self.admit_owner_edit(owner)?;
        self.require_owner(owner)?;
        self.owners
            .get_mut(&owner)
            .map(|working| &mut working.record)
            .ok_or_else(|| {
                request_error(
                    DiagnosticClass::Corrupt,
                    "change_authored_owner_cache",
                    "resolved owner was not retained in the authored candidate overlay",
                )
            })
    }

    fn candidate_type_object(
        &mut self,
        digest: TypeObjectDigest,
    ) -> Result<Option<TypeObject>, Diagnostic> {
        if let Some(object) = self.types.get(digest) {
            return Ok(Some(object.clone()));
        }
        if !self.base_types.contains_key(&digest) {
            let read = self.base.read_type_object(digest)?;
            self.work.canonical.add(read.work);
            self.base_types.insert(digest, read.value);
        }
        Ok(self.base_types.get(&digest).cloned().flatten())
    }

    fn load_dependency(&mut self, package: PackageId) -> Result<(), Diagnostic> {
        if self.dependencies.contains_key(&package) {
            return Ok(());
        }
        let read = self.base.read_dependency(package)?;
        self.work.canonical.add(read.work);
        let before = read
            .value
            .as_ref()
            .map(encode_dependency)
            .transpose()?
            .map(|(digest, _)| digest);
        self.dependencies.insert(
            package,
            WorkingDependency {
                before,
                original: None,
                record: read.value,
            },
        );
        Ok(())
    }

    fn add_dependency(&mut self, record: DependencyRecord) -> Result<(), Diagnostic> {
        self.validate_dependency_candidate(&record)?;
        self.admit_dependency_edit(record.package)?;
        self.load_dependency(record.package)?;
        let working = self
            .dependencies
            .get_mut(&record.package)
            .ok_or_else(dependency_cache_corrupt)?;
        if working.record.is_some() {
            return Err(request_error(
                DiagnosticClass::Semantic,
                "change_authored_dependency_present",
                format!(
                    "dependency package {} is already bound in the candidate change",
                    record.package
                ),
            ));
        }
        working.record = Some(record);
        Ok(())
    }

    fn replace_dependency(&mut self, record: DependencyRecord) -> Result<(), Diagnostic> {
        self.validate_dependency_candidate(&record)?;
        self.admit_dependency_edit(record.package)?;
        self.load_dependency(record.package)?;
        let working = self
            .dependencies
            .get_mut(&record.package)
            .ok_or_else(dependency_cache_corrupt)?;
        if working.record.is_none() {
            return Err(request_error(
                DiagnosticClass::Semantic,
                "change_authored_dependency_missing",
                format!(
                    "dependency package {} is absent from the candidate change",
                    record.package
                ),
            ));
        }
        if working.before.is_some() && working.original.is_none() {
            working.original = working.record.take();
        }
        working.record = Some(record);
        Ok(())
    }

    fn delete_dependency(&mut self, package: PackageId) -> Result<(), Diagnostic> {
        self.admit_dependency_edit(package)?;
        self.load_dependency(package)?;
        let working = self
            .dependencies
            .get_mut(&package)
            .ok_or_else(dependency_cache_corrupt)?;
        if working.record.is_none() {
            return Err(request_error(
                DiagnosticClass::Semantic,
                "change_authored_dependency_missing",
                format!("dependency package {package} is absent from the candidate change"),
            ));
        }
        if working.before.is_some() && working.original.is_none() {
            working.original = working.record.take();
        } else {
            working.record = None;
        }
        Ok(())
    }

    fn validate_dependency_candidate(&self, record: &DependencyRecord) -> Result<(), Diagnostic> {
        record.validate_local()?;
        if record.package == self.base.package_id() {
            return Err(request_error(
                DiagnosticClass::Semantic,
                "change_authored_dependency_self",
                "a package cannot bind itself as an exact dependency",
            ));
        }
        Ok(())
    }

    pub(super) fn admit_owner_edit(&mut self, owner: OwnerKey) -> Result<(), Diagnostic> {
        if !self.admitting_extraction && self.extraction_protected.contains(&owner) {
            return Err(request_error(
                DiagnosticClass::Semantic,
                "change_extract_conflict",
                format!("another authored operation touches extraction-protected owner {owner:?}"),
            ));
        }
        if self.owner_edits.contains(&owner) {
            return Ok(());
        }
        self.check_canonical_edit_admission(
            self.owner_edits.len().saturating_add(1),
            self.dependency_edits.len(),
            self.retirement_edits.len(),
            "authored owner edit admission",
        )?;
        self.owner_edits.insert(owner);
        Ok(())
    }

    fn admit_dependency_edit(&mut self, package: PackageId) -> Result<(), Diagnostic> {
        if self.dependency_edits.contains(&package) {
            return Ok(());
        }
        self.check_canonical_edit_admission(
            self.owner_edits.len(),
            self.dependency_edits.len().saturating_add(1),
            self.retirement_edits.len(),
            "authored dependency edit admission",
        )?;
        self.dependency_edits.insert(package);
        Ok(())
    }

    pub(super) fn admit_retirement_edit(&mut self, owner: OwnerKey) -> Result<(), Diagnostic> {
        if self.retirement_edits.contains(&owner) {
            return Ok(());
        }
        self.check_canonical_edit_admission(
            self.owner_edits.len(),
            self.dependency_edits.len(),
            self.retirement_edits.len().saturating_add(1),
            "authored retirement edit admission",
        )?;
        self.retirement_edits.insert(owner);
        Ok(())
    }

    fn check_canonical_edit_admission(
        &self,
        owner_edits: usize,
        dependency_edits: usize,
        retirement_edits: usize,
        phase: &str,
    ) -> Result<(), Diagnostic> {
        self.budget.check_canonical_edit_counts(
            u64::try_from(owner_edits).unwrap_or(u64::MAX),
            u64::try_from(self.type_additions.len()).unwrap_or(u64::MAX),
            u64::try_from(dependency_edits).unwrap_or(u64::MAX),
            u64::try_from(retirement_edits).unwrap_or(u64::MAX),
            phase,
        )
    }

    fn finish(mut self) -> Result<AuthoredLowering, Diagnostic> {
        self.work.type_nodes_interned = u64::try_from(self.types.len()).unwrap_or(u64::MAX);
        let type_objects = self.types.into_objects();
        self.budget.check_canonical_edit_counts(
            u64::try_from(self.owner_edits.len()).unwrap_or(u64::MAX),
            u64::try_from(self.type_additions.len()).unwrap_or(u64::MAX),
            u64::try_from(self.dependency_edits.len()).unwrap_or(u64::MAX),
            u64::try_from(self.retirement_edits.len()).unwrap_or(u64::MAX),
            "authored canonical edit preflight",
        )?;
        let estimated_edits = self
            .owner_edits
            .len()
            .saturating_add(self.type_additions.len())
            .saturating_add(self.dependency_edits.len())
            .saturating_add(self.retirement_edits.len());
        let mut edits = Vec::with_capacity(estimated_edits);
        for (digest, object) in type_objects {
            if self.type_additions.contains(&digest) {
                edits.push(PrimitiveEdit::AddTypeObject { digest, object });
            }
        }
        for (_, working) in self.owners {
            if working.deleted {
                let expected = working.before.ok_or_else(|| {
                    request_error(
                        DiagnosticClass::Corrupt,
                        "change_authored_delete_created",
                        "a request-local creation cannot be emitted as an accepted owner deletion",
                    )
                })?;
                edits.push(PrimitiveEdit::DeleteOwner {
                    owner: working.record.owner(),
                    expected,
                });
                continue;
            }
            let (after, _) = encode_owner(&working.record)?;
            match working.before {
                None => edits.push(PrimitiveEdit::InsertOwner {
                    record: working.record,
                }),
                Some(before) if before != after => edits.push(PrimitiveEdit::ReplaceOwner {
                    expected: before,
                    record: working.record,
                }),
                Some(_) => {}
            }
        }
        let mut dependency_befores = BTreeMap::new();
        for (package, working) in self.dependencies {
            let WorkingDependency {
                before,
                original,
                record,
            } = working;
            match (before, record) {
                (None, Some(record)) => edits.push(PrimitiveEdit::InsertDependency { record }),
                (Some(before), Some(record)) => {
                    let (after, _) = encode_dependency(&record)?;
                    if before != after {
                        let original = original.ok_or_else(dependency_before_corrupt)?;
                        dependency_befores.insert(package, original);
                        edits.push(PrimitiveEdit::ReplaceDependency {
                            expected: before,
                            record,
                        });
                    }
                }
                (Some(expected), None) => {
                    let original = original.ok_or_else(dependency_before_corrupt)?;
                    dependency_befores.insert(package, original);
                    edits.push(PrimitiveEdit::DeleteDependency { package, expected })
                }
                (None, None) => {}
            }
        }
        for (_, record) in self.retirements {
            edits.push(PrimitiveEdit::InsertRetirement { record });
        }
        Ok(AuthoredLowering {
            edits,
            allocated: self.allocated,
            allocations: {
                sort_allocations(&mut self.allocations);
                self.allocations
            },
            dependency_befores,
            extraction: self.extraction,
            work: self.work,
        })
    }
}

fn sort_allocations(allocations: &mut [AuthoredAllocation]) {
    allocations.sort_unstable_by(|left, right| {
        left.domain
            .tag()
            .cmp(&right.domain.tag())
            .then_with(|| left.ordinal.cmp(&right.ordinal))
            .then_with(|| EncodedOwnerKey::new(left.owner).cmp(&EncodedOwnerKey::new(right.owner)))
    });
}

fn dependency_cache_corrupt() -> Diagnostic {
    request_error(
        DiagnosticClass::Corrupt,
        "change_authored_dependency_cache",
        "resolved dependency was not retained in the authored candidate overlay",
    )
}

fn dependency_before_corrupt() -> Diagnostic {
    request_error(
        DiagnosticClass::Corrupt,
        "change_authored_dependency_before",
        "changed dependency lost its exact logical base binding",
    )
}

fn rename_owner(record: &mut OwnerRecord, name: Name) -> Result<(), Diagnostic> {
    let target = record.name_mut().ok_or_else(|| {
        request_error(
            DiagnosticClass::Semantic,
            "change_authored_rename_kind",
            "selected owner kind has no renameable semantic name",
        )
    })?;
    *target = name;
    Ok(())
}

fn validate_symbol(symbol: &str) -> Result<(), Diagnostic> {
    if !symbol.starts_with('$')
        || symbol.len() < 2
        || symbol.len() > MAXIMUM_REQUEST_SYMBOL_BYTES
        || !symbol.as_bytes()[1..]
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(*byte, b'_' | b'-'))
    {
        return Err(request_error(
            DiagnosticClass::Source,
            "change_authored_symbol",
            format!(
                "request-local symbol must start with '$' and contain 1 through {} ASCII name bytes",
                MAXIMUM_REQUEST_SYMBOL_BYTES - 1
            ),
        ));
    }
    Ok(())
}

fn request_error(
    class: DiagnosticClass,
    code: &'static str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

fn symbol_domain_corrupt(symbol: &str) -> Diagnostic {
    request_error(
        DiagnosticClass::Corrupt,
        "change_authored_symbol_domain",
        format!("request-local symbol {symbol} disagrees with its allocated identity domain"),
    )
}
