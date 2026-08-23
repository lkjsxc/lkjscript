//! Canonical logical meaning graph records and packed module shards.

use super::contract::registry::{MODULE_DIGEST_DOMAIN, MODULE_MAGIC};
use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::language::{Declaration, Effect, Expression, Module};
pub use super::language::{DeclarationReference, ModuleReference};
use super::packed;
use super::semantic_digest::ModuleObjectDigest;
use super::semantic_id::{
    AnnotationId, BindingId, CaseId, DeclarationId, DocumentationId, ExpressionId, FieldId,
    ModuleId, OperationId, ParameterId, PortId, RequirementId, TargetId, TypeParameterId,
};
use super::syntax::SourceSpan;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const GRAPH_CONTRACT_VERSION: u16 = 4;
pub const GRAPH_CONTRACT_IDENTITY: &str = "lkjscript-meaning-graph-4";
pub const MAXIMUM_MODULE_SEGMENT_BYTES: usize = 64 * 1_048_576;
pub const MAXIMUM_MODULE_DECLARATIONS: usize = 100_000;
pub const MAXIMUM_MODULE_IDENTITIES: usize = 2_000_000;
pub const MAXIMUM_EXPRESSION_DEPTH: usize = 256;
pub const MAXIMUM_DOCUMENTATION_BYTES: usize = 16 * 1_048_576;

#[derive(
    Decode, Encode, Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(rename_all = "snake_case")]
pub enum DeclarationKind {
    Record,
    Variant,
    Interface,
    External,
    PureFunction,
    TaskFunction,
    Constant,
    Component,
    Test,
}

impl DeclarationKind {
    pub fn of(declaration: &Declaration) -> Self {
        match declaration {
            Declaration::Record(_) => Self::Record,
            Declaration::Variant(_) => Self::Variant,
            Declaration::Interface(_) => Self::Interface,
            Declaration::External(_) => Self::External,
            Declaration::Function(function) => match function.effect {
                Effect::Pure => Self::PureFunction,
                Effect::Task { .. } => Self::TaskFunction,
            },
            Declaration::Constant(_) => Self::Constant,
            Declaration::Component(_) => Self::Component,
            Declaration::Test(_) => Self::Test,
        }
    }
}

#[derive(Decode, Encode, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MemberIdentity {
    TypeParameter { id: TypeParameterId, name: String },
    Field { id: FieldId, name: String },
    Case { id: CaseId, name: String },
    Operation { id: OperationId, name: String },
    Parameter { id: ParameterId, name: String },
    TaskRequirement { id: RequirementId, name: String },
    ComponentRequirement { id: RequirementId, name: String },
    Port { id: PortId, name: String },
}

impl MemberIdentity {
    pub fn name(&self) -> &str {
        match self {
            Self::TypeParameter { name, .. }
            | Self::Field { name, .. }
            | Self::Case { name, .. }
            | Self::Operation { name, .. }
            | Self::Parameter { name, .. }
            | Self::TaskRequirement { name, .. }
            | Self::ComponentRequirement { name, .. }
            | Self::Port { name, .. } => name,
        }
    }
}

#[derive(Decode, Encode, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BindingIdentity {
    pub id: BindingId,
    pub name: String,
    pub expression_path: Vec<u32>,
    pub slot: u32,
}

#[derive(
    Decode, Encode, Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ExpressionKind {
    Unit,
    Bool,
    I64,
    Text,
    StaticText,
    Variable,
    ConstantReference,
    If,
    Let,
    Do,
    Call,
    Invoke,
    Record,
    Variant,
    Field,
    List,
    Map,
    Match,
    FunctionReference,
    CapabilityCall,
    Transaction,
}

#[derive(Decode, Encode, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExpressionIdentity {
    pub id: ExpressionId,
    pub path: Vec<u32>,
    pub kind: ExpressionKind,
}

#[derive(Decode, Encode, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeclarationIdentity {
    pub id: DeclarationId,
    pub name: String,
    pub kind: DeclarationKind,
    pub members: Vec<MemberIdentity>,
    pub bindings: Vec<BindingIdentity>,
    pub expressions: Vec<ExpressionIdentity>,
}

#[derive(Decode, Encode, Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum RelationSource {
    Module(ModuleId),
    Declaration(DeclarationId),
    Field(FieldId),
    Case(CaseId),
    Operation(OperationId),
    Parameter(ParameterId),
    Binding(BindingId),
    Requirement(RequirementId),
    Port(PortId),
    Expression(ExpressionId),
    Target(TargetId),
}

#[derive(Decode, Encode, Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum RelationTarget {
    Module(ModuleReference),
    Declaration(DeclarationReference),
    Field {
        owner: DeclarationReference,
        field: FieldId,
    },
    Case {
        owner: DeclarationReference,
        case: CaseId,
    },
    Operation {
        owner: DeclarationReference,
        operation: OperationId,
    },
    TypeParameter {
        owner: DeclarationReference,
        type_parameter: TypeParameterId,
    },
    Parameter {
        owner: DeclarationReference,
        parameter: ParameterId,
    },
    Binding {
        owner: DeclarationReference,
        binding: BindingId,
    },
    Requirement {
        owner: DeclarationReference,
        requirement: RequirementId,
    },
    Port {
        owner: DeclarationReference,
        port: PortId,
    },
}

#[derive(
    Decode, Encode, Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(rename_all = "snake_case")]
pub enum RelationRole {
    Import,
    Export,
    TypeUse,
    ValueReference,
    Call,
    FieldUse,
    VariantConstruction,
    VariantPattern,
    CapabilityInterface,
    CapabilityOperation,
    ComponentPortFunction,
    TargetComponent,
    TargetPort,
    TestDependency,
}

impl RelationRole {
    pub const ALL: [Self; 14] = [
        Self::Import,
        Self::Export,
        Self::TypeUse,
        Self::ValueReference,
        Self::Call,
        Self::FieldUse,
        Self::VariantConstruction,
        Self::VariantPattern,
        Self::CapabilityInterface,
        Self::CapabilityOperation,
        Self::ComponentPortFunction,
        Self::TargetComponent,
        Self::TargetPort,
        Self::TestDependency,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Import => "import",
            Self::Export => "export",
            Self::TypeUse => "type_use",
            Self::ValueReference => "value_reference",
            Self::Call => "call",
            Self::FieldUse => "field_use",
            Self::VariantConstruction => "variant_construction",
            Self::VariantPattern => "variant_pattern",
            Self::CapabilityInterface => "capability_interface",
            Self::CapabilityOperation => "capability_operation",
            Self::ComponentPortFunction => "component_port_function",
            Self::TargetComponent => "target_component",
            Self::TargetPort => "target_port",
            Self::TestDependency => "test_dependency",
        }
    }
}

#[derive(Decode, Encode, Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticRelation {
    pub source: RelationSource,
    pub target: RelationTarget,
    pub role: RelationRole,
}

#[derive(Decode, Encode, Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum DocumentationOwner {
    Module(ModuleId),
    Declaration(DeclarationId),
    Field(FieldId),
    Case(CaseId),
    Operation(OperationId),
    Port(PortId),
}

#[derive(Decode, Encode, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Documentation {
    pub id: DocumentationId,
    pub owner: DocumentationOwner,
    pub text: String,
}

#[derive(Decode, Encode, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AnnotationClass {
    Semantic,
    Nonsemantic,
}

#[derive(Decode, Encode, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Annotation {
    pub id: AnnotationId,
    pub owner: DocumentationOwner,
    pub class: AnnotationClass,
    pub key: String,
    pub value: String,
}

#[derive(Decode, Encode, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MeaningModule {
    pub graph_contract_version: u16,
    pub module_id: ModuleId,
    pub module: Module,
    pub declarations: Vec<DeclarationIdentity>,
    pub relations: Vec<SemanticRelation>,
    pub documentation: Vec<Documentation>,
    pub annotations: Vec<Annotation>,
}

impl MeaningModule {
    pub fn import(
        mut module: Module,
        allocator: &mut MigrationIdentityAllocator,
    ) -> Result<Self, Diagnostic> {
        Self::allocate(&mut module, allocator)
    }

    /// Allocates stable semantic identities for a normalized public change. Request allocation
    /// is domain-separated from one-time migration and is deterministic under its exact seed.
    pub fn create(
        mut module: Module,
        allocator: &mut RequestIdentityAllocator,
    ) -> Result<Self, Diagnostic> {
        Self::allocate(&mut module, allocator)
    }

    pub fn create_declaration_identity(
        declaration: &Declaration,
        allocator: &mut RequestIdentityAllocator,
    ) -> Result<DeclarationIdentity, Diagnostic> {
        import_declaration_identity(declaration, allocator)
    }

    fn allocate(
        module: &mut Module,
        allocator: &mut impl IdentityAllocation,
    ) -> Result<Self, Diagnostic> {
        normalize_module_spans(module);
        let module_id = allocator.module()?;
        let mut declarations = Vec::with_capacity(module.declarations.len());
        for declaration in &module.declarations {
            declarations.push(import_declaration_identity(declaration, allocator)?);
        }
        let value = Self {
            graph_contract_version: GRAPH_CONTRACT_VERSION,
            module_id,
            module: module.clone(),
            declarations,
            relations: Vec::new(),
            documentation: Vec::new(),
            annotations: Vec::new(),
        };
        value.validate_identity_shape()?;
        Ok(value)
    }

    pub fn encode(&self) -> Result<Vec<u8>, Diagnostic> {
        self.validate_identity_shape()?;
        packed::encode(
            MODULE_MAGIC,
            MODULE_DIGEST_DOMAIN,
            self,
            MAXIMUM_MODULE_SEGMENT_BYTES,
        )
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, Diagnostic> {
        if bytes.len() > MAXIMUM_MODULE_SEGMENT_BYTES + 50 {
            return Err(meaning_error(
                DiagnosticClass::Resource,
                "meaning_module_size",
                format!("meaning module exceeds {MAXIMUM_MODULE_SEGMENT_BYTES} payload bytes"),
            ));
        }
        let value: Self = packed::decode(
            bytes,
            MODULE_MAGIC,
            MODULE_DIGEST_DOMAIN,
            MAXIMUM_MODULE_SEGMENT_BYTES,
        )?;
        value.validate_identity_shape()?;
        Ok(value)
    }

    pub fn digest(&self) -> Result<ModuleObjectDigest, Diagnostic> {
        let encoded = self.encode()?;
        Ok(ModuleObjectDigest::of(&encoded))
    }

    pub fn declaration(&self, id: DeclarationId) -> Option<(&DeclarationIdentity, &Declaration)> {
        self.declarations
            .iter()
            .zip(&self.module.declarations)
            .find(|(identity, _)| identity.id == id)
    }

    pub fn declaration_by_name(&self, name: &str) -> Option<(&DeclarationIdentity, &Declaration)> {
        self.declarations
            .iter()
            .zip(&self.module.declarations)
            .find(|(identity, _)| identity.name == name)
    }

    pub fn validate_identity_shape(&self) -> Result<(), Diagnostic> {
        if self.graph_contract_version != GRAPH_CONTRACT_VERSION {
            return Err(meaning_error(
                DiagnosticClass::Source,
                "meaning_graph_contract",
                format!(
                    "meaning module graph contract {} is not current contract {GRAPH_CONTRACT_VERSION}",
                    self.graph_contract_version
                ),
            ));
        }
        let mut normalized_module = self.module.clone();
        normalize_module_spans(&mut normalized_module);
        if normalized_module != self.module {
            return Err(meaning_error(
                DiagnosticClass::Corrupt,
                "meaning_source_coordinate",
                "canonical meaning contains a nonsemantic source coordinate",
            ));
        }
        if self.module.declarations.len() > MAXIMUM_MODULE_DECLARATIONS
            || self.declarations.len() != self.module.declarations.len()
        {
            return Err(meaning_error(
                DiagnosticClass::Corrupt,
                "meaning_declaration_identity_count",
                "meaning module declaration identities do not match declarations",
            ));
        }
        let mut declarations = BTreeSet::new();
        let mut all_identity_bytes = BTreeSet::new();
        let mut identity_count = 1usize;
        all_identity_bytes.insert(("module", self.module_id.bytes()));
        for (identity, declaration) in self.declarations.iter().zip(&self.module.declarations) {
            if identity.name != declaration.name()
                || identity.kind != DeclarationKind::of(declaration)
                || !declarations.insert(identity.id)
                || !all_identity_bytes.insert(("declaration", identity.id.bytes()))
            {
                return Err(meaning_error(
                    DiagnosticClass::Corrupt,
                    "meaning_declaration_identity",
                    "declaration identity, name, kind, or uniqueness is inconsistent",
                ));
            }
            validate_member_shape(identity, declaration, &mut all_identity_bytes)?;
            validate_expression_shape(identity, declaration)?;
            identity_count = identity_count
                .checked_add(1 + identity.members.len() + identity.bindings.len())
                .and_then(|value| value.checked_add(identity.expressions.len()))
                .ok_or_else(|| {
                    meaning_error(
                        DiagnosticClass::Resource,
                        "meaning_identity_count",
                        "meaning identity count overflowed",
                    )
                })?;
        }
        if identity_count > MAXIMUM_MODULE_IDENTITIES {
            return Err(meaning_error(
                DiagnosticClass::Resource,
                "meaning_identity_limit",
                format!("meaning module exceeds {MAXIMUM_MODULE_IDENTITIES} identities"),
            ));
        }
        let mut relations = self.relations.clone();
        relations.sort();
        if relations != self.relations || relations.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(meaning_error(
                DiagnosticClass::Corrupt,
                "meaning_relation_order",
                "semantic relations must be unique and canonically ordered",
            ));
        }
        let documentation_bytes = self
            .documentation
            .iter()
            .try_fold(0usize, |total, item| total.checked_add(item.text.len()))
            .ok_or_else(|| {
                meaning_error(
                    DiagnosticClass::Resource,
                    "meaning_documentation_length",
                    "documentation byte count overflowed",
                )
            })?;
        if documentation_bytes > MAXIMUM_DOCUMENTATION_BYTES {
            return Err(meaning_error(
                DiagnosticClass::Resource,
                "meaning_documentation_limit",
                format!("module documentation exceeds {MAXIMUM_DOCUMENTATION_BYTES} bytes"),
            ));
        }
        Ok(())
    }
}

trait IdentityAllocation {
    fn module(&mut self) -> Result<ModuleId, Diagnostic>;
    fn declaration(&mut self) -> Result<DeclarationId, Diagnostic>;
    fn field(&mut self) -> Result<FieldId, Diagnostic>;
    fn case(&mut self) -> Result<CaseId, Diagnostic>;
    fn operation(&mut self) -> Result<OperationId, Diagnostic>;
    fn type_parameter(&mut self) -> Result<TypeParameterId, Diagnostic>;
    fn parameter(&mut self) -> Result<ParameterId, Diagnostic>;
    fn binding(&mut self) -> Result<BindingId, Diagnostic>;
    fn expression(&mut self) -> Result<ExpressionId, Diagnostic>;
    fn requirement(&mut self) -> Result<RequirementId, Diagnostic>;
    fn port(&mut self) -> Result<PortId, Diagnostic>;
}

#[derive(Clone, Debug)]
pub struct MigrationIdentityAllocator {
    seed: Vec<u8>,
    module: u64,
    declaration: u64,
    field: u64,
    case: u64,
    operation: u64,
    type_parameter: u64,
    parameter: u64,
    binding: u64,
    expression: u64,
    requirement: u64,
    port: u64,
}

impl MigrationIdentityAllocator {
    pub fn new(seed: impl Into<Vec<u8>>) -> Self {
        Self {
            seed: seed.into(),
            module: 0,
            declaration: 0,
            field: 0,
            case: 0,
            operation: 0,
            type_parameter: 0,
            parameter: 0,
            binding: 0,
            expression: 0,
            requirement: 0,
            port: 0,
        }
    }

    fn next(counter: &mut u64) -> Result<u64, Diagnostic> {
        *counter = counter.checked_add(1).ok_or_else(|| {
            meaning_error(
                DiagnosticClass::Resource,
                "meaning_identity_ordinal_exhausted",
                "semantic identity allocation ordinal was exhausted",
            )
        })?;
        Ok(*counter)
    }

    fn module(&mut self) -> Result<ModuleId, Diagnostic> {
        Ok(ModuleId::migrate(&self.seed, Self::next(&mut self.module)?))
    }

    fn declaration(&mut self) -> Result<DeclarationId, Diagnostic> {
        Ok(DeclarationId::migrate(
            &self.seed,
            Self::next(&mut self.declaration)?,
        ))
    }

    fn field(&mut self) -> Result<FieldId, Diagnostic> {
        Ok(FieldId::migrate(&self.seed, Self::next(&mut self.field)?))
    }

    fn case(&mut self) -> Result<CaseId, Diagnostic> {
        Ok(CaseId::migrate(&self.seed, Self::next(&mut self.case)?))
    }

    fn operation(&mut self) -> Result<OperationId, Diagnostic> {
        Ok(OperationId::migrate(
            &self.seed,
            Self::next(&mut self.operation)?,
        ))
    }

    fn type_parameter(&mut self) -> Result<TypeParameterId, Diagnostic> {
        Ok(TypeParameterId::migrate(
            &self.seed,
            Self::next(&mut self.type_parameter)?,
        ))
    }

    fn parameter(&mut self) -> Result<ParameterId, Diagnostic> {
        Ok(ParameterId::migrate(
            &self.seed,
            Self::next(&mut self.parameter)?,
        ))
    }

    fn binding(&mut self) -> Result<BindingId, Diagnostic> {
        Ok(BindingId::migrate(
            &self.seed,
            Self::next(&mut self.binding)?,
        ))
    }

    fn expression(&mut self) -> Result<ExpressionId, Diagnostic> {
        Ok(ExpressionId::migrate(
            &self.seed,
            Self::next(&mut self.expression)?,
        ))
    }

    fn requirement(&mut self) -> Result<RequirementId, Diagnostic> {
        Ok(RequirementId::migrate(
            &self.seed,
            Self::next(&mut self.requirement)?,
        ))
    }

    fn port(&mut self) -> Result<PortId, Diagnostic> {
        Ok(PortId::migrate(&self.seed, Self::next(&mut self.port)?))
    }
}

impl IdentityAllocation for MigrationIdentityAllocator {
    fn module(&mut self) -> Result<ModuleId, Diagnostic> {
        Self::module(self)
    }

    fn declaration(&mut self) -> Result<DeclarationId, Diagnostic> {
        Self::declaration(self)
    }

    fn field(&mut self) -> Result<FieldId, Diagnostic> {
        Self::field(self)
    }

    fn case(&mut self) -> Result<CaseId, Diagnostic> {
        Self::case(self)
    }

    fn operation(&mut self) -> Result<OperationId, Diagnostic> {
        Self::operation(self)
    }

    fn type_parameter(&mut self) -> Result<TypeParameterId, Diagnostic> {
        Self::type_parameter(self)
    }

    fn parameter(&mut self) -> Result<ParameterId, Diagnostic> {
        Self::parameter(self)
    }

    fn binding(&mut self) -> Result<BindingId, Diagnostic> {
        Self::binding(self)
    }

    fn expression(&mut self) -> Result<ExpressionId, Diagnostic> {
        Self::expression(self)
    }

    fn requirement(&mut self) -> Result<RequirementId, Diagnostic> {
        Self::requirement(self)
    }

    fn port(&mut self) -> Result<PortId, Diagnostic> {
        Self::port(self)
    }
}

#[derive(Clone, Debug)]
pub struct RequestIdentityAllocator {
    seed: Vec<u8>,
    module: u64,
    declaration: u64,
    field: u64,
    case: u64,
    operation: u64,
    type_parameter: u64,
    parameter: u64,
    binding: u64,
    expression: u64,
    requirement: u64,
    port: u64,
    target: u64,
}

impl RequestIdentityAllocator {
    pub fn new(seed: impl Into<Vec<u8>>) -> Self {
        Self {
            seed: seed.into(),
            module: 0,
            declaration: 0,
            field: 0,
            case: 0,
            operation: 0,
            type_parameter: 0,
            parameter: 0,
            binding: 0,
            expression: 0,
            requirement: 0,
            port: 0,
            target: 0,
        }
    }

    fn next(counter: &mut u64) -> Result<u64, Diagnostic> {
        *counter = counter.checked_add(1).ok_or_else(|| {
            meaning_error(
                DiagnosticClass::Resource,
                "meaning_request_identity_ordinal_exhausted",
                "request-local identity allocation ordinal was exhausted",
            )
        })?;
        Ok(*counter)
    }

    pub fn allocate_module(&mut self) -> Result<ModuleId, Diagnostic> {
        <Self as IdentityAllocation>::module(self)
    }

    pub fn allocate_target(&mut self) -> Result<TargetId, Diagnostic> {
        Ok(TargetId::allocate(
            &self.seed,
            Self::next(&mut self.target)?,
        ))
    }
}

impl IdentityAllocation for RequestIdentityAllocator {
    fn module(&mut self) -> Result<ModuleId, Diagnostic> {
        Ok(ModuleId::allocate(
            &self.seed,
            Self::next(&mut self.module)?,
        ))
    }

    fn declaration(&mut self) -> Result<DeclarationId, Diagnostic> {
        Ok(DeclarationId::allocate(
            &self.seed,
            Self::next(&mut self.declaration)?,
        ))
    }

    fn field(&mut self) -> Result<FieldId, Diagnostic> {
        Ok(FieldId::allocate(&self.seed, Self::next(&mut self.field)?))
    }

    fn case(&mut self) -> Result<CaseId, Diagnostic> {
        Ok(CaseId::allocate(&self.seed, Self::next(&mut self.case)?))
    }

    fn operation(&mut self) -> Result<OperationId, Diagnostic> {
        Ok(OperationId::allocate(
            &self.seed,
            Self::next(&mut self.operation)?,
        ))
    }

    fn type_parameter(&mut self) -> Result<TypeParameterId, Diagnostic> {
        Ok(TypeParameterId::allocate(
            &self.seed,
            Self::next(&mut self.type_parameter)?,
        ))
    }

    fn parameter(&mut self) -> Result<ParameterId, Diagnostic> {
        Ok(ParameterId::allocate(
            &self.seed,
            Self::next(&mut self.parameter)?,
        ))
    }

    fn binding(&mut self) -> Result<BindingId, Diagnostic> {
        Ok(BindingId::allocate(
            &self.seed,
            Self::next(&mut self.binding)?,
        ))
    }

    fn expression(&mut self) -> Result<ExpressionId, Diagnostic> {
        Ok(ExpressionId::allocate(
            &self.seed,
            Self::next(&mut self.expression)?,
        ))
    }

    fn requirement(&mut self) -> Result<RequirementId, Diagnostic> {
        Ok(RequirementId::allocate(
            &self.seed,
            Self::next(&mut self.requirement)?,
        ))
    }

    fn port(&mut self) -> Result<PortId, Diagnostic> {
        Ok(PortId::allocate(&self.seed, Self::next(&mut self.port)?))
    }
}

fn import_declaration_identity(
    declaration: &Declaration,
    allocator: &mut impl IdentityAllocation,
) -> Result<DeclarationIdentity, Diagnostic> {
    let mut members = Vec::new();
    let mut bindings = Vec::new();
    let mut expressions = Vec::new();
    match declaration {
        Declaration::Record(record) => {
            for field in &record.fields {
                members.push(MemberIdentity::Field {
                    id: allocator.field()?,
                    name: field.name.clone(),
                });
            }
        }
        Declaration::Variant(variant) => {
            for case in &variant.cases {
                members.push(MemberIdentity::Case {
                    id: allocator.case()?,
                    name: case.name.clone(),
                });
            }
        }
        Declaration::Interface(interface) => {
            for operation in &interface.operations {
                members.push(MemberIdentity::Operation {
                    id: allocator.operation()?,
                    name: operation.name.clone(),
                });
                for parameter in &operation.parameters {
                    members.push(MemberIdentity::Parameter {
                        id: allocator.parameter()?,
                        name: parameter.name.clone(),
                    });
                }
            }
        }
        Declaration::External(function) => {
            for type_parameter in &function.type_parameters {
                members.push(MemberIdentity::TypeParameter {
                    id: allocator.type_parameter()?,
                    name: type_parameter.name.clone(),
                });
            }
            for parameter in &function.parameters {
                members.push(MemberIdentity::Parameter {
                    id: allocator.parameter()?,
                    name: parameter.name.clone(),
                });
            }
        }
        Declaration::Function(function) => {
            for type_parameter in &function.type_parameters {
                members.push(MemberIdentity::TypeParameter {
                    id: allocator.type_parameter()?,
                    name: type_parameter.name.clone(),
                });
            }
            for parameter in &function.parameters {
                members.push(MemberIdentity::Parameter {
                    id: allocator.parameter()?,
                    name: parameter.name.clone(),
                });
            }
            if let Effect::Task { capabilities } = &function.effect {
                for capability in capabilities {
                    members.push(MemberIdentity::TaskRequirement {
                        id: allocator.requirement()?,
                        name: capability.alias.clone(),
                    });
                }
            }
            index_expression(
                &function.body,
                vec![0],
                allocator,
                &mut expressions,
                &mut bindings,
            )?;
        }
        Declaration::Constant(constant) => index_expression(
            &constant.value,
            vec![0],
            allocator,
            &mut expressions,
            &mut bindings,
        )?,
        Declaration::Component(component) => {
            for requirement in &component.requirements {
                members.push(MemberIdentity::ComponentRequirement {
                    id: allocator.requirement()?,
                    name: requirement.alias.clone(),
                });
            }
            for (index, port) in component.ports.iter().enumerate() {
                members.push(MemberIdentity::Port {
                    id: allocator.port()?,
                    name: port.name.clone(),
                });
                index_expression(
                    &port.value,
                    vec![u32::try_from(index).map_err(|_| expression_limit())?],
                    allocator,
                    &mut expressions,
                    &mut bindings,
                )?;
            }
        }
        Declaration::Test(test) => {
            index_expression(
                &test.actual,
                vec![0],
                allocator,
                &mut expressions,
                &mut bindings,
            )?;
            index_expression(
                &test.expected,
                vec![1],
                allocator,
                &mut expressions,
                &mut bindings,
            )?;
        }
    }
    Ok(DeclarationIdentity {
        id: allocator.declaration()?,
        name: declaration.name().to_owned(),
        kind: DeclarationKind::of(declaration),
        members,
        bindings,
        expressions,
    })
}

fn index_expression(
    expression: &Expression,
    path: Vec<u32>,
    allocator: &mut impl IdentityAllocation,
    output: &mut Vec<ExpressionIdentity>,
    bindings: &mut Vec<BindingIdentity>,
) -> Result<(), Diagnostic> {
    if path.len() > MAXIMUM_EXPRESSION_DEPTH {
        return Err(expression_limit());
    }
    output.push(ExpressionIdentity {
        id: allocator.expression()?,
        path: path.clone(),
        kind: expression_kind(expression),
    });
    match expression {
        Expression::If {
            condition,
            when_true,
            when_false,
            ..
        } => {
            index_child(condition, &path, 0, allocator, output, bindings)?;
            index_child(when_true, &path, 1, allocator, output, bindings)?;
            index_child(when_false, &path, 2, allocator, output, bindings)?;
        }
        Expression::Let {
            bindings: values,
            body,
            ..
        } => {
            for (index, binding) in values.iter().enumerate() {
                let slot = u32::try_from(index).map_err(|_| expression_limit())?;
                bindings.push(BindingIdentity {
                    id: allocator.binding()?,
                    name: binding.name.clone(),
                    expression_path: path.clone(),
                    slot,
                });
                index_child(&binding.value, &path, slot, allocator, output, bindings)?;
            }
            index_child(
                body,
                &path,
                u32::try_from(values.len()).map_err(|_| expression_limit())?,
                allocator,
                output,
                bindings,
            )?;
        }
        Expression::Do { expressions, .. }
        | Expression::List {
            items: expressions, ..
        } => {
            for (index, child) in expressions.iter().enumerate() {
                index_child(
                    child,
                    &path,
                    u32::try_from(index).map_err(|_| expression_limit())?,
                    allocator,
                    output,
                    bindings,
                )?;
            }
        }
        Expression::Call { arguments, .. } | Expression::Perform { arguments, .. } => {
            for (index, child) in arguments.iter().enumerate() {
                index_child(
                    child,
                    &path,
                    u32::try_from(index).map_err(|_| expression_limit())?,
                    allocator,
                    output,
                    bindings,
                )?;
            }
        }
        Expression::Invoke {
            callee, arguments, ..
        } => {
            index_child(callee, &path, 0, allocator, output, bindings)?;
            for (index, child) in arguments.iter().enumerate() {
                index_child(
                    child,
                    &path,
                    u32::try_from(index)
                        .map_err(|_| expression_limit())?
                        .checked_add(1)
                        .ok_or_else(expression_limit)?,
                    allocator,
                    output,
                    bindings,
                )?;
            }
        }
        Expression::Record { fields, .. } => {
            for (index, field) in fields.iter().enumerate() {
                index_child(
                    &field.value,
                    &path,
                    u32::try_from(index).map_err(|_| expression_limit())?,
                    allocator,
                    output,
                    bindings,
                )?;
            }
        }
        Expression::Variant { payload, .. } => {
            if let Some(payload) = payload {
                index_child(payload, &path, 0, allocator, output, bindings)?;
            }
        }
        Expression::Field { value, .. } => {
            index_child(value, &path, 0, allocator, output, bindings)?;
        }
        Expression::Map { entries, .. } => {
            for (index, entry) in entries.iter().enumerate() {
                let key = index.checked_mul(2).ok_or_else(expression_limit)?;
                let value = key.checked_add(1).ok_or_else(expression_limit)?;
                index_child(
                    &entry.key,
                    &path,
                    u32::try_from(key).map_err(|_| expression_limit())?,
                    allocator,
                    output,
                    bindings,
                )?;
                index_child(
                    &entry.value,
                    &path,
                    u32::try_from(value).map_err(|_| expression_limit())?,
                    allocator,
                    output,
                    bindings,
                )?;
            }
        }
        Expression::Match { value, arms, .. } => {
            index_child(value, &path, 0, allocator, output, bindings)?;
            for (index, arm) in arms.iter().enumerate() {
                let slot = u32::try_from(index).map_err(|_| expression_limit())?;
                if let Some(binding) = &arm.binding {
                    bindings.push(BindingIdentity {
                        id: allocator.binding()?,
                        name: binding.clone(),
                        expression_path: path.clone(),
                        slot,
                    });
                }
                index_child(
                    &arm.body,
                    &path,
                    slot.checked_add(1).ok_or_else(expression_limit)?,
                    allocator,
                    output,
                    bindings,
                )?;
            }
        }
        Expression::Transaction { binding, body, .. } => {
            bindings.push(BindingIdentity {
                id: allocator.binding()?,
                name: binding.clone(),
                expression_path: path.clone(),
                slot: 0,
            });
            index_child(body, &path, 0, allocator, output, bindings)?;
        }
        Expression::Unit(_)
        | Expression::Bool(_, _)
        | Expression::I64(_, _)
        | Expression::Text(_, _)
        | Expression::StaticText(_, _)
        | Expression::Variable(_, _)
        | Expression::Constant(_, _)
        | Expression::FunctionRef { .. } => {}
    }
    Ok(())
}

fn index_child(
    expression: &Expression,
    parent: &[u32],
    ordinal: u32,
    allocator: &mut impl IdentityAllocation,
    output: &mut Vec<ExpressionIdentity>,
    bindings: &mut Vec<BindingIdentity>,
) -> Result<(), Diagnostic> {
    let mut path = parent.to_vec();
    path.push(ordinal);
    index_expression(expression, path, allocator, output, bindings)
}

fn expression_kind(expression: &Expression) -> ExpressionKind {
    match expression {
        Expression::Unit(_) => ExpressionKind::Unit,
        Expression::Bool(_, _) => ExpressionKind::Bool,
        Expression::I64(_, _) => ExpressionKind::I64,
        Expression::Text(_, _) => ExpressionKind::Text,
        Expression::StaticText(_, _) => ExpressionKind::StaticText,
        Expression::Variable(_, _) => ExpressionKind::Variable,
        Expression::Constant(_, _) => ExpressionKind::ConstantReference,
        Expression::If { .. } => ExpressionKind::If,
        Expression::Let { .. } => ExpressionKind::Let,
        Expression::Do { .. } => ExpressionKind::Do,
        Expression::Call { .. } => ExpressionKind::Call,
        Expression::Invoke { .. } => ExpressionKind::Invoke,
        Expression::Record { .. } => ExpressionKind::Record,
        Expression::Variant { .. } => ExpressionKind::Variant,
        Expression::Field { .. } => ExpressionKind::Field,
        Expression::List { .. } => ExpressionKind::List,
        Expression::Map { .. } => ExpressionKind::Map,
        Expression::Match { .. } => ExpressionKind::Match,
        Expression::FunctionRef { .. } => ExpressionKind::FunctionReference,
        Expression::Perform { .. } => ExpressionKind::CapabilityCall,
        Expression::Transaction { .. } => ExpressionKind::Transaction,
    }
}

fn validate_member_shape(
    identity: &DeclarationIdentity,
    declaration: &Declaration,
    all_identity_bytes: &mut BTreeSet<(&'static str, [u8; 16])>,
) -> Result<(), Diagnostic> {
    let expected = match declaration {
        Declaration::Record(record) => record
            .fields
            .iter()
            .map(|field| ("field", field.name.as_str()))
            .collect::<Vec<_>>(),
        Declaration::Variant(variant) => variant
            .cases
            .iter()
            .map(|case| ("case", case.name.as_str()))
            .collect::<Vec<_>>(),
        Declaration::Interface(interface) => {
            let mut expected = Vec::new();
            for operation in &interface.operations {
                expected.push(("operation", operation.name.as_str()));
                expected.extend(
                    operation
                        .parameters
                        .iter()
                        .map(|parameter| ("parameter", parameter.name.as_str())),
                );
            }
            expected
        }
        Declaration::External(function) => function
            .type_parameters
            .iter()
            .map(|parameter| ("type_parameter", parameter.name.as_str()))
            .chain(
                function
                    .parameters
                    .iter()
                    .map(|parameter| ("parameter", parameter.name.as_str())),
            )
            .collect(),
        Declaration::Function(function) => {
            let mut expected = function
                .type_parameters
                .iter()
                .map(|parameter| ("type_parameter", parameter.name.as_str()))
                .collect::<Vec<_>>();
            expected.extend(
                function
                    .parameters
                    .iter()
                    .map(|parameter| ("parameter", parameter.name.as_str())),
            );
            if let Effect::Task { capabilities } = &function.effect {
                expected.extend(
                    capabilities
                        .iter()
                        .map(|capability| ("task_requirement", capability.alias.as_str())),
                );
            }
            expected
        }
        Declaration::Component(component) => component
            .requirements
            .iter()
            .map(|requirement| ("component_requirement", requirement.alias.as_str()))
            .chain(
                component
                    .ports
                    .iter()
                    .map(|port| ("port", port.name.as_str())),
            )
            .collect(),
        Declaration::Constant(_) | Declaration::Test(_) => Vec::new(),
    };
    if expected.len() != identity.members.len()
        || expected
            .iter()
            .zip(&identity.members)
            .any(|((kind, name), member)| *name != member.name() || *kind != member_kind(member))
    {
        return Err(meaning_error(
            DiagnosticClass::Corrupt,
            "meaning_member_identity",
            format!(
                "member identities for declaration '{}' do not match its semantic members",
                identity.name
            ),
        ));
    }
    for member in &identity.members {
        let (domain, bytes) = member_domain_bytes(member);
        if !all_identity_bytes.insert((domain, bytes)) {
            return Err(meaning_error(
                DiagnosticClass::Corrupt,
                "meaning_member_identity_duplicate",
                "semantic member identity is duplicated",
            ));
        }
    }
    Ok(())
}

fn member_kind(member: &MemberIdentity) -> &'static str {
    match member {
        MemberIdentity::TypeParameter { .. } => "type_parameter",
        MemberIdentity::Field { .. } => "field",
        MemberIdentity::Case { .. } => "case",
        MemberIdentity::Operation { .. } => "operation",
        MemberIdentity::Parameter { .. } => "parameter",
        MemberIdentity::TaskRequirement { .. } => "task_requirement",
        MemberIdentity::ComponentRequirement { .. } => "component_requirement",
        MemberIdentity::Port { .. } => "port",
    }
}

fn member_domain_bytes(member: &MemberIdentity) -> (&'static str, [u8; 16]) {
    match member {
        MemberIdentity::TypeParameter { id, .. } => ("type_parameter", id.bytes()),
        MemberIdentity::Field { id, .. } => ("field", id.bytes()),
        MemberIdentity::Case { id, .. } => ("case", id.bytes()),
        MemberIdentity::Operation { id, .. } => ("operation", id.bytes()),
        MemberIdentity::Parameter { id, .. } => ("parameter", id.bytes()),
        MemberIdentity::TaskRequirement { id, .. }
        | MemberIdentity::ComponentRequirement { id, .. } => ("requirement", id.bytes()),
        MemberIdentity::Port { id, .. } => ("port", id.bytes()),
    }
}

fn validate_expression_shape(
    identity: &DeclarationIdentity,
    declaration: &Declaration,
) -> Result<(), Diagnostic> {
    let mut expected_expressions = Vec::new();
    let mut expected_bindings = Vec::new();
    collect_expected_expression_shape(
        declaration,
        &mut expected_expressions,
        &mut expected_bindings,
    )?;
    if expected_expressions.len() != identity.expressions.len()
        || expected_expressions
            .iter()
            .zip(&identity.expressions)
            .any(|((path, kind), actual)| *path != actual.path || *kind != actual.kind)
        || expected_bindings.len() != identity.bindings.len()
        || expected_bindings
            .iter()
            .zip(&identity.bindings)
            .any(|((name, path, slot), actual)| {
                *name != actual.name || *path != actual.expression_path || *slot != actual.slot
            })
    {
        return Err(meaning_error(
            DiagnosticClass::Corrupt,
            "meaning_expression_identity",
            format!(
                "expression or binding identities for declaration '{}' do not match its semantic body",
                identity.name
            ),
        ));
    }
    let unique_expressions = identity
        .expressions
        .iter()
        .map(|value| value.id)
        .collect::<BTreeSet<_>>();
    let unique_bindings = identity
        .bindings
        .iter()
        .map(|value| value.id)
        .collect::<BTreeSet<_>>();
    if unique_expressions.len() != identity.expressions.len()
        || unique_bindings.len() != identity.bindings.len()
    {
        return Err(meaning_error(
            DiagnosticClass::Corrupt,
            "meaning_local_identity_duplicate",
            "expression or binding identity is duplicated",
        ));
    }
    Ok(())
}

type ExpectedExpression = (Vec<u32>, ExpressionKind);
type ExpectedBinding = (String, Vec<u32>, u32);

fn collect_expected_expression_shape(
    declaration: &Declaration,
    expressions: &mut Vec<ExpectedExpression>,
    bindings: &mut Vec<ExpectedBinding>,
) -> Result<(), Diagnostic> {
    let mut allocator = MigrationIdentityAllocator::new(b"shape".to_vec());
    let mut indexed_expressions = Vec::new();
    let mut indexed_bindings = Vec::new();
    match declaration {
        Declaration::Function(function) => index_expression(
            &function.body,
            vec![0],
            &mut allocator,
            &mut indexed_expressions,
            &mut indexed_bindings,
        )?,
        Declaration::Constant(constant) => index_expression(
            &constant.value,
            vec![0],
            &mut allocator,
            &mut indexed_expressions,
            &mut indexed_bindings,
        )?,
        Declaration::Component(component) => {
            for (index, port) in component.ports.iter().enumerate() {
                index_expression(
                    &port.value,
                    vec![u32::try_from(index).map_err(|_| expression_limit())?],
                    &mut allocator,
                    &mut indexed_expressions,
                    &mut indexed_bindings,
                )?;
            }
        }
        Declaration::Test(test) => {
            index_expression(
                &test.actual,
                vec![0],
                &mut allocator,
                &mut indexed_expressions,
                &mut indexed_bindings,
            )?;
            index_expression(
                &test.expected,
                vec![1],
                &mut allocator,
                &mut indexed_expressions,
                &mut indexed_bindings,
            )?;
        }
        Declaration::Record(_)
        | Declaration::Variant(_)
        | Declaration::Interface(_)
        | Declaration::External(_) => {}
    }
    expressions.extend(
        indexed_expressions
            .into_iter()
            .map(|value| (value.path, value.kind)),
    );
    bindings.extend(
        indexed_bindings
            .into_iter()
            .map(|value| (value.name, value.expression_path, value.slot)),
    );
    Ok(())
}

pub(crate) fn normalize_module_spans(module: &mut Module) {
    for import in &mut module.imports {
        import.span = semantic_span();
    }
    for declaration in &mut module.declarations {
        normalize_declaration_spans(declaration);
    }
}

fn normalize_declaration_spans(declaration: &mut Declaration) {
    match declaration {
        Declaration::Record(record) => {
            record.span = semantic_span();
            for field in &mut record.fields {
                field.span = semantic_span();
            }
        }
        Declaration::Variant(variant) => {
            variant.span = semantic_span();
            for case in &mut variant.cases {
                case.span = semantic_span();
            }
        }
        Declaration::Interface(interface) => {
            interface.span = semantic_span();
            for operation in &mut interface.operations {
                operation.span = semantic_span();
                for parameter in &mut operation.parameters {
                    parameter.span = semantic_span();
                }
            }
        }
        Declaration::External(function) => {
            function.span = semantic_span();
            for type_parameter in &mut function.type_parameters {
                type_parameter.span = semantic_span();
            }
            for parameter in &mut function.parameters {
                parameter.span = semantic_span();
            }
        }
        Declaration::Function(function) => {
            function.span = semantic_span();
            for type_parameter in &mut function.type_parameters {
                type_parameter.span = semantic_span();
            }
            for parameter in &mut function.parameters {
                parameter.span = semantic_span();
            }
            if let Effect::Task { capabilities } = &mut function.effect {
                for capability in capabilities {
                    capability.span = semantic_span();
                }
            }
            normalize_expression_spans(&mut function.body);
        }
        Declaration::Constant(constant) => {
            constant.span = semantic_span();
            normalize_expression_spans(&mut constant.value);
        }
        Declaration::Component(component) => {
            component.span = semantic_span();
            for requirement in &mut component.requirements {
                requirement.span = semantic_span();
            }
            for port in &mut component.ports {
                port.span = semantic_span();
                normalize_expression_spans(&mut port.value);
            }
        }
        Declaration::Test(test) => {
            test.span = semantic_span();
            normalize_expression_spans(&mut test.actual);
            normalize_expression_spans(&mut test.expected);
        }
    }
}

fn normalize_expression_spans(expression: &mut Expression) {
    match expression {
        Expression::Unit(span)
        | Expression::Bool(_, span)
        | Expression::I64(_, span)
        | Expression::Text(_, span)
        | Expression::StaticText(_, span)
        | Expression::Variable(_, span)
        | Expression::Constant(_, span) => *span = semantic_span(),
        Expression::If {
            condition,
            when_true,
            when_false,
            span,
        } => {
            *span = semantic_span();
            normalize_expression_spans(condition);
            normalize_expression_spans(when_true);
            normalize_expression_spans(when_false);
        }
        Expression::Let {
            bindings,
            body,
            span,
            ..
        } => {
            *span = semantic_span();
            for binding in bindings {
                binding.span = semantic_span();
                normalize_expression_spans(&mut binding.value);
            }
            normalize_expression_spans(body);
        }
        Expression::Do { expressions, span }
        | Expression::List {
            items: expressions,
            span,
            ..
        } => {
            *span = semantic_span();
            for expression in expressions {
                normalize_expression_spans(expression);
            }
        }
        Expression::Call {
            arguments, span, ..
        }
        | Expression::Perform {
            arguments, span, ..
        } => {
            *span = semantic_span();
            for argument in arguments {
                normalize_expression_spans(argument);
            }
        }
        Expression::Invoke {
            callee,
            arguments,
            span,
        } => {
            *span = semantic_span();
            normalize_expression_spans(callee);
            for argument in arguments {
                normalize_expression_spans(argument);
            }
        }
        Expression::Record { fields, span, .. } => {
            *span = semantic_span();
            for field in fields {
                field.span = semantic_span();
                normalize_expression_spans(&mut field.value);
            }
        }
        Expression::Variant { payload, span, .. } => {
            *span = semantic_span();
            if let Some(payload) = payload {
                normalize_expression_spans(payload);
            }
        }
        Expression::Field { value, span, .. } => {
            *span = semantic_span();
            normalize_expression_spans(value);
        }
        Expression::Map { entries, span, .. } => {
            *span = semantic_span();
            for entry in entries {
                entry.span = semantic_span();
                normalize_expression_spans(&mut entry.key);
                normalize_expression_spans(&mut entry.value);
            }
        }
        Expression::Match { value, arms, span } => {
            *span = semantic_span();
            normalize_expression_spans(value);
            for arm in arms {
                arm.span = semantic_span();
                normalize_expression_spans(&mut arm.body);
            }
        }
        Expression::FunctionRef { span, .. } => *span = semantic_span(),
        Expression::Transaction { body, span, .. } => {
            *span = semantic_span();
            normalize_expression_spans(body);
        }
    }
}

fn semantic_span() -> SourceSpan {
    SourceSpan {
        byte_start: 0,
        byte_end: 0,
        line: 1,
        column: 1,
    }
}

fn expression_limit() -> Diagnostic {
    meaning_error(
        DiagnosticClass::Resource,
        "meaning_expression_limit",
        "expression identity path exceeds its checked representation",
    )
}

fn meaning_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{SourceLimits, parse_module, parse_source};

    #[test]
    fn imported_module_has_stable_domains_and_packed_round_trip() {
        let document = parse_source(
            "fixture.lkj",
            b"(module sample (record Item (name Text)) (fn read ((item Item)) Text (field item name)))\n",
            SourceLimits::default(),
        )
        .expect("source");
        let module = parse_module(&document).expect("module");
        let mut allocator = MigrationIdentityAllocator::new(b"package".to_vec());
        let meaning = MeaningModule::import(module, &mut allocator).expect("meaning");
        assert_eq!(meaning.declarations.len(), 2);
        assert_eq!(meaning.declarations[1].expressions.len(), 2);
        let bytes = meaning.encode().expect("encode");
        assert_eq!(MeaningModule::decode(&bytes).expect("decode"), meaning);
        assert_eq!(bytes, meaning.encode().expect("repeat"));
    }

    #[test]
    fn identity_shape_rejects_cross_record_drift() {
        let document = parse_source(
            "fixture.lkj",
            b"(module sample (record Item (name Text)))\n",
            SourceLimits::default(),
        )
        .expect("source");
        let module = parse_module(&document).expect("module");
        let mut allocator = MigrationIdentityAllocator::new(b"package".to_vec());
        let mut meaning = MeaningModule::import(module, &mut allocator).expect("meaning");
        meaning.declarations[0].name = "Other".to_owned();
        assert_eq!(
            meaning.validate_identity_shape().expect_err("drift").code,
            "meaning_declaration_identity"
        );
    }

    #[test]
    fn canonical_module_rejects_nonsemantic_source_coordinates() {
        let document = parse_source(
            "fixture.lkj",
            b"(module sample (record Item (name Text)))\n",
            SourceLimits::default(),
        )
        .expect("source oracle");
        let module = parse_module(&document).expect("module oracle");
        let mut allocator = MigrationIdentityAllocator::new(b"coordinate-rejection".to_vec());
        let mut meaning = MeaningModule::import(module, &mut allocator).expect("meaning");
        let Declaration::Record(record) = &mut meaning.module.declarations[0] else {
            panic!("fixture record");
        };
        record.span.byte_start = 7;
        assert_eq!(
            meaning.encode().expect_err("coordinate rejects").code,
            "meaning_source_coordinate"
        );
    }
}
