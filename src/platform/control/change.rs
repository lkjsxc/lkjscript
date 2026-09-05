//! Closed compact-record adapter for normalized semantic changes.

use super::{CompactField, CompactRecord, parse_records};
use crate::platform::change::{
    AuthoredBindingDefinition, AuthoredCase, AuthoredCaseReference, AuthoredChange,
    AuthoredChangeSet, AuthoredDeclarationReference, AuthoredDeletePolicy, AuthoredExpression,
    AuthoredExpressionOperation, AuthoredField, AuthoredFieldReference, AuthoredFieldSelector,
    AuthoredFunctionEffect, AuthoredLetBinding, AuthoredLocalReference, AuthoredMapExpressionEntry,
    AuthoredMatchExpressionArm, AuthoredOperationReference, AuthoredOwnerParent, AuthoredParameter,
    AuthoredPort, AuthoredPortImplementation, AuthoredPortReference, AuthoredPrecondition,
    AuthoredRecordExpressionField, AuthoredRequirement, AuthoredRequirementReference,
    AuthoredResourceLimit, AuthoredStructuralTypeField, AuthoredType, AuthoredTypeParameter,
    AuthoredTypeParameterReference, DeclarationSelector, ModuleSelector, OwnerSelector,
    ParameterParentSelector,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass, SourceLocation};
use crate::platform::kernel::{
    DeclarationVisibility, ExternalVisibility, HttpRouteSelector, Idempotency, ImplementationName,
    Name, NamespaceClass, OwnerKey, PackageId, PackageRevisionDigest, ParameterUse, ResourceUnit,
};
use crate::platform::package::RunnerKind;
use crate::platform::publication::{PublicationOptions, idempotency_key_is_valid};
use crate::platform::semantic_id::{
    BindingId, CaseId, DeclarationId, ExpressionId, FieldId, ModuleId, OperationId, ParameterId,
    PortId, RequirementId, RevisionId,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;

pub const COMPACT_CHANGE_CONTRACT_IDENTITY: &str = "lkjscript-change-records-14";
pub const COMPACT_CHANGE_CONTRACT_VERSION: u16 = 14;
pub const AUTHORED_CHANGE_CODEC_IDENTITY: &str = "lkjscript-authored-change-codec-11";
pub const AUTHORED_CHANGE_CODEC_VERSION: u16 = 11;
pub const CHANGE_REQUEST_COMMITMENT_DOMAIN: &str = "lkjscript.change-request-commitment.v1";
pub const COMPACT_DELETE_POLICIES: &[&str] = &["reject", "owned-closure"];
pub(crate) const COMPACT_DECLARATION_VISIBILITIES: &[(&str, DeclarationVisibility)] = &[
    ("private", DeclarationVisibility::Private),
    ("package", DeclarationVisibility::Package),
    ("public", DeclarationVisibility::Public),
];
pub(crate) const COMPACT_FUNCTION_EFFECTS: &[&str] = &["pure", "task"];
pub const COMPACT_CHANGE_PRECONDITIONS: &[&str] = &[
    "precondition.owner-exists",
    "precondition.owner-absent",
    "precondition.owner-name",
    "precondition.owner-parent",
    "precondition.namespace-absent",
    "precondition.namespace-points-to",
    "precondition.dependency-binding",
];
pub(crate) const COMPACT_NAMESPACE_CLASSES: &[(&str, NamespaceClass)] = &[
    ("module", NamespaceClass::Module),
    ("declaration", NamespaceClass::Declaration),
    ("type-parameter", NamespaceClass::TypeParameter),
    ("field", NamespaceClass::Field),
    ("case", NamespaceClass::Case),
    ("operation", NamespaceClass::Operation),
    ("parameter", NamespaceClass::Parameter),
    ("requirement", NamespaceClass::Requirement),
    ("port", NamespaceClass::Port),
    ("target", NamespaceClass::Target),
];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum CompactChangeOperation {
    CreateModule,
    CreateRecord,
    CreateVariant,
    CreateInterface,
    CreateExternal,
    CreateFunction,
    CreateConstant,
    CreateComponent,
    CreateTest,
    CreateTarget,
    AddField,
    AddCase,
    AddOperation,
    AddTypeParameter,
    AddParameter,
    AddRequirement,
    AddPort,
    AddHttpRoute,
    AddDependency,
    ReplaceDependency,
    SetFunctionContract,
    SetRequirementContract,
    SetHttpRoute,
    DeleteOwner,
    RenameOwner,
    MoveDeclaration,
    ReplaceBody,
    ExtractFunction,
}

impl CompactChangeOperation {
    pub(crate) const ALL: [Self; 28] = [
        Self::CreateModule,
        Self::CreateRecord,
        Self::CreateVariant,
        Self::CreateInterface,
        Self::CreateExternal,
        Self::CreateFunction,
        Self::CreateConstant,
        Self::CreateComponent,
        Self::CreateTest,
        Self::CreateTarget,
        Self::AddField,
        Self::AddCase,
        Self::AddOperation,
        Self::AddTypeParameter,
        Self::AddParameter,
        Self::AddRequirement,
        Self::AddPort,
        Self::AddHttpRoute,
        Self::AddDependency,
        Self::ReplaceDependency,
        Self::SetFunctionContract,
        Self::SetRequirementContract,
        Self::SetHttpRoute,
        Self::DeleteOwner,
        Self::RenameOwner,
        Self::MoveDeclaration,
        Self::ReplaceBody,
        Self::ExtractFunction,
    ];
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum CompactChangeFieldForm {
    RequestLocalSymbol,
    ModuleSelector,
    DeclarationSelector,
    OwnerSelector,
    ExactOwner,
    Name,
    DeclarationVisibility,
    FunctionEffect,
    TypeReference,
    ExpressionReference,
    DeletePolicy,
    OwnerParent,
    NamespaceClass,
    ExactPackage,
    ExactRevision,
    ExactPackageRevision,
    RequestFragment,
    DeclarationReference,
    PortReference,
    RunnerKind,
    OperationSelector,
    Idempotency,
    ExternalVisibility,
    ParameterUse,
    RequirementReference,
    ImplementationName,
    ExactExpression,
    HttpMethod,
    HttpPath,
    HttpPattern,
}

impl CompactChangeFieldForm {
    pub(crate) const ALL: [Self; 30] = [
        Self::RequestLocalSymbol,
        Self::ModuleSelector,
        Self::DeclarationSelector,
        Self::OwnerSelector,
        Self::ExactOwner,
        Self::Name,
        Self::DeclarationVisibility,
        Self::FunctionEffect,
        Self::TypeReference,
        Self::ExpressionReference,
        Self::DeletePolicy,
        Self::OwnerParent,
        Self::NamespaceClass,
        Self::ExactPackage,
        Self::ExactRevision,
        Self::ExactPackageRevision,
        Self::RequestFragment,
        Self::DeclarationReference,
        Self::PortReference,
        Self::RunnerKind,
        Self::OperationSelector,
        Self::Idempotency,
        Self::ExternalVisibility,
        Self::ParameterUse,
        Self::RequirementReference,
        Self::ImplementationName,
        Self::ExactExpression,
        Self::HttpMethod,
        Self::HttpPath,
        Self::HttpPattern,
    ];

    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::RequestLocalSymbol => "request_local_symbol",
            Self::ModuleSelector => "module_selector",
            Self::DeclarationSelector => "declaration_selector",
            Self::OwnerSelector => "owner_selector",
            Self::ExactOwner => "exact_owner",
            Self::Name => "name",
            Self::DeclarationVisibility => "declaration_visibility",
            Self::FunctionEffect => "function_effect",
            Self::TypeReference => "type_reference",
            Self::ExpressionReference => "expression_reference",
            Self::DeletePolicy => "delete_policy",
            Self::OwnerParent => "owner_parent",
            Self::NamespaceClass => "namespace_class",
            Self::ExactPackage => "exact_package",
            Self::ExactRevision => "exact_revision",
            Self::ExactPackageRevision => "exact_package_revision",
            Self::RequestFragment => "request_fragment",
            Self::DeclarationReference => "declaration_reference",
            Self::PortReference => "port_reference",
            Self::RunnerKind => "runner_kind",
            Self::OperationSelector => "operation_selector",
            Self::Idempotency => "idempotency",
            Self::ExternalVisibility => "external_visibility",
            Self::ParameterUse => "parameter_use",
            Self::RequirementReference => "requirement_reference",
            Self::ImplementationName => "implementation_name",
            Self::ExactExpression => "exact_expression",
            Self::HttpMethod => "http_method",
            Self::HttpPath => "http_path",
            Self::HttpPattern => "http_pattern",
        }
    }

    pub(crate) const fn syntax(self) -> &'static str {
        match self {
            Self::RequestLocalSymbol => "$NAME",
            Self::ModuleSelector => "$NAME|mod_HEX|MODULE_NAME",
            Self::DeclarationSelector => "$NAME|decl_HEX|MODULE/NAME",
            Self::OwnerSelector => "$NAME|DOMAIN_HEX",
            Self::ExactOwner => "DOMAIN_HEX",
            Self::Name => "[A-Za-z_][A-Za-z0-9_-]{0,127}",
            Self::DeclarationVisibility => "private|package|public",
            Self::FunctionEffect => "pure|task",
            Self::TypeReference => "unit|bool|i64|bytes|text|static-text|secret|@NAME",
            Self::ExpressionReference => "$NAME",
            Self::DeletePolicy => "reject|owned-closure",
            Self::OwnerParent => "package|DOMAIN_HEX",
            Self::NamespaceClass => "change.namespace-class.name",
            Self::ExactPackage => "pkg_HEX",
            Self::ExactRevision => "rev_HEX",
            Self::ExactPackageRevision => "package_revision_HEX",
            Self::RequestFragment => "%NAME",
            Self::DeclarationReference => "$NAME|decl_HEX|MODULE/NAME|pkg_HEX/decl_HEX",
            Self::PortReference => "$NAME|pkg_HEX/port_HEX",
            Self::RunnerKind => "command|http|interactive",
            Self::OperationSelector => "$NAME|op_HEX",
            Self::Idempotency => "idempotent|idempotent-with-key|non-idempotent",
            Self::ExternalVisibility => "none|possible",
            Self::ParameterUse => "unrestricted|borrow|consume",
            Self::RequirementReference => "$NAME|pkg_HEX/req_HEX",
            Self::ImplementationName => "dot.separated.name",
            Self::ExactExpression => "expr_HEX",
            Self::HttpMethod => "ASCII_HTTP_TOKEN_1_TO_32_BYTES",
            Self::HttpPath => "/EXACT_PATH_1_TO_16384_BYTES",
            Self::HttpPattern => "/LITERAL/{capture}/..._1_TO_64_SEGMENTS",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CompactChangeOperationField {
    pub(crate) name: &'static str,
    pub(crate) required: bool,
    pub(crate) form: CompactChangeFieldForm,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CompactChangeDirectOperation {
    pub(crate) plan_usage: &'static str,
    pub(crate) apply_usage: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CompactChangeOperationDescriptor {
    pub(crate) operation: CompactChangeOperation,
    pub(crate) name: &'static str,
    pub(crate) fields: &'static [CompactChangeOperationField],
    pub(crate) direct: Option<CompactChangeDirectOperation>,
}

use CompactChangeFieldForm as FieldForm;

pub(crate) const COMPACT_CHANGE_OPERATION_DESCRIPTORS: &[CompactChangeOperationDescriptor] = &[
    CompactChangeOperationDescriptor {
        operation: CompactChangeOperation::CreateModule,
        name: "create.module",
        fields: &[
            CompactChangeOperationField {
                name: "as",
                required: true,
                form: FieldForm::RequestLocalSymbol,
            },
            CompactChangeOperationField {
                name: "name",
                required: true,
                form: FieldForm::Name,
            },
        ],
        direct: None,
    },
    CompactChangeOperationDescriptor {
        operation: CompactChangeOperation::CreateRecord,
        name: "create.record",
        fields: &[
            CompactChangeOperationField {
                name: "as",
                required: true,
                form: FieldForm::RequestLocalSymbol,
            },
            CompactChangeOperationField {
                name: "module",
                required: true,
                form: FieldForm::ModuleSelector,
            },
            CompactChangeOperationField {
                name: "name",
                required: true,
                form: FieldForm::Name,
            },
            CompactChangeOperationField {
                name: "visibility",
                required: true,
                form: FieldForm::DeclarationVisibility,
            },
        ],
        direct: None,
    },
    CompactChangeOperationDescriptor {
        operation: CompactChangeOperation::CreateVariant,
        name: "create.variant",
        fields: &[
            CompactChangeOperationField {
                name: "as",
                required: true,
                form: FieldForm::RequestLocalSymbol,
            },
            CompactChangeOperationField {
                name: "module",
                required: true,
                form: FieldForm::ModuleSelector,
            },
            CompactChangeOperationField {
                name: "name",
                required: true,
                form: FieldForm::Name,
            },
            CompactChangeOperationField {
                name: "visibility",
                required: true,
                form: FieldForm::DeclarationVisibility,
            },
        ],
        direct: None,
    },
    CompactChangeOperationDescriptor {
        operation: CompactChangeOperation::CreateInterface,
        name: "create.interface",
        fields: &[
            CompactChangeOperationField {
                name: "as",
                required: true,
                form: FieldForm::RequestLocalSymbol,
            },
            CompactChangeOperationField {
                name: "module",
                required: true,
                form: FieldForm::ModuleSelector,
            },
            CompactChangeOperationField {
                name: "name",
                required: true,
                form: FieldForm::Name,
            },
            CompactChangeOperationField {
                name: "visibility",
                required: true,
                form: FieldForm::DeclarationVisibility,
            },
        ],
        direct: None,
    },
    CompactChangeOperationDescriptor {
        operation: CompactChangeOperation::CreateExternal,
        name: "create.external",
        fields: &[
            CompactChangeOperationField {
                name: "as",
                required: true,
                form: FieldForm::RequestLocalSymbol,
            },
            CompactChangeOperationField {
                name: "module",
                required: true,
                form: FieldForm::ModuleSelector,
            },
            CompactChangeOperationField {
                name: "name",
                required: true,
                form: FieldForm::Name,
            },
            CompactChangeOperationField {
                name: "visibility",
                required: true,
                form: FieldForm::DeclarationVisibility,
            },
            CompactChangeOperationField {
                name: "result",
                required: true,
                form: FieldForm::TypeReference,
            },
            CompactChangeOperationField {
                name: "implementation",
                required: true,
                form: FieldForm::ImplementationName,
            },
        ],
        direct: None,
    },
    CompactChangeOperationDescriptor {
        operation: CompactChangeOperation::CreateFunction,
        name: "create.function",
        fields: &[
            CompactChangeOperationField {
                name: "as",
                required: true,
                form: FieldForm::RequestLocalSymbol,
            },
            CompactChangeOperationField {
                name: "module",
                required: true,
                form: FieldForm::ModuleSelector,
            },
            CompactChangeOperationField {
                name: "name",
                required: true,
                form: FieldForm::Name,
            },
            CompactChangeOperationField {
                name: "visibility",
                required: true,
                form: FieldForm::DeclarationVisibility,
            },
            CompactChangeOperationField {
                name: "result",
                required: true,
                form: FieldForm::TypeReference,
            },
            CompactChangeOperationField {
                name: "effect",
                required: true,
                form: FieldForm::FunctionEffect,
            },
            CompactChangeOperationField {
                name: "body",
                required: true,
                form: FieldForm::ExpressionReference,
            },
        ],
        direct: None,
    },
    CompactChangeOperationDescriptor {
        operation: CompactChangeOperation::CreateConstant,
        name: "create.constant",
        fields: &[
            CompactChangeOperationField {
                name: "as",
                required: true,
                form: FieldForm::RequestLocalSymbol,
            },
            CompactChangeOperationField {
                name: "module",
                required: true,
                form: FieldForm::ModuleSelector,
            },
            CompactChangeOperationField {
                name: "name",
                required: true,
                form: FieldForm::Name,
            },
            CompactChangeOperationField {
                name: "visibility",
                required: true,
                form: FieldForm::DeclarationVisibility,
            },
            CompactChangeOperationField {
                name: "type",
                required: true,
                form: FieldForm::TypeReference,
            },
            CompactChangeOperationField {
                name: "value",
                required: true,
                form: FieldForm::ExpressionReference,
            },
        ],
        direct: None,
    },
    CompactChangeOperationDescriptor {
        operation: CompactChangeOperation::CreateComponent,
        name: "create.component",
        fields: &[
            CompactChangeOperationField {
                name: "as",
                required: true,
                form: FieldForm::RequestLocalSymbol,
            },
            CompactChangeOperationField {
                name: "module",
                required: true,
                form: FieldForm::ModuleSelector,
            },
            CompactChangeOperationField {
                name: "name",
                required: true,
                form: FieldForm::Name,
            },
            CompactChangeOperationField {
                name: "visibility",
                required: true,
                form: FieldForm::DeclarationVisibility,
            },
        ],
        direct: None,
    },
    CompactChangeOperationDescriptor {
        operation: CompactChangeOperation::CreateTest,
        name: "create.test",
        fields: &[
            CompactChangeOperationField {
                name: "as",
                required: true,
                form: FieldForm::RequestLocalSymbol,
            },
            CompactChangeOperationField {
                name: "module",
                required: true,
                form: FieldForm::ModuleSelector,
            },
            CompactChangeOperationField {
                name: "name",
                required: true,
                form: FieldForm::Name,
            },
            CompactChangeOperationField {
                name: "visibility",
                required: true,
                form: FieldForm::DeclarationVisibility,
            },
            CompactChangeOperationField {
                name: "actual",
                required: true,
                form: FieldForm::ExpressionReference,
            },
            CompactChangeOperationField {
                name: "expected",
                required: true,
                form: FieldForm::ExpressionReference,
            },
        ],
        direct: None,
    },
    CompactChangeOperationDescriptor {
        operation: CompactChangeOperation::CreateTarget,
        name: "create.target",
        fields: &[
            CompactChangeOperationField {
                name: "as",
                required: true,
                form: FieldForm::RequestLocalSymbol,
            },
            CompactChangeOperationField {
                name: "name",
                required: true,
                form: FieldForm::Name,
            },
            CompactChangeOperationField {
                name: "component",
                required: true,
                form: FieldForm::DeclarationReference,
            },
            CompactChangeOperationField {
                name: "port",
                required: false,
                form: FieldForm::PortReference,
            },
            CompactChangeOperationField {
                name: "runner",
                required: true,
                form: FieldForm::RunnerKind,
            },
        ],
        direct: None,
    },
    CompactChangeOperationDescriptor {
        operation: CompactChangeOperation::AddField,
        name: "add.field",
        fields: &[
            CompactChangeOperationField {
                name: "as",
                required: true,
                form: FieldForm::RequestLocalSymbol,
            },
            CompactChangeOperationField {
                name: "record",
                required: true,
                form: FieldForm::DeclarationSelector,
            },
            CompactChangeOperationField {
                name: "name",
                required: true,
                form: FieldForm::Name,
            },
            CompactChangeOperationField {
                name: "type",
                required: true,
                form: FieldForm::TypeReference,
            },
        ],
        direct: None,
    },
    CompactChangeOperationDescriptor {
        operation: CompactChangeOperation::AddCase,
        name: "add.case",
        fields: &[
            CompactChangeOperationField {
                name: "as",
                required: true,
                form: FieldForm::RequestLocalSymbol,
            },
            CompactChangeOperationField {
                name: "variant",
                required: true,
                form: FieldForm::DeclarationSelector,
            },
            CompactChangeOperationField {
                name: "name",
                required: true,
                form: FieldForm::Name,
            },
            CompactChangeOperationField {
                name: "payload",
                required: false,
                form: FieldForm::TypeReference,
            },
        ],
        direct: None,
    },
    CompactChangeOperationDescriptor {
        operation: CompactChangeOperation::AddOperation,
        name: "add.operation",
        fields: &[
            CompactChangeOperationField {
                name: "as",
                required: true,
                form: FieldForm::RequestLocalSymbol,
            },
            CompactChangeOperationField {
                name: "interface",
                required: true,
                form: FieldForm::DeclarationSelector,
            },
            CompactChangeOperationField {
                name: "name",
                required: true,
                form: FieldForm::Name,
            },
            CompactChangeOperationField {
                name: "result",
                required: true,
                form: FieldForm::TypeReference,
            },
            CompactChangeOperationField {
                name: "idempotency",
                required: true,
                form: FieldForm::Idempotency,
            },
            CompactChangeOperationField {
                name: "external-visibility",
                required: true,
                form: FieldForm::ExternalVisibility,
            },
        ],
        direct: None,
    },
    CompactChangeOperationDescriptor {
        operation: CompactChangeOperation::AddTypeParameter,
        name: "add.type-parameter",
        fields: &[
            CompactChangeOperationField {
                name: "as",
                required: true,
                form: FieldForm::RequestLocalSymbol,
            },
            CompactChangeOperationField {
                name: "function",
                required: true,
                form: FieldForm::DeclarationSelector,
            },
            CompactChangeOperationField {
                name: "name",
                required: true,
                form: FieldForm::Name,
            },
        ],
        direct: None,
    },
    CompactChangeOperationDescriptor {
        operation: CompactChangeOperation::AddParameter,
        name: "add.parameter",
        fields: &[
            CompactChangeOperationField {
                name: "as",
                required: true,
                form: FieldForm::RequestLocalSymbol,
            },
            CompactChangeOperationField {
                name: "function",
                required: false,
                form: FieldForm::DeclarationSelector,
            },
            CompactChangeOperationField {
                name: "operation",
                required: false,
                form: FieldForm::OperationSelector,
            },
            CompactChangeOperationField {
                name: "name",
                required: true,
                form: FieldForm::Name,
            },
            CompactChangeOperationField {
                name: "type",
                required: true,
                form: FieldForm::TypeReference,
            },
            CompactChangeOperationField {
                name: "use",
                required: false,
                form: FieldForm::ParameterUse,
            },
            CompactChangeOperationField {
                name: "requirement",
                required: false,
                form: FieldForm::RequirementReference,
            },
        ],
        direct: None,
    },
    CompactChangeOperationDescriptor {
        operation: CompactChangeOperation::AddRequirement,
        name: "add.requirement",
        fields: &[
            CompactChangeOperationField {
                name: "as",
                required: true,
                form: FieldForm::RequestLocalSymbol,
            },
            CompactChangeOperationField {
                name: "component",
                required: true,
                form: FieldForm::DeclarationSelector,
            },
            CompactChangeOperationField {
                name: "name",
                required: true,
                form: FieldForm::Name,
            },
            CompactChangeOperationField {
                name: "interface",
                required: true,
                form: FieldForm::DeclarationReference,
            },
        ],
        direct: None,
    },
    CompactChangeOperationDescriptor {
        operation: CompactChangeOperation::AddPort,
        name: "add.port",
        fields: &[
            CompactChangeOperationField {
                name: "as",
                required: true,
                form: FieldForm::RequestLocalSymbol,
            },
            CompactChangeOperationField {
                name: "component",
                required: true,
                form: FieldForm::DeclarationSelector,
            },
            CompactChangeOperationField {
                name: "name",
                required: true,
                form: FieldForm::Name,
            },
            CompactChangeOperationField {
                name: "type",
                required: true,
                form: FieldForm::TypeReference,
            },
            CompactChangeOperationField {
                name: "function",
                required: true,
                form: FieldForm::DeclarationReference,
            },
        ],
        direct: None,
    },
    CompactChangeOperationDescriptor {
        operation: CompactChangeOperation::AddHttpRoute,
        name: "add.http-route",
        fields: &[
            CompactChangeOperationField {
                name: "as",
                required: true,
                form: FieldForm::RequestLocalSymbol,
            },
            CompactChangeOperationField {
                name: "target",
                required: true,
                form: FieldForm::OwnerSelector,
            },
            CompactChangeOperationField {
                name: "method",
                required: true,
                form: FieldForm::HttpMethod,
            },
            CompactChangeOperationField {
                name: "path",
                required: false,
                form: FieldForm::HttpPath,
            },
            CompactChangeOperationField {
                name: "pattern",
                required: false,
                form: FieldForm::HttpPattern,
            },
            CompactChangeOperationField {
                name: "port",
                required: true,
                form: FieldForm::PortReference,
            },
        ],
        direct: None,
    },
    CompactChangeOperationDescriptor {
        operation: CompactChangeOperation::AddDependency,
        name: "add.dependency",
        fields: &[
            CompactChangeOperationField {
                name: "package",
                required: true,
                form: FieldForm::ExactPackage,
            },
            CompactChangeOperationField {
                name: "semantic-revision",
                required: true,
                form: FieldForm::ExactRevision,
            },
            CompactChangeOperationField {
                name: "package-revision",
                required: true,
                form: FieldForm::ExactPackageRevision,
            },
        ],
        direct: None,
    },
    CompactChangeOperationDescriptor {
        operation: CompactChangeOperation::ReplaceDependency,
        name: "replace.dependency",
        fields: &[
            CompactChangeOperationField {
                name: "package",
                required: true,
                form: FieldForm::ExactPackage,
            },
            CompactChangeOperationField {
                name: "semantic-revision",
                required: true,
                form: FieldForm::ExactRevision,
            },
            CompactChangeOperationField {
                name: "package-revision",
                required: true,
                form: FieldForm::ExactPackageRevision,
            },
        ],
        direct: None,
    },
    CompactChangeOperationDescriptor {
        operation: CompactChangeOperation::SetFunctionContract,
        name: "set.function-contract",
        fields: &[
            CompactChangeOperationField {
                name: "as",
                required: true,
                form: FieldForm::RequestFragment,
            },
            CompactChangeOperationField {
                name: "function",
                required: true,
                form: FieldForm::DeclarationSelector,
            },
            CompactChangeOperationField {
                name: "result",
                required: true,
                form: FieldForm::TypeReference,
            },
            CompactChangeOperationField {
                name: "effect",
                required: true,
                form: FieldForm::FunctionEffect,
            },
        ],
        direct: None,
    },
    CompactChangeOperationDescriptor {
        operation: CompactChangeOperation::SetRequirementContract,
        name: "set.requirement-contract",
        fields: &[
            CompactChangeOperationField {
                name: "as",
                required: true,
                form: FieldForm::RequestFragment,
            },
            CompactChangeOperationField {
                name: "requirement",
                required: true,
                form: FieldForm::OwnerSelector,
            },
            CompactChangeOperationField {
                name: "interface",
                required: true,
                form: FieldForm::DeclarationReference,
            },
        ],
        direct: None,
    },
    CompactChangeOperationDescriptor {
        operation: CompactChangeOperation::SetHttpRoute,
        name: "set.http-route",
        fields: &[
            CompactChangeOperationField {
                name: "route",
                required: true,
                form: FieldForm::OwnerSelector,
            },
            CompactChangeOperationField {
                name: "method",
                required: true,
                form: FieldForm::HttpMethod,
            },
            CompactChangeOperationField {
                name: "path",
                required: false,
                form: FieldForm::HttpPath,
            },
            CompactChangeOperationField {
                name: "pattern",
                required: false,
                form: FieldForm::HttpPattern,
            },
            CompactChangeOperationField {
                name: "port",
                required: true,
                form: FieldForm::PortReference,
            },
        ],
        direct: None,
    },
    CompactChangeOperationDescriptor {
        operation: CompactChangeOperation::DeleteOwner,
        name: "delete.owner",
        fields: &[
            CompactChangeOperationField {
                name: "owner",
                required: true,
                form: FieldForm::ExactOwner,
            },
            CompactChangeOperationField {
                name: "policy",
                required: true,
                form: FieldForm::DeletePolicy,
            },
        ],
        direct: None,
    },
    CompactChangeOperationDescriptor {
        operation: CompactChangeOperation::RenameOwner,
        name: "rename.owner",
        fields: &[
            CompactChangeOperationField {
                name: "owner",
                required: true,
                form: FieldForm::OwnerSelector,
            },
            CompactChangeOperationField {
                name: "name",
                required: true,
                form: FieldForm::Name,
            },
        ],
        direct: Some(CompactChangeDirectOperation {
            plan_usage: "change plan rename.owner --base REVISION --owner OWNER --name NAME [--idempotency KEY] [--intent TEXT] [--output PATH]",
            apply_usage: "change apply rename.owner --base REVISION --owner OWNER --name NAME [--idempotency KEY] [--intent TEXT] --plan PLAN",
        }),
    },
    CompactChangeOperationDescriptor {
        operation: CompactChangeOperation::MoveDeclaration,
        name: "move.declaration",
        fields: &[
            CompactChangeOperationField {
                name: "declaration",
                required: true,
                form: FieldForm::DeclarationSelector,
            },
            CompactChangeOperationField {
                name: "module",
                required: true,
                form: FieldForm::ModuleSelector,
            },
        ],
        direct: None,
    },
    CompactChangeOperationDescriptor {
        operation: CompactChangeOperation::ReplaceBody,
        name: "replace.body",
        fields: &[
            CompactChangeOperationField {
                name: "function",
                required: true,
                form: FieldForm::DeclarationSelector,
            },
            CompactChangeOperationField {
                name: "body",
                required: true,
                form: FieldForm::ExpressionReference,
            },
        ],
        direct: None,
    },
    CompactChangeOperationDescriptor {
        operation: CompactChangeOperation::ExtractFunction,
        name: "extract.function",
        fields: &[
            CompactChangeOperationField {
                name: "as",
                required: true,
                form: FieldForm::RequestLocalSymbol,
            },
            CompactChangeOperationField {
                name: "function",
                required: true,
                form: FieldForm::DeclarationSelector,
            },
            CompactChangeOperationField {
                name: "expression",
                required: true,
                form: FieldForm::ExactExpression,
            },
            CompactChangeOperationField {
                name: "name",
                required: true,
                form: FieldForm::Name,
            },
        ],
        direct: Some(CompactChangeDirectOperation {
            plan_usage: "change plan extract.function --base REVISION --as SYMBOL --function FUNCTION --expression EXPRESSION --name NAME [--idempotency KEY] [--intent TEXT] [--output PATH]",
            apply_usage: "change apply extract.function --base REVISION --as SYMBOL --function FUNCTION --expression EXPRESSION --name NAME [--idempotency KEY] [--intent TEXT] --plan PLAN",
        }),
    },
];

pub(crate) fn compact_change_operation_descriptor(
    name: &str,
) -> Option<&'static CompactChangeOperationDescriptor> {
    COMPACT_CHANGE_OPERATION_DESCRIPTORS
        .iter()
        .find(|descriptor| descriptor.name == name)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CompactChangePreconditionField {
    pub(crate) record: &'static str,
    pub(crate) name: &'static str,
    pub(crate) required: bool,
    pub(crate) form: CompactChangeFieldForm,
}

pub(crate) const COMPACT_CHANGE_PRECONDITION_FIELDS: &[CompactChangePreconditionField] = &[
    CompactChangePreconditionField {
        record: "precondition.owner-exists",
        name: "owner",
        required: true,
        form: FieldForm::ExactOwner,
    },
    CompactChangePreconditionField {
        record: "precondition.owner-absent",
        name: "owner",
        required: true,
        form: FieldForm::ExactOwner,
    },
    CompactChangePreconditionField {
        record: "precondition.owner-name",
        name: "owner",
        required: true,
        form: FieldForm::ExactOwner,
    },
    CompactChangePreconditionField {
        record: "precondition.owner-name",
        name: "name",
        required: true,
        form: FieldForm::Name,
    },
    CompactChangePreconditionField {
        record: "precondition.owner-parent",
        name: "owner",
        required: true,
        form: FieldForm::ExactOwner,
    },
    CompactChangePreconditionField {
        record: "precondition.owner-parent",
        name: "parent",
        required: true,
        form: FieldForm::OwnerParent,
    },
    CompactChangePreconditionField {
        record: "precondition.namespace-absent",
        name: "parent",
        required: true,
        form: FieldForm::OwnerParent,
    },
    CompactChangePreconditionField {
        record: "precondition.namespace-absent",
        name: "class",
        required: true,
        form: FieldForm::NamespaceClass,
    },
    CompactChangePreconditionField {
        record: "precondition.namespace-absent",
        name: "name",
        required: true,
        form: FieldForm::Name,
    },
    CompactChangePreconditionField {
        record: "precondition.namespace-points-to",
        name: "parent",
        required: true,
        form: FieldForm::OwnerParent,
    },
    CompactChangePreconditionField {
        record: "precondition.namespace-points-to",
        name: "class",
        required: true,
        form: FieldForm::NamespaceClass,
    },
    CompactChangePreconditionField {
        record: "precondition.namespace-points-to",
        name: "name",
        required: true,
        form: FieldForm::Name,
    },
    CompactChangePreconditionField {
        record: "precondition.namespace-points-to",
        name: "owner",
        required: true,
        form: FieldForm::ExactOwner,
    },
    CompactChangePreconditionField {
        record: "precondition.dependency-binding",
        name: "package",
        required: true,
        form: FieldForm::ExactPackage,
    },
    CompactChangePreconditionField {
        record: "precondition.dependency-binding",
        name: "semantic-revision",
        required: true,
        form: FieldForm::ExactRevision,
    },
    CompactChangePreconditionField {
        record: "precondition.dependency-binding",
        name: "package-revision",
        required: true,
        form: FieldForm::ExactPackageRevision,
    },
];
pub const COMPACT_TYPE_FORMS: &[&str] = &[
    "unit",
    "bool",
    "i64",
    "bytes",
    "text",
    "static-text",
    "secret",
    "parameter",
    "named",
    "capability-resource",
    "structural-record",
    "list",
    "map",
    "option",
    "result",
    "stream",
    "function",
];
pub const COMPACT_EXPRESSION_FORMS: &[&str] = &[
    "unit",
    "bool",
    "i64",
    "text",
    "static-text",
    "local",
    "constant",
    "if",
    "sequence",
    "call",
    "function-value",
    "invoke",
    "let",
    "record",
    "variant",
    "field",
    "list",
    "map",
    "match",
    "capability-call",
    "transaction",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CompactFormField {
    pub(crate) form: &'static str,
    pub(crate) name: &'static str,
    pub(crate) required: bool,
    pub(crate) syntax: &'static str,
}

pub(crate) const COMPACT_TYPE_FORM_FIELDS: &[CompactFormField] = &[
    CompactFormField {
        form: "unit",
        name: "as",
        required: true,
        syntax: "@NAME",
    },
    CompactFormField {
        form: "bool",
        name: "as",
        required: true,
        syntax: "@NAME",
    },
    CompactFormField {
        form: "i64",
        name: "as",
        required: true,
        syntax: "@NAME",
    },
    CompactFormField {
        form: "bytes",
        name: "as",
        required: true,
        syntax: "@NAME",
    },
    CompactFormField {
        form: "text",
        name: "as",
        required: true,
        syntax: "@NAME",
    },
    CompactFormField {
        form: "static-text",
        name: "as",
        required: true,
        syntax: "@NAME",
    },
    CompactFormField {
        form: "secret",
        name: "as",
        required: true,
        syntax: "@NAME",
    },
    CompactFormField {
        form: "parameter",
        name: "as",
        required: true,
        syntax: "@NAME",
    },
    CompactFormField {
        form: "parameter",
        name: "parameter",
        required: true,
        syntax: "$NAME|tparam_HEX",
    },
    CompactFormField {
        form: "named",
        name: "as",
        required: true,
        syntax: "@NAME",
    },
    CompactFormField {
        form: "named",
        name: "declaration",
        required: true,
        syntax: "$NAME|decl_HEX|MODULE/NAME|pkg_HEX/decl_HEX",
    },
    CompactFormField {
        form: "capability-resource",
        name: "as",
        required: true,
        syntax: "@NAME",
    },
    CompactFormField {
        form: "capability-resource",
        name: "interface",
        required: true,
        syntax: "$NAME|decl_HEX|MODULE/NAME|pkg_HEX/decl_HEX",
    },
    CompactFormField {
        form: "structural-record",
        name: "as",
        required: true,
        syntax: "@NAME",
    },
    CompactFormField {
        form: "list",
        name: "as",
        required: true,
        syntax: "@NAME",
    },
    CompactFormField {
        form: "list",
        name: "item",
        required: true,
        syntax: "type-reference",
    },
    CompactFormField {
        form: "map",
        name: "as",
        required: true,
        syntax: "@NAME",
    },
    CompactFormField {
        form: "map",
        name: "key",
        required: true,
        syntax: "type-reference",
    },
    CompactFormField {
        form: "map",
        name: "value",
        required: true,
        syntax: "type-reference",
    },
    CompactFormField {
        form: "option",
        name: "as",
        required: true,
        syntax: "@NAME",
    },
    CompactFormField {
        form: "option",
        name: "item",
        required: true,
        syntax: "type-reference",
    },
    CompactFormField {
        form: "result",
        name: "as",
        required: true,
        syntax: "@NAME",
    },
    CompactFormField {
        form: "result",
        name: "ok",
        required: true,
        syntax: "type-reference",
    },
    CompactFormField {
        form: "result",
        name: "error",
        required: true,
        syntax: "type-reference",
    },
    CompactFormField {
        form: "stream",
        name: "as",
        required: true,
        syntax: "@NAME",
    },
    CompactFormField {
        form: "stream",
        name: "item",
        required: true,
        syntax: "type-reference",
    },
    CompactFormField {
        form: "function",
        name: "as",
        required: true,
        syntax: "@NAME",
    },
    CompactFormField {
        form: "function",
        name: "result",
        required: true,
        syntax: "type-reference",
    },
];

pub(crate) const COMPACT_EXPRESSION_FORM_FIELDS: &[CompactFormField] = &[
    CompactFormField {
        form: "unit",
        name: "as",
        required: true,
        syntax: "$NAME",
    },
    CompactFormField {
        form: "bool",
        name: "as",
        required: true,
        syntax: "$NAME",
    },
    CompactFormField {
        form: "bool",
        name: "value",
        required: true,
        syntax: "true|false",
    },
    CompactFormField {
        form: "i64",
        name: "as",
        required: true,
        syntax: "$NAME",
    },
    CompactFormField {
        form: "i64",
        name: "value",
        required: true,
        syntax: "signed-i64",
    },
    CompactFormField {
        form: "text",
        name: "as",
        required: true,
        syntax: "$NAME",
    },
    CompactFormField {
        form: "text",
        name: "value",
        required: true,
        syntax: "escaped-utf8",
    },
    CompactFormField {
        form: "static-text",
        name: "as",
        required: true,
        syntax: "$NAME",
    },
    CompactFormField {
        form: "static-text",
        name: "value",
        required: true,
        syntax: "escaped-utf8",
    },
    CompactFormField {
        form: "local",
        name: "as",
        required: true,
        syntax: "$NAME",
    },
    CompactFormField {
        form: "local",
        name: "value",
        required: true,
        syntax: "$NAME|param_HEX|bind_HEX",
    },
    CompactFormField {
        form: "constant",
        name: "as",
        required: true,
        syntax: "$NAME",
    },
    CompactFormField {
        form: "constant",
        name: "declaration",
        required: true,
        syntax: "$NAME|decl_HEX|MODULE/NAME|pkg_HEX/decl_HEX",
    },
    CompactFormField {
        form: "if",
        name: "as",
        required: true,
        syntax: "$NAME",
    },
    CompactFormField {
        form: "if",
        name: "condition",
        required: true,
        syntax: "$NAME",
    },
    CompactFormField {
        form: "if",
        name: "when-true",
        required: true,
        syntax: "$NAME",
    },
    CompactFormField {
        form: "if",
        name: "when-false",
        required: true,
        syntax: "$NAME",
    },
    CompactFormField {
        form: "sequence",
        name: "as",
        required: true,
        syntax: "$NAME",
    },
    CompactFormField {
        form: "call",
        name: "as",
        required: true,
        syntax: "$NAME",
    },
    CompactFormField {
        form: "call",
        name: "function",
        required: true,
        syntax: "$NAME|decl_HEX|MODULE/NAME|pkg_HEX/decl_HEX",
    },
    CompactFormField {
        form: "function-value",
        name: "as",
        required: true,
        syntax: "$NAME",
    },
    CompactFormField {
        form: "function-value",
        name: "function",
        required: true,
        syntax: "$NAME|decl_HEX|MODULE/NAME|pkg_HEX/decl_HEX",
    },
    CompactFormField {
        form: "invoke",
        name: "as",
        required: true,
        syntax: "$NAME",
    },
    CompactFormField {
        form: "invoke",
        name: "function",
        required: true,
        syntax: "$NAME",
    },
    CompactFormField {
        form: "let",
        name: "as",
        required: true,
        syntax: "$NAME",
    },
    CompactFormField {
        form: "let",
        name: "body",
        required: true,
        syntax: "$NAME",
    },
    CompactFormField {
        form: "record",
        name: "as",
        required: true,
        syntax: "$NAME",
    },
    CompactFormField {
        form: "record",
        name: "type",
        required: false,
        syntax: "$NAME|decl_HEX|MODULE/NAME|pkg_HEX/decl_HEX",
    },
    CompactFormField {
        form: "variant",
        name: "as",
        required: true,
        syntax: "$NAME",
    },
    CompactFormField {
        form: "variant",
        name: "case",
        required: true,
        syntax: "$NAME|pkg_HEX/case_HEX",
    },
    CompactFormField {
        form: "variant",
        name: "payload",
        required: false,
        syntax: "$NAME",
    },
    CompactFormField {
        form: "field",
        name: "as",
        required: true,
        syntax: "$NAME",
    },
    CompactFormField {
        form: "field",
        name: "value",
        required: true,
        syntax: "$NAME",
    },
    CompactFormField {
        form: "field",
        name: "name",
        required: false,
        syntax: "structural-field-name",
    },
    CompactFormField {
        form: "field",
        name: "field",
        required: false,
        syntax: "$NAME|pkg_HEX/field_HEX",
    },
    CompactFormField {
        form: "list",
        name: "as",
        required: true,
        syntax: "$NAME",
    },
    CompactFormField {
        form: "list",
        name: "item",
        required: true,
        syntax: "type-reference",
    },
    CompactFormField {
        form: "map",
        name: "as",
        required: true,
        syntax: "$NAME",
    },
    CompactFormField {
        form: "map",
        name: "key",
        required: true,
        syntax: "type-reference",
    },
    CompactFormField {
        form: "map",
        name: "value",
        required: true,
        syntax: "type-reference",
    },
    CompactFormField {
        form: "match",
        name: "as",
        required: true,
        syntax: "$NAME",
    },
    CompactFormField {
        form: "match",
        name: "value",
        required: true,
        syntax: "$NAME",
    },
    CompactFormField {
        form: "capability-call",
        name: "as",
        required: true,
        syntax: "$NAME",
    },
    CompactFormField {
        form: "capability-call",
        name: "requirement",
        required: true,
        syntax: "$NAME|pkg_HEX/req_HEX",
    },
    CompactFormField {
        form: "capability-call",
        name: "operation",
        required: true,
        syntax: "$NAME|pkg_HEX/op_HEX",
    },
    CompactFormField {
        form: "transaction",
        name: "as",
        required: true,
        syntax: "$NAME",
    },
    CompactFormField {
        form: "transaction",
        name: "requirement",
        required: true,
        syntax: "$NAME|pkg_HEX/req_HEX",
    },
    CompactFormField {
        form: "transaction",
        name: "binding",
        required: true,
        syntax: "$NAME",
    },
    CompactFormField {
        form: "transaction",
        name: "name",
        required: true,
        syntax: "name",
    },
    CompactFormField {
        form: "transaction",
        name: "body",
        required: true,
        syntax: "$NAME",
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CompactEdgeDescriptor {
    pub(crate) name: &'static str,
    pub(crate) parent: &'static str,
    pub(crate) child: &'static str,
    pub(crate) fields: &'static [CompactFormField],
}

pub(crate) const COMPACT_CHANGE_EDGE_DESCRIPTORS: &[CompactEdgeDescriptor] = &[
    CompactEdgeDescriptor {
        name: "expression.argument",
        parent: "expression",
        child: "expression",
        fields: &[
            CompactFormField {
                form: "expression.argument",
                name: "parent",
                required: true,
                syntax: "$NAME",
            },
            CompactFormField {
                form: "expression.argument",
                name: "index",
                required: true,
                syntax: "zero-based-index",
            },
            CompactFormField {
                form: "expression.argument",
                name: "expression",
                required: true,
                syntax: "$NAME",
            },
        ],
    },
    CompactEdgeDescriptor {
        name: "type.argument",
        parent: "type-or-expression",
        child: "type",
        fields: &[
            CompactFormField {
                form: "type.argument",
                name: "parent",
                required: true,
                syntax: "@NAME|$NAME",
            },
            CompactFormField {
                form: "type.argument",
                name: "index",
                required: true,
                syntax: "zero-based-index",
            },
            CompactFormField {
                form: "type.argument",
                name: "type",
                required: true,
                syntax: "type-reference",
            },
        ],
    },
    CompactEdgeDescriptor {
        name: "type.field",
        parent: "structural-record-type",
        child: "structural-field",
        fields: &[
            CompactFormField {
                form: "type.field",
                name: "parent",
                required: true,
                syntax: "@NAME",
            },
            CompactFormField {
                form: "type.field",
                name: "index",
                required: true,
                syntax: "zero-based-index",
            },
            CompactFormField {
                form: "type.field",
                name: "name",
                required: true,
                syntax: "name",
            },
            CompactFormField {
                form: "type.field",
                name: "type",
                required: true,
                syntax: "type-reference",
            },
        ],
    },
    CompactEdgeDescriptor {
        name: "effect.requirement",
        parent: "task-function-or-contract-fragment",
        child: "requirement-reference",
        fields: &[
            CompactFormField {
                form: "effect.requirement",
                name: "parent",
                required: true,
                syntax: "$NAME|%NAME",
            },
            CompactFormField {
                form: "effect.requirement",
                name: "index",
                required: true,
                syntax: "zero-based-index",
            },
            CompactFormField {
                form: "effect.requirement",
                name: "requirement",
                required: true,
                syntax: "$NAME|pkg_HEX/req_HEX",
            },
        ],
    },
    CompactEdgeDescriptor {
        name: "requirement.operation",
        parent: "requirement",
        child: "operation-reference",
        fields: &[
            CompactFormField {
                form: "requirement.operation",
                name: "parent",
                required: true,
                syntax: "$NAME",
            },
            CompactFormField {
                form: "requirement.operation",
                name: "index",
                required: true,
                syntax: "zero-based-index",
            },
            CompactFormField {
                form: "requirement.operation",
                name: "operation",
                required: true,
                syntax: "$NAME|pkg_HEX/op_HEX",
            },
        ],
    },
    CompactEdgeDescriptor {
        name: "requirement.limit",
        parent: "requirement",
        child: "resource-limit",
        fields: &[
            CompactFormField {
                form: "requirement.limit",
                name: "parent",
                required: true,
                syntax: "$NAME",
            },
            CompactFormField {
                form: "requirement.limit",
                name: "index",
                required: true,
                syntax: "zero-based-index",
            },
            CompactFormField {
                form: "requirement.limit",
                name: "name",
                required: true,
                syntax: "name",
            },
            CompactFormField {
                form: "requirement.limit",
                name: "maximum",
                required: true,
                syntax: "positive-u64",
            },
            CompactFormField {
                form: "requirement.limit",
                name: "unit",
                required: true,
                syntax: "bytes|items|calls|tasks|milliseconds",
            },
        ],
    },
    CompactEdgeDescriptor {
        name: "expression.binding",
        parent: "let-expression",
        child: "lexical-binding",
        fields: &[
            CompactFormField {
                form: "expression.binding",
                name: "parent",
                required: true,
                syntax: "$NAME",
            },
            CompactFormField {
                form: "expression.binding",
                name: "index",
                required: true,
                syntax: "zero-based-index",
            },
            CompactFormField {
                form: "expression.binding",
                name: "as",
                required: true,
                syntax: "$NAME",
            },
            CompactFormField {
                form: "expression.binding",
                name: "name",
                required: true,
                syntax: "name",
            },
            CompactFormField {
                form: "expression.binding",
                name: "value",
                required: true,
                syntax: "$NAME",
            },
            CompactFormField {
                form: "expression.binding",
                name: "type",
                required: false,
                syntax: "type-reference",
            },
        ],
    },
    CompactEdgeDescriptor {
        name: "expression.record-field",
        parent: "record-expression",
        child: "record-field",
        fields: &[
            CompactFormField {
                form: "expression.record-field",
                name: "parent",
                required: true,
                syntax: "$NAME",
            },
            CompactFormField {
                form: "expression.record-field",
                name: "index",
                required: true,
                syntax: "zero-based-index",
            },
            CompactFormField {
                form: "expression.record-field",
                name: "name",
                required: false,
                syntax: "structural-field-name",
            },
            CompactFormField {
                form: "expression.record-field",
                name: "field",
                required: false,
                syntax: "$NAME|pkg_HEX/field_HEX",
            },
            CompactFormField {
                form: "expression.record-field",
                name: "value",
                required: true,
                syntax: "$NAME",
            },
        ],
    },
    CompactEdgeDescriptor {
        name: "expression.map-entry",
        parent: "map-expression",
        child: "map-entry",
        fields: &[
            CompactFormField {
                form: "expression.map-entry",
                name: "parent",
                required: true,
                syntax: "$NAME",
            },
            CompactFormField {
                form: "expression.map-entry",
                name: "index",
                required: true,
                syntax: "zero-based-index",
            },
            CompactFormField {
                form: "expression.map-entry",
                name: "key",
                required: true,
                syntax: "$NAME",
            },
            CompactFormField {
                form: "expression.map-entry",
                name: "value",
                required: true,
                syntax: "$NAME",
            },
        ],
    },
    CompactEdgeDescriptor {
        name: "expression.match-arm",
        parent: "match-expression",
        child: "match-arm",
        fields: &[
            CompactFormField {
                form: "expression.match-arm",
                name: "parent",
                required: true,
                syntax: "$NAME",
            },
            CompactFormField {
                form: "expression.match-arm",
                name: "index",
                required: true,
                syntax: "zero-based-index",
            },
            CompactFormField {
                form: "expression.match-arm",
                name: "case",
                required: true,
                syntax: "$NAME|pkg_HEX/case_HEX",
            },
            CompactFormField {
                form: "expression.match-arm",
                name: "as",
                required: false,
                syntax: "$NAME",
            },
            CompactFormField {
                form: "expression.match-arm",
                name: "name",
                required: false,
                syntax: "name",
            },
            CompactFormField {
                form: "expression.match-arm",
                name: "type",
                required: false,
                syntax: "type-reference",
            },
            CompactFormField {
                form: "expression.match-arm",
                name: "body",
                required: true,
                syntax: "$NAME",
            },
        ],
    },
];

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct ChangeRequestCommitment([u8; 32]);

impl ChangeRequestCommitment {
    pub(crate) const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub(crate) const fn bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Display for ChangeRequestCommitment {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("request_")?;
        formatter.write_str(&crate::platform::semantic_id::encode_hex(&self.0))
    }
}

/// One transport-neutral public request. Semantic operations remain owned by the change engine;
/// publication options and the request commitment remain operational control data.
#[derive(Clone, Debug)]
pub(crate) struct NormalizedChangeRequest {
    pub semantic: AuthoredChangeSet,
    pub options: PublicationOptions,
    pub request_commitment: ChangeRequestCommitment,
}

pub(crate) fn decode_compact_change(
    path: &str,
    input: &[u8],
) -> Result<NormalizedChangeRequest, Vec<Diagnostic>> {
    let records = parse_records(path, input)?;
    Decoder::new(records)
        .decode()
        .map_err(|diagnostic| vec![diagnostic])
}

#[derive(Clone, Debug)]
struct IndexedValue {
    index: usize,
    value: String,
    location: SourceLocation,
}

#[derive(Clone, Debug)]
struct IndexedRecord {
    index: usize,
    record: CompactRecord,
}

struct Decoder {
    records: Vec<CompactRecord>,
    types: BTreeMap<String, CompactRecord>,
    expressions: BTreeMap<String, CompactRecord>,
    arguments: BTreeMap<String, Vec<IndexedValue>>,
    type_parameters: BTreeMap<String, Vec<IndexedValue>>,
    record_edges: BTreeMap<String, BTreeMap<String, Vec<IndexedRecord>>>,
    consumed_record_edges: BTreeSet<(String, String)>,
    consumed_value_edges: BTreeSet<(bool, String)>,
    fragments: BTreeMap<String, SourceLocation>,
    preconditions: Vec<CompactRecord>,
    changes: Vec<(&'static CompactChangeOperationDescriptor, CompactRecord)>,
    type_cache: BTreeMap<String, AuthoredType>,
    type_stack: BTreeSet<String>,
    expression_stack: BTreeSet<String>,
    expression_uses: BTreeMap<String, usize>,
}

impl Decoder {
    fn new(records: Vec<CompactRecord>) -> Self {
        Self {
            records,
            types: BTreeMap::new(),
            expressions: BTreeMap::new(),
            arguments: BTreeMap::new(),
            type_parameters: BTreeMap::new(),
            record_edges: BTreeMap::new(),
            consumed_record_edges: BTreeSet::new(),
            consumed_value_edges: BTreeSet::new(),
            fragments: BTreeMap::new(),
            preconditions: Vec::new(),
            changes: Vec::new(),
            type_cache: BTreeMap::new(),
            type_stack: BTreeSet::new(),
            expression_stack: BTreeSet::new(),
            expression_uses: BTreeMap::new(),
        }
    }

    fn decode(mut self) -> Result<NormalizedChangeRequest, Diagnostic> {
        let mut request = None;
        for record in std::mem::take(&mut self.records) {
            match record.operation.as_str() {
                "request" => {
                    if request.is_some() {
                        return Err(record_error(
                            &record,
                            "change_request_duplicate",
                            "compact change contains more than one request record",
                        ));
                    }
                    request = Some(record);
                }
                "expression.argument" => self.insert_indexed_edge(&record, "expression", false)?,
                "type.argument" => self.insert_indexed_edge(&record, "type", true)?,
                "type.field" => {
                    self.insert_indexed_record_edge(record, &["parent", "index", "name", "type"])?
                }
                "effect.requirement" => {
                    self.insert_indexed_record_edge(record, &["parent", "index", "requirement"])?
                }
                "requirement.operation" => {
                    self.insert_indexed_record_edge(record, &["parent", "index", "operation"])?
                }
                "requirement.limit" => self.insert_indexed_record_edge(
                    record,
                    &["parent", "index", "name", "maximum", "unit"],
                )?,
                "expression.binding" => self.insert_indexed_record_edge(
                    record,
                    &["parent", "index", "as", "name", "value", "type"],
                )?,
                "expression.record-field" => self.insert_indexed_record_edge(
                    record,
                    &["parent", "index", "name", "field", "value"],
                )?,
                "expression.map-entry" => {
                    self.insert_indexed_record_edge(record, &["parent", "index", "key", "value"])?
                }
                "expression.match-arm" => self.insert_indexed_record_edge(
                    record,
                    &["parent", "index", "case", "as", "name", "type", "body"],
                )?,
                operation if operation.starts_with("type.") => {
                    let label = required(&record, "as")?.to_owned();
                    validate_local_label(&record, "as", &label, '@')?;
                    if self.types.insert(label.clone(), record.clone()).is_some() {
                        return Err(field_error(
                            &record,
                            "as",
                            "change_type_duplicate",
                            format!("type label '{label}' is defined more than once"),
                        ));
                    }
                }
                operation if operation.starts_with("expression.") => {
                    let symbol = required(&record, "as")?.to_owned();
                    validate_local_label(&record, "as", &symbol, '$')?;
                    if self
                        .expressions
                        .insert(symbol.clone(), record.clone())
                        .is_some()
                    {
                        return Err(field_error(
                            &record,
                            "as",
                            "change_expression_duplicate",
                            format!("expression symbol '{symbol}' is defined more than once"),
                        ));
                    }
                    self.expression_uses.insert(symbol, 0);
                }
                operation if is_change_precondition(operation) => self.preconditions.push(record),
                operation if operation.starts_with("precondition.") => {
                    return Err(record_error(
                        &record,
                        "change_precondition_unknown",
                        format!(
                            "unknown compact semantic precondition '{}'; use focused change discovery",
                            record.operation
                        ),
                    ));
                }
                operation => {
                    if let Some(descriptor) = compact_change_operation_descriptor(operation) {
                        if matches!(
                            descriptor.operation,
                            CompactChangeOperation::SetFunctionContract
                                | CompactChangeOperation::SetRequirementContract
                        ) {
                            let label = fragment(&record, "as")?;
                            if self
                                .fragments
                                .insert(label.clone(), record.location.clone())
                                .is_some()
                            {
                                return Err(field_error(
                                    &record,
                                    "as",
                                    "change_fragment_duplicate",
                                    format!("fragment label '{label}' is defined more than once"),
                                ));
                            }
                        }
                        self.changes.push((descriptor, record));
                    } else {
                        return Err(record_error(
                            &record,
                            "change_operation_unknown",
                            format!(
                                "unknown compact change record '{}'; use 'capabilities change'",
                                record.operation
                            ),
                        ));
                    }
                }
            }
        }
        let request = request.ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Source,
                "change_request_missing",
                "compact change requires one request record with an exact base revision",
            )
        })?;
        check_fields(&request, &["base", "idempotency", "intent"])?;
        let base = parse_field::<RevisionId>(&request, "base")?;
        let idempotency_key = optional(&request, "idempotency").map(str::to_owned);
        let intent = optional(&request, "intent").map(str::to_owned);
        let type_labels = self.types.keys().cloned().collect::<Vec<_>>();
        for label in type_labels {
            let _ = self.decode_type(&label)?;
        }
        if self.changes.is_empty() {
            return Err(record_error(
                &request,
                "change_operations_missing",
                "compact change requires at least one semantic operation",
            ));
        }

        let preconditions = self
            .preconditions
            .iter()
            .map(decode_precondition)
            .collect::<Result<Vec<_>, _>>()?;
        let mut changes = Vec::with_capacity(self.changes.len());
        for (descriptor, record) in std::mem::take(&mut self.changes) {
            changes.push(self.decode_change(descriptor, &record)?);
        }
        for (symbol, uses) in &self.expression_uses {
            if *uses == 0 {
                let record = self.expressions.get(symbol).ok_or_else(|| {
                    Diagnostic::new(
                        DiagnosticClass::Infrastructure,
                        "change_expression_inventory",
                        "expression use inventory lost a definition",
                    )
                })?;
                return Err(field_error(
                    record,
                    "as",
                    "change_expression_unused",
                    format!("expression '{symbol}' is not reachable from a semantic operation"),
                ));
            }
            if *uses > 1 {
                let record = self.expressions.get(symbol).ok_or_else(|| {
                    Diagnostic::new(
                        DiagnosticClass::Infrastructure,
                        "change_expression_inventory",
                        "expression use inventory lost a definition",
                    )
                })?;
                return Err(field_error(
                    record,
                    "as",
                    "change_expression_shared",
                    format!(
                        "expression '{symbol}' is referenced {uses} times; expression definitions form one owned tree"
                    ),
                ));
            }
        }
        for (parent, edges) in self.arguments.iter().chain(self.type_parameters.iter()) {
            if !self.expressions.contains_key(parent) && !self.types.contains_key(parent) {
                return Err(Diagnostic::source(
                    "change_edge_parent",
                    format!("edge parent '{parent}' has no matching compact definition"),
                    edges
                        .first()
                        .map(|edge| edge.location.clone())
                        .unwrap_or_else(|| request.location.clone()),
                ));
            }
        }
        for (type_edge, parents) in [(false, &self.arguments), (true, &self.type_parameters)] {
            for (parent, edges) in parents {
                if !self
                    .consumed_value_edges
                    .contains(&(type_edge, parent.clone()))
                {
                    return Err(Diagnostic::source(
                        "change_edge_unconsumed",
                        format!("child edges for '{parent}' are not accepted by its compact form"),
                        edges
                            .first()
                            .map(|edge| edge.location.clone())
                            .unwrap_or_else(|| request.location.clone()),
                    ));
                }
            }
        }
        for (operation, parents) in &self.record_edges {
            for (parent, edges) in parents {
                if !self
                    .consumed_record_edges
                    .contains(&(operation.clone(), parent.clone()))
                {
                    return Err(Diagnostic::source(
                        "change_edge_unconsumed",
                        format!(
                            "record edges '{operation}' for '{parent}' are not accepted by its compact form"
                        ),
                        edges
                            .first()
                            .map(|edge| edge.record.location.clone())
                            .unwrap_or_else(|| request.location.clone()),
                    ));
                }
            }
        }

        let semantic = AuthoredChangeSet {
            base,
            preconditions,
            changes,
            budget: Default::default(),
        };
        let options = PublicationOptions {
            idempotency_key,
            intent,
        };
        normalize_change_request(semantic, options).map_err(|mut diagnostic| {
            let option = match diagnostic.code.as_str() {
                "change_idempotency" => "idempotency",
                "change_intent_bytes" => "intent",
                _ => return diagnostic,
            };
            diagnostic.location = field(&request, option).map(|field| field.location.clone());
            diagnostic
        })
    }

    fn insert_indexed_edge(
        &mut self,
        record: &CompactRecord,
        value_field: &str,
        type_edge: bool,
    ) -> Result<(), Diagnostic> {
        check_fields(record, &["parent", "index", value_field])?;
        let parent = required(record, "parent")?.to_owned();
        validate_fragment_parent(record, "parent", &parent)?;
        let index = parse_field::<usize>(record, "index")?;
        let value = required(record, value_field)?.to_owned();
        let edge = IndexedValue {
            index,
            value,
            location: record.location.clone(),
        };
        let edges = if type_edge {
            self.type_parameters.entry(parent).or_default()
        } else {
            self.arguments.entry(parent).or_default()
        };
        if edges.iter().any(|candidate| candidate.index == index) {
            return Err(field_error(
                record,
                "index",
                "change_edge_index_duplicate",
                format!("parent repeats child index {index}"),
            ));
        }
        edges.push(edge);
        Ok(())
    }

    fn insert_indexed_record_edge(
        &mut self,
        record: CompactRecord,
        allowed: &[&str],
    ) -> Result<(), Diagnostic> {
        check_fields(&record, allowed)?;
        let parent = required(&record, "parent")?.to_owned();
        validate_fragment_parent(&record, "parent", &parent)?;
        let index = parse_field::<usize>(&record, "index")?;
        let operation = record.operation.clone();
        let edges = self
            .record_edges
            .entry(operation)
            .or_default()
            .entry(parent)
            .or_default();
        if edges.iter().any(|candidate| candidate.index == index) {
            return Err(field_error(
                &record,
                "index",
                "change_edge_index_duplicate",
                format!("parent repeats child index {index}"),
            ));
        }
        edges.push(IndexedRecord { index, record });
        Ok(())
    }

    fn decode_change(
        &mut self,
        descriptor: &CompactChangeOperationDescriptor,
        record: &CompactRecord,
    ) -> Result<AuthoredChange, Diagnostic> {
        check_operation_fields(record, descriptor.fields)?;
        match descriptor.operation {
            CompactChangeOperation::CreateModule => Ok(AuthoredChange::CreateModule {
                symbol: symbol(record, "as")?,
                name: parse_name(record, "name")?,
            }),
            CompactChangeOperation::CreateRecord => Ok(AuthoredChange::CreateRecord {
                symbol: symbol(record, "as")?,
                module: parse_module_selector(record, "module")?,
                name: parse_name(record, "name")?,
                visibility: parse_visibility(record, "visibility")?,
                fields: Vec::new(),
            }),
            CompactChangeOperation::CreateVariant => Ok(AuthoredChange::CreateVariant {
                symbol: symbol(record, "as")?,
                module: parse_module_selector(record, "module")?,
                name: parse_name(record, "name")?,
                visibility: parse_visibility(record, "visibility")?,
                cases: Vec::new(),
            }),
            CompactChangeOperation::CreateInterface => Ok(AuthoredChange::CreateInterface {
                symbol: symbol(record, "as")?,
                module: parse_module_selector(record, "module")?,
                name: parse_name(record, "name")?,
                visibility: parse_visibility(record, "visibility")?,
                operations: Vec::new(),
            }),
            CompactChangeOperation::CreateExternal => Ok(AuthoredChange::CreateExternal {
                symbol: symbol(record, "as")?,
                module: parse_module_selector(record, "module")?,
                name: parse_name(record, "name")?,
                visibility: parse_visibility(record, "visibility")?,
                type_parameters: Vec::new(),
                parameters: Vec::new(),
                result: self.decode_type(required(record, "result")?)?,
                implementation: parse_implementation_name(record, "implementation")?,
            }),
            CompactChangeOperation::CreateFunction => {
                let function_symbol = symbol(record, "as")?;
                let body = required(record, "body")?.to_owned();
                Ok(AuthoredChange::CreateFunction {
                    symbol: function_symbol.clone(),
                    module: parse_module_selector(record, "module")?,
                    name: parse_name(record, "name")?,
                    visibility: parse_visibility(record, "visibility")?,
                    type_parameters: Vec::new(),
                    parameters: Vec::new(),
                    result: self.decode_type(required(record, "result")?)?,
                    effect: self.decode_function_effect(
                        record,
                        &function_symbol,
                        required(record, "effect")?,
                    )?,
                    body: self.decode_expression(&body)?,
                })
            }
            CompactChangeOperation::CreateConstant => {
                let value = required(record, "value")?.to_owned();
                Ok(AuthoredChange::CreateConstant {
                    symbol: symbol(record, "as")?,
                    module: parse_module_selector(record, "module")?,
                    name: parse_name(record, "name")?,
                    visibility: parse_visibility(record, "visibility")?,
                    ty: self.decode_type(required(record, "type")?)?,
                    value: self.decode_expression(&value)?,
                })
            }
            CompactChangeOperation::CreateComponent => Ok(AuthoredChange::CreateComponent {
                symbol: symbol(record, "as")?,
                module: parse_module_selector(record, "module")?,
                name: parse_name(record, "name")?,
                visibility: parse_visibility(record, "visibility")?,
                requirements: Vec::new(),
                ports: Vec::new(),
            }),
            CompactChangeOperation::CreateTest => {
                let actual = required(record, "actual")?.to_owned();
                let expected = required(record, "expected")?.to_owned();
                Ok(AuthoredChange::CreateTest {
                    symbol: symbol(record, "as")?,
                    module: parse_module_selector(record, "module")?,
                    name: parse_name(record, "name")?,
                    visibility: parse_visibility(record, "visibility")?,
                    actual: self.decode_expression(&actual)?,
                    expected: self.decode_expression(&expected)?,
                })
            }
            CompactChangeOperation::CreateTarget => {
                let runner = parse_runner_kind(record, "runner")?;
                let port = optional(record, "port")
                    .map(|_| parse_port_reference(record, "port"))
                    .transpose()?;
                if (runner == RunnerKind::Http) == port.is_some() {
                    return Err(record_error(
                        record,
                        "change_target_port_condition",
                        "create.target forbids port for http and requires port for every other runner",
                    ));
                }
                Ok(AuthoredChange::CreateTarget {
                    symbol: symbol(record, "as")?,
                    name: parse_name(record, "name")?,
                    component: parse_declaration_reference(record, "component")?,
                    port,
                    runner,
                })
            }
            CompactChangeOperation::AddField => Ok(AuthoredChange::AddField {
                record: parse_declaration_selector(record, "record")?,
                field: AuthoredField {
                    symbol: symbol(record, "as")?,
                    name: parse_name(record, "name")?,
                    ty: self.decode_type(required(record, "type")?)?,
                },
            }),
            CompactChangeOperation::AddCase => Ok(AuthoredChange::AddCase {
                variant: parse_declaration_selector(record, "variant")?,
                case: AuthoredCase {
                    symbol: symbol(record, "as")?,
                    name: parse_name(record, "name")?,
                    payload: optional(record, "payload")
                        .map(|value| self.decode_type(value))
                        .transpose()?,
                },
            }),
            CompactChangeOperation::AddOperation => Ok(AuthoredChange::AddOperation {
                interface: parse_declaration_selector(record, "interface")?,
                operation: crate::platform::change::AuthoredOperation {
                    symbol: symbol(record, "as")?,
                    name: parse_name(record, "name")?,
                    parameters: Vec::new(),
                    result: self.decode_type(required(record, "result")?)?,
                    idempotency: parse_idempotency(record, "idempotency")?,
                    external_visibility: parse_external_visibility(record, "external-visibility")?,
                },
            }),
            CompactChangeOperation::AddTypeParameter => Ok(AuthoredChange::AddTypeParameter {
                declaration: parse_declaration_selector(record, "function")?,
                parameter: AuthoredTypeParameter {
                    symbol: symbol(record, "as")?,
                    name: parse_name(record, "name")?,
                },
            }),
            CompactChangeOperation::AddParameter => {
                let parent = match (optional(record, "function"), optional(record, "operation")) {
                    (Some(_), None) => ParameterParentSelector::Declaration {
                        declaration: parse_declaration_selector(record, "function")?,
                    },
                    (None, Some(_)) => ParameterParentSelector::Operation {
                        operation: parse_owner_selector(record, "operation")?,
                    },
                    _ => {
                        return Err(record_error(
                            record,
                            "change_parameter_parent",
                            "add.parameter requires exactly one of function or operation",
                        ));
                    }
                };
                Ok(AuthoredChange::AddParameter {
                    parent,
                    parameter: AuthoredParameter {
                        symbol: symbol(record, "as")?,
                        name: parse_name(record, "name")?,
                        ty: self.decode_type(required(record, "type")?)?,
                        use_mode: optional(record, "use")
                            .map(|_| parse_parameter_use(record, "use"))
                            .transpose()?
                            .unwrap_or_default(),
                        resource_requirement: optional(record, "requirement")
                            .map(|_| parse_requirement_reference(record, "requirement"))
                            .transpose()?,
                    },
                })
            }
            CompactChangeOperation::AddRequirement => {
                let requirement_symbol = symbol(record, "as")?;
                let operations = self
                    .ordered_record_edges("requirement.operation", &requirement_symbol)?
                    .iter()
                    .map(|edge| {
                        required(&edge.record, "operation")?;
                        parse_operation_reference(&edge.record, "operation")
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let limits = self
                    .ordered_record_edges("requirement.limit", &requirement_symbol)?
                    .iter()
                    .map(|edge| {
                        Ok(AuthoredResourceLimit {
                            name: parse_name(&edge.record, "name")?,
                            maximum: parse_field(&edge.record, "maximum")?,
                            unit: parse_resource_unit(&edge.record, "unit")?,
                        })
                    })
                    .collect::<Result<Vec<_>, Diagnostic>>()?;
                Ok(AuthoredChange::AddRequirement {
                    component: parse_declaration_selector(record, "component")?,
                    requirement: AuthoredRequirement {
                        symbol: requirement_symbol,
                        name: parse_name(record, "name")?,
                        interface: parse_declaration_reference(record, "interface")?,
                        operations,
                        limits,
                    },
                })
            }
            CompactChangeOperation::AddPort => Ok(AuthoredChange::AddPort {
                component: parse_declaration_selector(record, "component")?,
                port: AuthoredPort {
                    symbol: symbol(record, "as")?,
                    name: parse_name(record, "name")?,
                    function_type: self.decode_type(required(record, "type")?)?,
                    implementation: AuthoredPortImplementation::Function {
                        function: parse_declaration_reference(record, "function")?,
                    },
                },
            }),
            CompactChangeOperation::AddHttpRoute => Ok(AuthoredChange::AddHttpRoute {
                symbol: symbol(record, "as")?,
                target: parse_owner_selector(record, "target")?,
                method: required(record, "method")?.to_owned(),
                selector: parse_http_route_selector(record)?,
                port: parse_port_reference(record, "port")?,
            }),
            CompactChangeOperation::AddDependency => {
                let package = parse_field(record, "package")?;
                let semantic_revision = parse_field(record, "semantic-revision")?;
                let package_revision = parse_field(record, "package-revision")?;
                Ok(AuthoredChange::AddDependency {
                    package,
                    semantic_revision,
                    package_revision,
                })
            }
            CompactChangeOperation::ReplaceDependency => Ok(AuthoredChange::ReplaceDependency {
                package: parse_field(record, "package")?,
                semantic_revision: parse_field(record, "semantic-revision")?,
                package_revision: parse_field(record, "package-revision")?,
            }),
            CompactChangeOperation::SetFunctionContract => {
                let fragment = fragment(record, "as")?;
                Ok(AuthoredChange::SetFunctionContract {
                    function: parse_declaration_selector(record, "function")?,
                    result: self.decode_type(required(record, "result")?)?,
                    effect: self.decode_function_effect(
                        record,
                        &fragment,
                        required(record, "effect")?,
                    )?,
                })
            }
            CompactChangeOperation::SetRequirementContract => {
                let fragment = fragment(record, "as")?;
                let operations = self
                    .ordered_record_edges("requirement.operation", &fragment)?
                    .iter()
                    .map(|edge| parse_operation_reference(&edge.record, "operation"))
                    .collect::<Result<Vec<_>, _>>()?;
                let limits = self
                    .ordered_record_edges("requirement.limit", &fragment)?
                    .iter()
                    .map(|edge| {
                        Ok(AuthoredResourceLimit {
                            name: parse_name(&edge.record, "name")?,
                            maximum: parse_field(&edge.record, "maximum")?,
                            unit: parse_resource_unit(&edge.record, "unit")?,
                        })
                    })
                    .collect::<Result<Vec<_>, Diagnostic>>()?;
                Ok(AuthoredChange::SetRequirementContract {
                    requirement: parse_owner_selector(record, "requirement")?,
                    interface: parse_declaration_reference(record, "interface")?,
                    operations,
                    limits,
                })
            }
            CompactChangeOperation::SetHttpRoute => Ok(AuthoredChange::SetHttpRoute {
                route: parse_owner_selector(record, "route")?,
                method: required(record, "method")?.to_owned(),
                selector: parse_http_route_selector(record)?,
                port: parse_port_reference(record, "port")?,
            }),
            CompactChangeOperation::DeleteOwner => {
                let policy = required(record, "policy")?;
                let policy = match policy {
                    "reject" => AuthoredDeletePolicy::Reject,
                    "owned-closure" => AuthoredDeletePolicy::OwnedClosure,
                    _ => {
                        return Err(field_error(
                            record,
                            "policy",
                            "change_delete_policy",
                            format!(
                                "deletion policy must be reject or owned-closure; observed '{policy}'"
                            ),
                        ));
                    }
                };
                Ok(AuthoredChange::DeleteOwner {
                    owner: OwnerSelector::Exact {
                        owner: parse_field::<OwnerKey>(record, "owner")?,
                    },
                    policy,
                })
            }
            CompactChangeOperation::RenameOwner => Ok(AuthoredChange::RenameOwner {
                owner: parse_owner_selector(record, "owner")?,
                name: parse_name(record, "name")?,
            }),
            CompactChangeOperation::MoveDeclaration => Ok(AuthoredChange::MoveDeclaration {
                declaration: parse_declaration_selector(record, "declaration")?,
                module: parse_module_selector(record, "module")?,
            }),
            CompactChangeOperation::ReplaceBody => {
                let body = required(record, "body")?.to_owned();
                Ok(AuthoredChange::ReplaceFunctionBody {
                    function: parse_declaration_selector(record, "function")?,
                    body: self.decode_expression(&body)?,
                })
            }
            CompactChangeOperation::ExtractFunction => Ok(AuthoredChange::ExtractFunction {
                symbol: symbol(record, "as")?,
                function: parse_declaration_selector(record, "function")?,
                expression: parse_field::<ExpressionId>(record, "expression")?,
                name: parse_name(record, "name")?,
            }),
        }
    }

    fn decode_function_effect(
        &mut self,
        record: &CompactRecord,
        parent: &str,
        effect: &str,
    ) -> Result<AuthoredFunctionEffect, Diagnostic> {
        match effect {
            "pure" => Ok(AuthoredFunctionEffect::Pure {}),
            "task" => {
                let requirements = self
                    .ordered_record_edges("effect.requirement", parent)?
                    .iter()
                    .map(|edge| parse_requirement_reference(&edge.record, "requirement"))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(AuthoredFunctionEffect::Task { requirements })
            }
            _ => Err(field_error(
                record,
                "effect",
                "change_effect_unsupported",
                format!("function effect must be pure or task; observed '{effect}'"),
            )),
        }
    }

    fn decode_type(&mut self, reference: &str) -> Result<AuthoredType, Diagnostic> {
        let primitive = match reference {
            "unit" => Some(AuthoredType::Unit {}),
            "bool" => Some(AuthoredType::Bool {}),
            "i64" => Some(AuthoredType::I64 {}),
            "bytes" => Some(AuthoredType::Bytes {}),
            "text" => Some(AuthoredType::Text {}),
            "static-text" => Some(AuthoredType::StaticText {}),
            "secret" => Some(AuthoredType::Secret {}),
            _ => None,
        };
        if let Some(primitive) = primitive {
            return Ok(primitive);
        }
        if !reference.starts_with('@') {
            return Err(Diagnostic::new(
                DiagnosticClass::Source,
                "change_type_reference",
                format!("unknown type reference '{reference}'"),
            ));
        }
        if let Some(cached) = self.type_cache.get(reference) {
            return Ok(cached.clone());
        }
        if !self.type_stack.insert(reference.to_owned()) {
            return Err(Diagnostic::new(
                DiagnosticClass::Semantic,
                "change_type_cycle",
                format!("type definition cycle reaches '{reference}'"),
            ));
        }
        let record = self.types.get(reference).cloned().ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Source,
                "change_type_undefined",
                format!("type label '{reference}' is not defined"),
            )
        })?;
        let ty = match record.operation.as_str() {
            "type.unit" | "type.bool" | "type.i64" | "type.bytes" | "type.text"
            | "type.static-text" | "type.secret" => {
                check_fields(&record, &["as"])?;
                match record.operation.as_str() {
                    "type.unit" => AuthoredType::Unit {},
                    "type.bool" => AuthoredType::Bool {},
                    "type.i64" => AuthoredType::I64 {},
                    "type.bytes" => AuthoredType::Bytes {},
                    "type.text" => AuthoredType::Text {},
                    "type.static-text" => AuthoredType::StaticText {},
                    _ => AuthoredType::Secret {},
                }
            }
            "type.list" | "type.option" | "type.stream" => {
                check_fields(&record, &["as", "item"])?;
                let item = Box::new(self.decode_type(required(&record, "item")?)?);
                match record.operation.as_str() {
                    "type.list" => AuthoredType::List { item },
                    "type.option" => AuthoredType::Option { item },
                    _ => AuthoredType::Stream { item },
                }
            }
            "type.map" => {
                check_fields(&record, &["as", "key", "value"])?;
                AuthoredType::Map {
                    key: Box::new(self.decode_type(required(&record, "key")?)?),
                    value: Box::new(self.decode_type(required(&record, "value")?)?),
                }
            }
            "type.result" => {
                check_fields(&record, &["as", "ok", "error"])?;
                AuthoredType::Result {
                    ok: Box::new(self.decode_type(required(&record, "ok")?)?),
                    error: Box::new(self.decode_type(required(&record, "error")?)?),
                }
            }
            "type.named" => {
                check_fields(&record, &["as", "declaration"])?;
                AuthoredType::Named {
                    declaration: parse_declaration_reference(&record, "declaration")?,
                }
            }
            "type.capability-resource" => {
                check_fields(&record, &["as", "interface"])?;
                AuthoredType::CapabilityResource {
                    interface: parse_declaration_reference(&record, "interface")?,
                }
            }
            "type.structural-record" => {
                check_fields(&record, &["as"])?;
                let fields = self
                    .ordered_record_edges("type.field", reference)?
                    .iter()
                    .map(|edge| {
                        Ok(AuthoredStructuralTypeField {
                            name: parse_name(&edge.record, "name")?,
                            ty: self.decode_type(required(&edge.record, "type")?)?,
                        })
                    })
                    .collect::<Result<Vec<_>, Diagnostic>>()?;
                AuthoredType::StructuralRecord { fields }
            }
            "type.parameter" => {
                check_fields(&record, &["as", "parameter"])?;
                AuthoredType::TypeParameter {
                    parameter: parse_type_parameter_reference(&record, "parameter")?,
                }
            }
            "type.function" => {
                check_fields(&record, &["as", "result"])?;
                let parameters = self
                    .ordered_edges(reference, true)?
                    .into_iter()
                    .map(|edge| self.decode_type(&edge.value))
                    .collect::<Result<Vec<_>, _>>()?;
                AuthoredType::Function {
                    parameters,
                    result: Box::new(self.decode_type(required(&record, "result")?)?),
                }
            }
            operation => {
                return Err(record_error(
                    &record,
                    "change_type_form_unknown",
                    format!("unknown compact type form '{operation}'"),
                ));
            }
        };
        self.type_stack.remove(reference);
        self.type_cache.insert(reference.to_owned(), ty.clone());
        Ok(ty)
    }

    fn decode_expression(&mut self, symbol: &str) -> Result<AuthoredExpression, Diagnostic> {
        if !symbol.starts_with('$') {
            return Err(Diagnostic::new(
                DiagnosticClass::Source,
                "change_expression_reference",
                format!("expression reference '{symbol}' must be a $ symbol"),
            ));
        }
        if !self.expression_stack.insert(symbol.to_owned()) {
            return Err(Diagnostic::new(
                DiagnosticClass::Semantic,
                "change_expression_cycle",
                format!("expression definition cycle reaches '{symbol}'"),
            ));
        }
        let record = self.expressions.get(symbol).cloned().ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Source,
                "change_expression_undefined",
                format!("expression symbol '{symbol}' is not defined"),
            )
        })?;
        let uses = self.expression_uses.get_mut(symbol).ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Infrastructure,
                "change_expression_inventory",
                "expression use inventory lost a definition",
            )
        })?;
        *uses = uses.saturating_add(1);
        let operation = match record.operation.as_str() {
            "expression.unit" => {
                check_fields(&record, &["as"])?;
                AuthoredExpressionOperation::Unit {}
            }
            "expression.bool" => {
                check_fields(&record, &["as", "value"])?;
                AuthoredExpressionOperation::Bool {
                    value: parse_bool(&record, "value")?,
                }
            }
            "expression.i64" => {
                check_fields(&record, &["as", "value"])?;
                AuthoredExpressionOperation::I64 {
                    value: parse_field(&record, "value")?,
                }
            }
            "expression.text" | "expression.static-text" => {
                check_fields(&record, &["as", "value"])?;
                let value = required(&record, "value")?.to_owned();
                if record.operation == "expression.text" {
                    AuthoredExpressionOperation::Text { value }
                } else {
                    AuthoredExpressionOperation::StaticText { value }
                }
            }
            "expression.local" => {
                check_fields(&record, &["as", "value"])?;
                AuthoredExpressionOperation::Local {
                    value: parse_local_reference(&record, "value")?,
                }
            }
            "expression.constant" => {
                check_fields(&record, &["as", "declaration"])?;
                AuthoredExpressionOperation::Constant {
                    declaration: parse_declaration_reference(&record, "declaration")?,
                }
            }
            "expression.if" => {
                check_fields(&record, &["as", "condition", "when-true", "when-false"])?;
                let condition = required(&record, "condition")?.to_owned();
                let when_true = required(&record, "when-true")?.to_owned();
                let when_false = required(&record, "when-false")?.to_owned();
                AuthoredExpressionOperation::If {
                    condition: Box::new(self.decode_expression(&condition)?),
                    when_true: Box::new(self.decode_expression(&when_true)?),
                    when_false: Box::new(self.decode_expression(&when_false)?),
                }
            }
            "expression.sequence" => {
                check_fields(&record, &["as"])?;
                AuthoredExpressionOperation::Sequence {
                    items: self.decode_expression_edges(symbol)?,
                }
            }
            "expression.call" => {
                check_fields(&record, &["as", "function"])?;
                AuthoredExpressionOperation::Call {
                    function: parse_declaration_reference(&record, "function")?,
                    type_arguments: self
                        .ordered_edges(symbol, true)?
                        .into_iter()
                        .map(|edge| self.decode_type(&edge.value))
                        .collect::<Result<Vec<_>, _>>()?,
                    arguments: self.decode_expression_edges(symbol)?,
                }
            }
            "expression.function-value" => {
                check_fields(&record, &["as", "function"])?;
                AuthoredExpressionOperation::FunctionValue {
                    function: parse_declaration_reference(&record, "function")?,
                    type_arguments: self
                        .ordered_edges(symbol, true)?
                        .into_iter()
                        .map(|edge| self.decode_type(&edge.value))
                        .collect::<Result<Vec<_>, _>>()?,
                }
            }
            "expression.invoke" => {
                check_fields(&record, &["as", "function"])?;
                let function = required(&record, "function")?.to_owned();
                AuthoredExpressionOperation::Invoke {
                    callee: Box::new(self.decode_expression(&function)?),
                    arguments: self.decode_expression_edges(symbol)?,
                }
            }
            "expression.let" => {
                check_fields(&record, &["as", "body"])?;
                let mut bindings = Vec::new();
                for edge in self.ordered_record_edges("expression.binding", symbol)? {
                    let value = required(&edge.record, "value")?.to_owned();
                    bindings.push(AuthoredLetBinding {
                        symbol: symbol_field(&edge.record, "as")?,
                        name: parse_name(&edge.record, "name")?,
                        value: self.decode_expression(&value)?,
                        declared_type: optional(&edge.record, "type")
                            .map(|value| self.decode_type(value))
                            .transpose()?,
                    });
                }
                let body = required(&record, "body")?.to_owned();
                AuthoredExpressionOperation::Let {
                    bindings,
                    body: Box::new(self.decode_expression(&body)?),
                }
            }
            "expression.record" => {
                check_fields(&record, &["as", "type"])?;
                let nominal_type = optional(&record, "type")
                    .map(|_| parse_declaration_reference(&record, "type"))
                    .transpose()?;
                let mut fields = Vec::new();
                for edge in self.ordered_record_edges("expression.record-field", symbol)? {
                    let selector = parse_field_selector(&edge.record)?;
                    let value = required(&edge.record, "value")?.to_owned();
                    fields.push(AuthoredRecordExpressionField {
                        selector,
                        value: self.decode_expression(&value)?,
                    });
                }
                AuthoredExpressionOperation::Record {
                    nominal_type,
                    fields,
                }
            }
            "expression.variant" => {
                check_fields(&record, &["as", "case", "payload"])?;
                let payload = optional(&record, "payload")
                    .map(|value| self.decode_expression(value).map(Box::new))
                    .transpose()?;
                AuthoredExpressionOperation::Variant {
                    case: parse_case_reference(&record, "case")?,
                    payload,
                }
            }
            "expression.field" => {
                check_fields(&record, &["as", "value", "name", "field"])?;
                let value = required(&record, "value")?.to_owned();
                AuthoredExpressionOperation::Field {
                    value: Box::new(self.decode_expression(&value)?),
                    selector: parse_field_selector(&record)?,
                }
            }
            "expression.list" => {
                check_fields(&record, &["as", "item"])?;
                AuthoredExpressionOperation::List {
                    item_type: self.decode_type(required(&record, "item")?)?,
                    items: self.decode_expression_edges(symbol)?,
                }
            }
            "expression.map" => {
                check_fields(&record, &["as", "key", "value"])?;
                let mut entries = Vec::new();
                for edge in self.ordered_record_edges("expression.map-entry", symbol)? {
                    let key = required(&edge.record, "key")?.to_owned();
                    let value = required(&edge.record, "value")?.to_owned();
                    entries.push(AuthoredMapExpressionEntry {
                        key: self.decode_expression(&key)?,
                        value: self.decode_expression(&value)?,
                    });
                }
                AuthoredExpressionOperation::Map {
                    key_type: self.decode_type(required(&record, "key")?)?,
                    value_type: self.decode_type(required(&record, "value")?)?,
                    entries,
                }
            }
            "expression.match" => {
                check_fields(&record, &["as", "value"])?;
                let value = required(&record, "value")?.to_owned();
                let mut arms = Vec::new();
                for edge in self.ordered_record_edges("expression.match-arm", symbol)? {
                    let binding_symbol = optional(&edge.record, "as");
                    let binding_name = optional(&edge.record, "name");
                    let binding_type = optional(&edge.record, "type");
                    let payload_binding = match (binding_symbol, binding_name, binding_type) {
                        (None, None, None) => None,
                        (Some(_), Some(_), Some(binding_type)) => Some(AuthoredBindingDefinition {
                            symbol: symbol_field(&edge.record, "as")?,
                            name: parse_name(&edge.record, "name")?,
                            declared_type: Some(self.decode_type(binding_type)?),
                        }),
                        _ => {
                            return Err(record_error(
                                &edge.record,
                                "change_match_binding",
                                "match arm payload binding requires as, name, and its exact type",
                            ));
                        }
                    };
                    let body = required(&edge.record, "body")?.to_owned();
                    arms.push(AuthoredMatchExpressionArm {
                        case: parse_case_reference(&edge.record, "case")?,
                        payload_binding,
                        body: self.decode_expression(&body)?,
                    });
                }
                AuthoredExpressionOperation::Match {
                    value: Box::new(self.decode_expression(&value)?),
                    arms,
                }
            }
            "expression.capability-call" => {
                check_fields(&record, &["as", "requirement", "operation"])?;
                AuthoredExpressionOperation::CapabilityCall {
                    requirement: parse_requirement_reference(&record, "requirement")?,
                    operation: parse_operation_reference(&record, "operation")?,
                    arguments: self.decode_expression_edges(symbol)?,
                }
            }
            "expression.transaction" => {
                check_fields(&record, &["as", "requirement", "binding", "name", "body"])?;
                let body = required(&record, "body")?.to_owned();
                AuthoredExpressionOperation::Transaction {
                    requirement: parse_requirement_reference(&record, "requirement")?,
                    binding: AuthoredBindingDefinition {
                        symbol: symbol_field(&record, "binding")?,
                        name: parse_name(&record, "name")?,
                        declared_type: None,
                    },
                    body: Box::new(self.decode_expression(&body)?),
                }
            }
            operation => {
                return Err(record_error(
                    &record,
                    "change_expression_form_unknown",
                    format!("unknown compact expression form '{operation}'"),
                ));
            }
        };
        self.expression_stack.remove(symbol);
        Ok(AuthoredExpression {
            symbol: Some(symbol.to_owned()),
            operation,
        })
    }

    fn decode_expression_edges(
        &mut self,
        parent: &str,
    ) -> Result<Vec<AuthoredExpression>, Diagnostic> {
        self.ordered_edges(parent, false)?
            .into_iter()
            .map(|edge| self.decode_expression(&edge.value))
            .collect()
    }

    fn ordered_edges(
        &mut self,
        parent: &str,
        type_edges: bool,
    ) -> Result<Vec<IndexedValue>, Diagnostic> {
        let mut edges = if type_edges {
            self.type_parameters.get(parent)
        } else {
            self.arguments.get(parent)
        }
        .cloned()
        .unwrap_or_default();
        edges.sort_by_key(|edge| edge.index);
        for (expected, edge) in edges.iter().enumerate() {
            if edge.index != expected {
                return Err(Diagnostic::source(
                    "change_edge_index_order",
                    format!(
                        "parent '{parent}' child indexes must be contiguous from zero; expected {expected}, observed {}",
                        edge.index
                    ),
                    edge.location.clone(),
                ));
            }
        }
        self.consumed_value_edges
            .insert((type_edges, parent.to_owned()));
        Ok(edges)
    }

    fn ordered_record_edges(
        &mut self,
        operation: &str,
        parent: &str,
    ) -> Result<Vec<IndexedRecord>, Diagnostic> {
        let mut edges = self
            .record_edges
            .get(operation)
            .and_then(|parents| parents.get(parent))
            .cloned()
            .unwrap_or_default();
        edges.sort_by_key(|edge| edge.index);
        for (expected, edge) in edges.iter().enumerate() {
            if edge.index != expected {
                return Err(Diagnostic::source(
                    "change_edge_index_order",
                    format!(
                        "parent '{parent}' child indexes must be contiguous from zero; expected {expected}, observed {}",
                        edge.index
                    ),
                    edge.record.location.clone(),
                ));
            }
        }
        self.consumed_record_edges
            .insert((operation.to_owned(), parent.to_owned()));
        Ok(edges)
    }
}

fn is_change_precondition(operation: &str) -> bool {
    COMPACT_CHANGE_PRECONDITIONS.contains(&operation)
}

fn decode_precondition(record: &CompactRecord) -> Result<AuthoredPrecondition, Diagnostic> {
    check_precondition_fields(record, COMPACT_CHANGE_PRECONDITION_FIELDS)?;
    match record.operation.as_str() {
        "precondition.owner-exists" => Ok(AuthoredPrecondition::OwnerExists {
            owner: parse_field(record, "owner")?,
        }),
        "precondition.owner-absent" => Ok(AuthoredPrecondition::OwnerAbsent {
            owner: parse_field(record, "owner")?,
        }),
        "precondition.owner-name" => Ok(AuthoredPrecondition::OwnerName {
            owner: parse_field(record, "owner")?,
            equals: parse_name(record, "name")?,
        }),
        "precondition.owner-parent" => Ok(AuthoredPrecondition::OwnerParent {
            owner: parse_field(record, "owner")?,
            equals: parse_owner_parent(record, "parent")?,
        }),
        "precondition.namespace-absent" => Ok(AuthoredPrecondition::NamespaceAbsent {
            parent: parse_namespace_parent(record, "parent")?,
            class: parse_namespace_class(record, "class")?,
            name: parse_name(record, "name")?,
        }),
        "precondition.namespace-points-to" => Ok(AuthoredPrecondition::NamespacePointsTo {
            parent: parse_namespace_parent(record, "parent")?,
            class: parse_namespace_class(record, "class")?,
            name: parse_name(record, "name")?,
            owner: parse_field(record, "owner")?,
        }),
        "precondition.dependency-binding" => Ok(AuthoredPrecondition::DependencyBinding {
            package: parse_field::<PackageId>(record, "package")?,
            semantic_revision: parse_field::<RevisionId>(record, "semantic-revision")?,
            package_revision: parse_field::<PackageRevisionDigest>(record, "package-revision")?,
        }),
        _ => Err(record_error(
            record,
            "change_precondition_unknown",
            format!("unknown compact precondition '{}'", record.operation),
        )),
    }
}

pub(crate) fn normalize_change_request(
    semantic: AuthoredChangeSet,
    options: PublicationOptions,
) -> Result<NormalizedChangeRequest, Diagnostic> {
    if let Some(key) = options.idempotency_key.as_deref()
        && !idempotency_key_is_valid(key)
    {
        return Err(Diagnostic::new(
            DiagnosticClass::Source,
            "change_idempotency",
            "idempotency must contain 1 through 128 portable identifier bytes",
        ));
    }
    if options.intent.as_ref().is_some_and(|value| {
        value.len() > crate::platform::publication::contract::MAXIMUM_INTENT_BYTES
    }) {
        return Err(Diagnostic::new(
            DiagnosticClass::Source,
            "change_intent_bytes",
            "intent exceeds its 4096-byte operational bound",
        ));
    }
    let request_commitment = change_request_commitment(&semantic, &options)?;
    Ok(NormalizedChangeRequest {
        semantic,
        options,
        request_commitment,
    })
}

fn change_request_commitment(
    request: &AuthoredChangeSet,
    options: &PublicationOptions,
) -> Result<ChangeRequestCommitment, Diagnostic> {
    let intent = crate::platform::change::canonical_authored_intent_bytes(request)?;
    let budget = crate::platform::change::canonical_authored_budget_bytes(request.budget)?;
    let mut hasher = blake3::Hasher::new_derive_key(CHANGE_REQUEST_COMMITMENT_DOMAIN);
    hash_digest_field(&mut hasher, AUTHORED_CHANGE_CODEC_IDENTITY.as_bytes())?;
    hash_digest_field(&mut hasher, &intent)?;
    hash_digest_field(&mut hasher, &budget)?;
    hash_optional_digest_field(&mut hasher, options.idempotency_key.as_deref())?;
    hash_optional_digest_field(&mut hasher, options.intent.as_deref())?;
    Ok(ChangeRequestCommitment(*hasher.finalize().as_bytes()))
}

fn hash_optional_digest_field(
    hasher: &mut blake3::Hasher,
    value: Option<&str>,
) -> Result<(), Diagnostic> {
    match value {
        Some(value) => {
            hasher.update(&[1]);
            hash_digest_field(hasher, value.as_bytes())
        }
        None => {
            hasher.update(&[0]);
            Ok(())
        }
    }
}

fn hash_digest_field(hasher: &mut blake3::Hasher, value: &[u8]) -> Result<(), Diagnostic> {
    let length = u64::try_from(value.len()).map_err(|_| {
        Diagnostic::new(
            DiagnosticClass::Resource,
            "change_request_commitment_field_length",
            "normalized request commitment field exceeds its digest length domain",
        )
    })?;
    hasher.update(&length.to_be_bytes());
    hasher.update(value);
    Ok(())
}

fn check_fields(record: &CompactRecord, allowed: &[&str]) -> Result<(), Diagnostic> {
    for field in &record.fields {
        if !allowed.contains(&field.name.as_str()) {
            return Err(Diagnostic::source(
                "change_field_unknown",
                format!(
                    "record '{}' does not define field '{}'; use 'capabilities change'",
                    record.operation, field.name
                ),
                field.location.clone(),
            ));
        }
    }
    Ok(())
}

fn check_operation_fields(
    record: &CompactRecord,
    descriptors: &[CompactChangeOperationField],
) -> Result<(), Diagnostic> {
    for field in &record.fields {
        if !descriptors
            .iter()
            .any(|descriptor| descriptor.name == field.name)
        {
            return Err(field_error(
                record,
                &field.name,
                "change_field_unknown",
                format!(
                    "operation '{}' does not accept field '{}'",
                    record.operation, field.name
                ),
            ));
        }
    }
    for descriptor in descriptors {
        if descriptor.required {
            required(record, descriptor.name)?;
        }
    }
    Ok(())
}

fn check_precondition_fields(
    record: &CompactRecord,
    fields: &[CompactChangePreconditionField],
) -> Result<(), Diagnostic> {
    let descriptors = fields
        .iter()
        .filter(|descriptor| descriptor.record == record.operation)
        .collect::<Vec<_>>();
    for field in &record.fields {
        if !descriptors
            .iter()
            .any(|descriptor| descriptor.name == field.name)
        {
            return Err(field_error(
                record,
                &field.name,
                "change_field_unknown",
                format!(
                    "precondition '{}' does not accept field '{}'",
                    record.operation, field.name
                ),
            ));
        }
    }
    for descriptor in descriptors {
        if descriptor.required {
            required(record, descriptor.name)?;
        }
    }
    Ok(())
}

fn required<'a>(record: &'a CompactRecord, name: &str) -> Result<&'a str, Diagnostic> {
    optional(record, name).ok_or_else(|| {
        record_error(
            record,
            "change_field_missing",
            format!("record '{}' requires field '{name}'", record.operation),
        )
    })
}

fn optional<'a>(record: &'a CompactRecord, name: &str) -> Option<&'a str> {
    record
        .fields
        .iter()
        .find(|field| field.name == name)
        .map(|field| field.value.as_str())
}

fn field<'a>(record: &'a CompactRecord, name: &str) -> Option<&'a CompactField> {
    record.fields.iter().find(|field| field.name == name)
}

fn parse_field<T>(record: &CompactRecord, name: &str) -> Result<T, Diagnostic>
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    let value = required(record, name)?;
    value.parse().map_err(|error| {
        field_error(
            record,
            name,
            "change_field_value",
            format!("field '{name}' has invalid value '{value}': {error}"),
        )
    })
}

fn parse_bool(record: &CompactRecord, name: &str) -> Result<bool, Diagnostic> {
    match required(record, name)? {
        "true" => Ok(true),
        "false" => Ok(false),
        value => Err(field_error(
            record,
            name,
            "change_boolean",
            format!("field '{name}' requires true or false, observed '{value}'"),
        )),
    }
}

fn parse_name(record: &CompactRecord, field_name: &str) -> Result<Name, Diagnostic> {
    let value = required(record, field_name)?;
    Name::new(value).map_err(|error| field_error(record, field_name, error.code, error.message))
}

fn parse_owner_parent(
    record: &CompactRecord,
    field_name: &str,
) -> Result<AuthoredOwnerParent, Diagnostic> {
    let value = required(record, field_name)?;
    if value == "package" {
        Ok(AuthoredOwnerParent::Package)
    } else {
        parse_field(record, field_name).map(AuthoredOwnerParent::Owner)
    }
}

fn parse_http_route_selector(record: &CompactRecord) -> Result<HttpRouteSelector, Diagnostic> {
    match (optional(record, "path"), optional(record, "pattern")) {
        (Some(path), None) => HttpRouteSelector::exact(path.to_owned()),
        (None, Some(pattern)) => HttpRouteSelector::parse_pattern(pattern),
        _ => Err(record_error(
            record,
            "change_http_route_selector",
            "HTTP route mutation requires exactly one of path or pattern",
        )),
    }
}

fn parse_namespace_parent(
    record: &CompactRecord,
    field_name: &str,
) -> Result<Option<OwnerKey>, Diagnostic> {
    match parse_owner_parent(record, field_name)? {
        AuthoredOwnerParent::Package => Ok(None),
        AuthoredOwnerParent::Owner(owner) => Ok(Some(owner)),
    }
}

fn parse_namespace_class(
    record: &CompactRecord,
    field_name: &str,
) -> Result<NamespaceClass, Diagnostic> {
    let value = required(record, field_name)?;
    COMPACT_NAMESPACE_CLASSES
        .iter()
        .find_map(|(name, class)| (*name == value).then_some(*class))
        .ok_or_else(|| {
            field_error(
                record,
                field_name,
                "change_precondition_namespace_class",
                format!("unknown namespace class '{value}'; use focused change discovery"),
            )
        })
}

fn symbol(record: &CompactRecord, name: &str) -> Result<String, Diagnostic> {
    let value = required(record, name)?.to_owned();
    validate_local_label(record, name, &value, '$')?;
    Ok(value)
}

fn symbol_field(record: &CompactRecord, name: &str) -> Result<String, Diagnostic> {
    symbol(record, name)
}

fn validate_local_label(
    record: &CompactRecord,
    field_name: &str,
    value: &str,
    prefix: char,
) -> Result<(), Diagnostic> {
    let mut characters = value.chars();
    if characters.next() != Some(prefix)
        || !characters
            .clone()
            .next()
            .is_some_and(|value| value.is_ascii_alphabetic() || value == '_')
        || !characters.all(|value| value.is_ascii_alphanumeric() || value == '_' || value == '-')
        || value.len() > 128
    {
        return Err(field_error(
            record,
            field_name,
            "change_local_label",
            format!(
                "field '{field_name}' requires {prefix} followed by 1 through 127 portable identifier bytes"
            ),
        ));
    }
    Ok(())
}

fn validate_fragment_parent(
    record: &CompactRecord,
    field_name: &str,
    value: &str,
) -> Result<(), Diagnostic> {
    let Some(prefix) = value.chars().next() else {
        return Err(field_error(
            record,
            field_name,
            "change_edge_parent",
            "edge parent requires a request-local $, @, or % label",
        ));
    };
    if !['$', '@', '%'].contains(&prefix) {
        return Err(field_error(
            record,
            field_name,
            "change_edge_parent",
            "edge parent requires a request-local $, @, or % label",
        ));
    }
    validate_local_label(record, field_name, value, prefix)
}

fn fragment(record: &CompactRecord, name: &str) -> Result<String, Diagnostic> {
    let value = required(record, name)?.to_owned();
    validate_local_label(record, name, &value, '%')?;
    Ok(value)
}

fn parse_visibility(
    record: &CompactRecord,
    field_name: &str,
) -> Result<DeclarationVisibility, Diagnostic> {
    let value = required(record, field_name)?;
    COMPACT_DECLARATION_VISIBILITIES
        .iter()
        .find_map(|(name, visibility)| (*name == value).then_some(*visibility))
        .ok_or_else(|| {
            field_error(
                record,
                field_name,
                "change_visibility",
                format!("visibility must be private, package, or public; observed '{value}'"),
            )
        })
}

fn parse_idempotency(record: &CompactRecord, field_name: &str) -> Result<Idempotency, Diagnostic> {
    match required(record, field_name)? {
        "idempotent" => Ok(Idempotency::Idempotent),
        "idempotent-with-key" => Ok(Idempotency::IdempotentWithKey),
        "non-idempotent" => Ok(Idempotency::NonIdempotent),
        value => Err(field_error(
            record,
            field_name,
            "change_idempotency_class",
            format!(
                "idempotency must be idempotent, idempotent-with-key, or non-idempotent; observed '{value}'"
            ),
        )),
    }
}

fn parse_external_visibility(
    record: &CompactRecord,
    field_name: &str,
) -> Result<ExternalVisibility, Diagnostic> {
    match required(record, field_name)? {
        "none" => Ok(ExternalVisibility::None),
        "possible" => Ok(ExternalVisibility::Possible),
        value => Err(field_error(
            record,
            field_name,
            "change_external_visibility",
            format!("external visibility must be none or possible; observed '{value}'"),
        )),
    }
}

fn parse_parameter_use(
    record: &CompactRecord,
    field_name: &str,
) -> Result<ParameterUse, Diagnostic> {
    match required(record, field_name)? {
        "unrestricted" => Ok(ParameterUse::Unrestricted),
        "borrow" => Ok(ParameterUse::Borrow),
        "consume" => Ok(ParameterUse::Consume),
        value => Err(field_error(
            record,
            field_name,
            "change_parameter_use",
            format!("parameter use must be unrestricted, borrow, or consume; observed '{value}'"),
        )),
    }
}

fn parse_implementation_name(
    record: &CompactRecord,
    field_name: &str,
) -> Result<ImplementationName, Diagnostic> {
    ImplementationName::new(required(record, field_name)?)
        .map_err(|error| field_error(record, field_name, error.code, error.message))
}

fn parse_module_selector(
    record: &CompactRecord,
    field_name: &str,
) -> Result<ModuleSelector, Diagnostic> {
    let value = required(record, field_name)?;
    if value.starts_with('$') {
        validate_local_label(record, field_name, value, '$')?;
        Ok(ModuleSelector::Symbol {
            symbol: value.to_owned(),
        })
    } else if value.starts_with(ModuleId::PREFIX) {
        Ok(ModuleSelector::Id {
            module: parse_field(record, field_name)?,
        })
    } else {
        Ok(ModuleSelector::Name {
            name: parse_name(record, field_name)?,
        })
    }
}

fn parse_declaration_selector(
    record: &CompactRecord,
    field_name: &str,
) -> Result<DeclarationSelector, Diagnostic> {
    let value = required(record, field_name)?;
    if value.starts_with('$') {
        validate_local_label(record, field_name, value, '$')?;
        Ok(DeclarationSelector::Symbol {
            symbol: value.to_owned(),
        })
    } else if value.starts_with(DeclarationId::PREFIX) {
        Ok(DeclarationSelector::Id {
            declaration: parse_field(record, field_name)?,
        })
    } else if let Some((module, name)) = value.split_once('/') {
        let module = if module.starts_with(ModuleId::PREFIX) {
            ModuleSelector::Id {
                module: module.parse().map_err(|error: Diagnostic| {
                    field_error(record, field_name, error.code, error.message)
                })?,
            }
        } else {
            ModuleSelector::Name {
                name: Name::new(module)
                    .map_err(|error| field_error(record, field_name, error.code, error.message))?,
            }
        };
        Ok(DeclarationSelector::Qualified {
            module,
            name: Name::new(name)
                .map_err(|error| field_error(record, field_name, error.code, error.message))?,
        })
    } else {
        Err(field_error(
            record,
            field_name,
            "change_declaration_selector",
            "declaration selector requires $symbol, decl_ID, or MODULE/NAME",
        ))
    }
}

fn parse_owner_selector(
    record: &CompactRecord,
    field_name: &str,
) -> Result<OwnerSelector, Diagnostic> {
    let value = required(record, field_name)?;
    if value.starts_with('$') {
        validate_local_label(record, field_name, value, '$')?;
        Ok(OwnerSelector::Symbol {
            symbol: value.to_owned(),
        })
    } else {
        Ok(OwnerSelector::Exact {
            owner: parse_field(record, field_name)?,
        })
    }
}

fn parse_declaration_reference(
    record: &CompactRecord,
    field_name: &str,
) -> Result<AuthoredDeclarationReference, Diagnostic> {
    let value = required(record, field_name)?;
    if let Some((package, declaration)) = value.split_once('/') {
        return Ok(AuthoredDeclarationReference::Exact {
            package: package.parse().map_err(|error: Diagnostic| {
                field_error(record, field_name, error.code, error.message)
            })?,
            declaration: declaration.parse().map_err(|error: Diagnostic| {
                field_error(record, field_name, error.code, error.message)
            })?,
        });
    }
    Ok(AuthoredDeclarationReference::Local {
        declaration: parse_declaration_selector(record, field_name)?,
    })
}

fn parse_port_reference(
    record: &CompactRecord,
    field_name: &str,
) -> Result<AuthoredPortReference, Diagnostic> {
    let value = required(record, field_name)?;
    if value.starts_with('$') {
        validate_local_label(record, field_name, value, '$')?;
        return Ok(AuthoredPortReference::Symbol {
            symbol: value.to_owned(),
        });
    }
    let (package, port) = value.split_once('/').ok_or_else(|| {
        field_error(
            record,
            field_name,
            "change_port_reference",
            "port reference requires $symbol or pkg_ID/port_ID",
        )
    })?;
    Ok(AuthoredPortReference::Exact {
        package: package.parse().map_err(|error: Diagnostic| {
            field_error(record, field_name, error.code, error.message)
        })?,
        port: port.parse::<PortId>().map_err(|error| {
            field_error(
                record,
                field_name,
                "change_port_reference",
                error.to_string(),
            )
        })?,
    })
}

fn parse_runner_kind(record: &CompactRecord, field_name: &str) -> Result<RunnerKind, Diagnostic> {
    match required(record, field_name)? {
        "command" => Ok(RunnerKind::Command),
        "http" => Ok(RunnerKind::Http),
        "interactive" => Ok(RunnerKind::Interactive),
        value => Err(field_error(
            record,
            field_name,
            "change_runner_kind",
            format!("runner must be command, http, or interactive; observed '{value}'"),
        )),
    }
}

fn parse_field_reference(
    record: &CompactRecord,
    field_name: &str,
) -> Result<AuthoredFieldReference, Diagnostic> {
    let value = required(record, field_name)?;
    if value.starts_with('$') {
        validate_local_label(record, field_name, value, '$')?;
        return Ok(AuthoredFieldReference::Symbol {
            symbol: value.to_owned(),
        });
    }
    let (package, field) = value.split_once('/').ok_or_else(|| {
        field_error(
            record,
            field_name,
            "change_field_reference",
            "field reference requires $symbol or pkg_ID/field_ID",
        )
    })?;
    Ok(AuthoredFieldReference::Exact {
        package: package.parse().map_err(|error: Diagnostic| {
            field_error(record, field_name, error.code, error.message)
        })?,
        field: field.parse::<FieldId>().map_err(|error| {
            field_error(
                record,
                field_name,
                "change_field_reference",
                error.to_string(),
            )
        })?,
    })
}

fn parse_case_reference(
    record: &CompactRecord,
    field_name: &str,
) -> Result<AuthoredCaseReference, Diagnostic> {
    let value = required(record, field_name)?;
    if value.starts_with('$') {
        validate_local_label(record, field_name, value, '$')?;
        return Ok(AuthoredCaseReference::Symbol {
            symbol: value.to_owned(),
        });
    }
    let (package, case) = value.split_once('/').ok_or_else(|| {
        field_error(
            record,
            field_name,
            "change_case_reference",
            "case reference requires $symbol or pkg_ID/case_ID",
        )
    })?;
    Ok(AuthoredCaseReference::Exact {
        package: package.parse().map_err(|error: Diagnostic| {
            field_error(record, field_name, error.code, error.message)
        })?,
        case: case.parse::<CaseId>().map_err(|error| {
            field_error(
                record,
                field_name,
                "change_case_reference",
                error.to_string(),
            )
        })?,
    })
}

fn parse_operation_reference(
    record: &CompactRecord,
    field_name: &str,
) -> Result<AuthoredOperationReference, Diagnostic> {
    let value = required(record, field_name)?;
    if value.starts_with('$') {
        validate_local_label(record, field_name, value, '$')?;
        return Ok(AuthoredOperationReference::Symbol {
            symbol: value.to_owned(),
        });
    }
    let (package, operation) = value.split_once('/').ok_or_else(|| {
        field_error(
            record,
            field_name,
            "change_operation_reference",
            "operation reference requires $symbol or pkg_ID/op_ID",
        )
    })?;
    Ok(AuthoredOperationReference::Exact {
        package: package.parse().map_err(|error: Diagnostic| {
            field_error(record, field_name, error.code, error.message)
        })?,
        operation: operation.parse::<OperationId>().map_err(|error| {
            field_error(
                record,
                field_name,
                "change_operation_reference",
                error.to_string(),
            )
        })?,
    })
}

fn parse_requirement_reference(
    record: &CompactRecord,
    field_name: &str,
) -> Result<AuthoredRequirementReference, Diagnostic> {
    let value = required(record, field_name)?;
    if value.starts_with('$') {
        validate_local_label(record, field_name, value, '$')?;
        return Ok(AuthoredRequirementReference::Symbol {
            symbol: value.to_owned(),
        });
    }
    let (package, requirement) = value.split_once('/').ok_or_else(|| {
        field_error(
            record,
            field_name,
            "change_requirement_reference",
            "requirement reference requires $symbol or pkg_ID/req_ID",
        )
    })?;
    Ok(AuthoredRequirementReference::Exact {
        package: package.parse().map_err(|error: Diagnostic| {
            field_error(record, field_name, error.code, error.message)
        })?,
        requirement: requirement.parse::<RequirementId>().map_err(|error| {
            field_error(
                record,
                field_name,
                "change_requirement_reference",
                error.to_string(),
            )
        })?,
    })
}

fn parse_field_selector(record: &CompactRecord) -> Result<AuthoredFieldSelector, Diagnostic> {
    match (optional(record, "name"), optional(record, "field")) {
        (Some(_), None) => Ok(AuthoredFieldSelector::Structural {
            name: parse_name(record, "name")?,
        }),
        (None, Some(_)) => Ok(AuthoredFieldSelector::Nominal {
            field: parse_field_reference(record, "field")?,
        }),
        _ => Err(record_error(
            record,
            "change_field_selector",
            "field selection requires exactly one of name or field",
        )),
    }
}

fn parse_resource_unit(
    record: &CompactRecord,
    field_name: &str,
) -> Result<ResourceUnit, Diagnostic> {
    match required(record, field_name)? {
        "bytes" => Ok(ResourceUnit::Bytes),
        "items" => Ok(ResourceUnit::Items),
        "calls" => Ok(ResourceUnit::Calls),
        "tasks" => Ok(ResourceUnit::Tasks),
        "milliseconds" => Ok(ResourceUnit::Milliseconds),
        value => Err(field_error(
            record,
            field_name,
            "change_resource_unit",
            format!(
                "resource unit must be bytes, items, calls, tasks, or milliseconds; observed '{value}'"
            ),
        )),
    }
}

fn parse_type_parameter_reference(
    record: &CompactRecord,
    field_name: &str,
) -> Result<AuthoredTypeParameterReference, Diagnostic> {
    let value = required(record, field_name)?;
    if value.starts_with('$') {
        validate_local_label(record, field_name, value, '$')?;
        Ok(AuthoredTypeParameterReference::Symbol {
            symbol: value.to_owned(),
        })
    } else {
        Ok(AuthoredTypeParameterReference::Id {
            parameter: parse_field(record, field_name)?,
        })
    }
}

fn parse_local_reference(
    record: &CompactRecord,
    field_name: &str,
) -> Result<AuthoredLocalReference, Diagnostic> {
    let value = required(record, field_name)?;
    if value.starts_with('$') {
        validate_local_label(record, field_name, value, '$')?;
        return Ok(AuthoredLocalReference::Symbol {
            symbol: value.to_owned(),
        });
    }
    if value.starts_with(ParameterId::PREFIX) {
        return Ok(AuthoredLocalReference::FunctionParameter {
            parameter: parse_field(record, field_name)?,
        });
    }
    if value.starts_with(BindingId::PREFIX) {
        return Ok(AuthoredLocalReference::LexicalBinding {
            binding: parse_field(record, field_name)?,
        });
    }
    Err(field_error(
        record,
        field_name,
        "change_local_reference",
        "local reference requires $symbol, param_ID, or bind_ID",
    ))
}

fn record_error(
    record: &CompactRecord,
    code: impl Into<String>,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::source(code, message, record.location.clone())
}

fn field_error(
    record: &CompactRecord,
    field_name: &str,
    code: impl Into<String>,
    message: impl Into<String>,
) -> Diagnostic {
    let location = field(record, field_name)
        .map(|field| field.location.clone())
        .unwrap_or_else(|| record.location.clone());
    Diagnostic::source(code, message, location)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn revision() -> RevisionId {
        RevisionId::from_digest([7; 32])
    }

    #[test]
    fn connected_creation_decodes_to_one_typed_request_and_stable_plan() {
        let input = format!(
            "request base={} idempotency=connected-1\n\
             create.module as=$notes name=notes\n\
             create.record as=$note module=$notes name=Note visibility=public\n\
             add.field as=$text record=$note name=text type=text\n\
             expression.local as=$read value=$value\n\
             create.function as=$make module=$notes name=make visibility=public result=text effect=pure body=$read\n\
             add.parameter as=$value function=$make name=value type=text\n",
            revision()
        );
        let decoded = decode_compact_change("change.lk", input.as_bytes()).unwrap();
        assert_eq!(decoded.semantic.base, revision());
        assert_eq!(decoded.semantic.changes.len(), 5);
        assert_eq!(
            decoded.options.idempotency_key.as_deref(),
            Some("connected-1")
        );
        let repeated = decode_compact_change("other.lk", input.as_bytes()).unwrap();
        assert_eq!(decoded.request_commitment, repeated.request_commitment);
    }

    #[test]
    fn dependency_component_function_port_and_target_decode_as_one_forward_referenced_request() {
        let standard =
            crate::platform::builtin_standard::BuiltinStandard::load().expect("built-in standard");
        let input = format!(
            "request base={} idempotency=topology-1\n\
             create.target as=$target name=main component=$component port=$port runner=command\n\
             add.port as=$port component=$component name=main type=@entry function=$entry\n\
             type.function as=@entry result=text\n\
             expression.text as=$body value=hello\n\
             create.function as=$entry module=$module name=entry visibility=private result=text effect=pure body=$body\n\
             create.component as=$component module=$module name=application visibility=package\n\
             create.module as=$module name=application\n\
             add.dependency package={} semantic-revision={} package-revision={}\n",
            revision(),
            standard.package,
            standard.semantic_revision,
            standard.package_revision,
        );
        let decoded = decode_compact_change("topology.lkjc", input.as_bytes()).unwrap();
        assert_eq!(decoded.semantic.changes.len(), 6);
        assert!(matches!(
            &decoded.semantic.changes[0],
            AuthoredChange::CreateTarget {
                runner: RunnerKind::Command,
                component: AuthoredDeclarationReference::Local { .. },
                port: Some(AuthoredPortReference::Symbol { symbol }),
                ..
            } if symbol == "$port"
        ));
        assert!(matches!(
            &decoded.semantic.changes[1],
            AuthoredChange::AddPort {
                port: AuthoredPort {
                    implementation: AuthoredPortImplementation::Function { .. },
                    ..
                },
                ..
            }
        ));
        assert!(matches!(
            &decoded.semantic.changes[5],
            AuthoredChange::AddDependency {
                package,
                semantic_revision,
                package_revision,
            } if *package == standard.package
                && *semantic_revision == standard.semantic_revision
                && *package_revision == standard.package_revision
        ));
        let repeated = decode_compact_change("repeated.lkjc", input.as_bytes()).unwrap();
        assert_eq!(decoded.request_commitment, repeated.request_commitment);
    }

    #[test]
    fn topology_vocabulary_and_dependency_binding_fail_closed() {
        let standard =
            crate::platform::builtin_standard::BuiltinStandard::load().expect("built-in standard");
        let foreign = PackageId::migrate(b"foreign-public-dependency", 1);
        let unsupported = format!(
            "request base={}\nadd.dependency package={} semantic-revision={} package-revision={}\n",
            revision(),
            foreign,
            standard.semantic_revision,
            standard.package_revision,
        );
        // Parsing admits exact typed intent. Repository admission, not embedded identity,
        // owns verification of the package/revision binding and complete immutable source.
        assert!(decode_compact_change("dependency.lkjc", unsupported.as_bytes()).is_ok());

        let bad_runner = format!(
            "request base={}\ncreate.target as=$target name=serve component=$component port=$port runner=worker\n",
            revision()
        );
        assert_eq!(
            decode_compact_change("runner.lkjc", bad_runner.as_bytes()).unwrap_err()[0].code,
            "change_runner_kind"
        );

        for predecessor in [
            "add.package",
            "create.service",
            "add.endpoint",
            "create.runner",
        ] {
            let input = format!("request base={}\n{predecessor} as=$old\n", revision());
            assert_eq!(
                decode_compact_change("predecessor.lkjc", input.as_bytes()).unwrap_err()[0].code,
                "change_operation_unknown"
            );
        }
    }

    #[test]
    fn interfaces_operations_and_generic_externals_decode_with_exact_contracts() {
        let input = format!(
            "request base={}\n\
             create.module as=$data name=data\n\
             create.interface as=$store module=$data name=DataStore visibility=public\n\
             add.operation as=$get interface=$store name=get result=bytes idempotency=idempotent external-visibility=none\n\
             add.parameter as=$key operation=$get name=key type=bytes\n\
             type.parameter as=@item parameter=$item-type\n\
             create.external as=$encode module=$data name=data-encode visibility=public result=bytes implementation=core.data.encode\n\
             add.type-parameter as=$item-type function=$encode name=Item\n\
             add.parameter as=$value function=$encode name=value type=@item\n",
            revision()
        );
        let decoded = decode_compact_change("data.lkjc", input.as_bytes()).unwrap();
        assert_eq!(decoded.semantic.changes.len(), 7);
        assert!(matches!(
            &decoded.semantic.changes[1],
            AuthoredChange::CreateInterface { name, operations, .. }
                if name.as_str() == "DataStore" && operations.is_empty()
        ));
        assert!(matches!(
            &decoded.semantic.changes[2],
            AuthoredChange::AddOperation { operation, .. }
                if operation.name.as_str() == "get"
                    && operation.idempotency == Idempotency::Idempotent
                    && operation.external_visibility == ExternalVisibility::None
        ));
        assert!(matches!(
            &decoded.semantic.changes[3],
            AuthoredChange::AddParameter {
                parent: ParameterParentSelector::Operation { .. },
                ..
            }
        ));
        assert!(matches!(
            &decoded.semantic.changes[4],
            AuthoredChange::CreateExternal { implementation, .. }
                if implementation.as_str() == "core.data.encode"
        ));
    }

    #[test]
    fn generic_function_values_and_invocation_decode_through_one_public_vocabulary() {
        let input = format!(
            "request base={}\n\
             type.parameter as=@item parameter=$item_type\n\
             type.function as=@step result=@item\n\
             type.argument parent=@step index=0 type=@item\n\
             expression.local as=$step_local value=$step\n\
             expression.local as=$value_local value=$value\n\
             expression.invoke as=$apply_body function=$step_local\n\
             expression.argument parent=$apply_body index=0 expression=$value_local\n\
             create.module as=$module name=higher_order\n\
             create.function as=$apply module=$module name=apply visibility=private result=@item effect=pure body=$apply_body\n\
             add.type-parameter as=$item_type function=$apply name=Item\n\
             add.parameter as=$value function=$apply name=value type=@item\n\
             add.parameter as=$step function=$apply name=step type=@step\n\
             expression.local as=$keep_body value=$keep_value\n\
             create.function as=$keep module=$module name=keep visibility=private result=text effect=pure body=$keep_body\n\
             add.parameter as=$keep_value function=$keep name=value type=text\n\
             expression.function-value as=$apply_value function=$apply\n\
             type.argument parent=$apply_value index=0 type=text\n\
             expression.function-value as=$keep_value_expression function=$keep\n\
             expression.text as=$text value=kept\n\
             expression.invoke as=$entry_body function=$apply_value\n\
             expression.argument parent=$entry_body index=0 expression=$text\n\
             expression.argument parent=$entry_body index=1 expression=$keep_value_expression\n\
             create.function as=$entry module=$module name=entry visibility=public result=text effect=pure body=$entry_body\n",
            revision()
        );
        let decoded = decode_compact_change("higher-order.lkjc", input.as_bytes()).unwrap();
        assert_eq!(decoded.semantic.changes.len(), 8);
        assert!(matches!(
            &decoded.semantic.changes[2],
            AuthoredChange::AddTypeParameter { parameter, .. }
                if parameter.symbol == "$item_type" && parameter.name.as_str() == "Item"
        ));
        let AuthoredChange::CreateFunction { body, .. } = &decoded.semantic.changes[7] else {
            panic!("entry function")
        };
        let AuthoredExpressionOperation::Invoke { callee, arguments } = &body.operation else {
            panic!("entry invocation")
        };
        assert_eq!(arguments.len(), 2);
        assert!(matches!(
            callee.operation,
            AuthoredExpressionOperation::FunctionValue {
                ref type_arguments,
                ..
            } if type_arguments.len() == 1
        ));

        let repeated = decode_compact_change("other.lkjc", input.as_bytes()).unwrap();
        assert_eq!(decoded.request_commitment, repeated.request_commitment);
    }

    #[test]
    fn higher_order_aliases_and_incomplete_invoke_fail_closed() {
        for alias in ["function-ref", "lambda", "apply"] {
            let input = format!(
                "request base={}\n\
                 expression.{alias} as=$body function=$function\n\
                 create.module as=$module name=module\n\
                 create.function as=$function module=$module name=function visibility=private result=unit effect=pure body=$body\n",
                revision()
            );
            assert_eq!(
                decode_compact_change("alias.lkjc", input.as_bytes()).unwrap_err()[0].code,
                "change_expression_form_unknown"
            );
        }

        let missing_function = format!(
            "request base={}\n\
             expression.invoke as=$body\n\
             create.module as=$module name=module\n\
             create.function as=$function module=$module name=function visibility=private result=unit effect=pure body=$body\n",
            revision()
        );
        assert_eq!(
            decode_compact_change("invoke.lkjc", missing_function.as_bytes()).unwrap_err()[0].code,
            "change_field_missing"
        );
    }

    #[test]
    fn flat_expression_edges_are_ordered_and_nested_without_shared_authority() {
        let input = format!(
            "request base={}\n\
             expression.text as=$second value=second\n\
             expression.text as=$first value=first\n\
             expression.sequence as=$body\n\
             expression.argument parent=$body index=1 expression=$second\n\
             expression.argument parent=$body index=0 expression=$first\n\
             create.module as=$m name=m\n\
             create.function as=$f module=$m name=f visibility=private result=text effect=pure body=$body\n",
            revision()
        );
        let decoded = decode_compact_change("change.lk", input.as_bytes()).unwrap();
        let AuthoredChange::CreateFunction { body, .. } = &decoded.semantic.changes[1] else {
            panic!("function operation")
        };
        let AuthoredExpressionOperation::Sequence { items } = &body.operation else {
            panic!("sequence body")
        };
        assert_eq!(items.len(), 2);
        assert!(matches!(
            &items[0].operation,
            AuthoredExpressionOperation::Text { value } if value == "first"
        ));
    }

    #[test]
    fn map_expressions_and_entries_decode_through_the_public_vocabulary() {
        let input = format!(
            "request base={}\n\
             type.map as=@map key=text value=i64\n\
             expression.text as=$second_key value=second\n\
             expression.i64 as=$second_value value=2\n\
             expression.text as=$first_key value=first\n\
             expression.i64 as=$first_value value=1\n\
             expression.map as=$body key=text value=i64\n\
             expression.map-entry parent=$body index=1 key=$second_key value=$second_value\n\
             expression.map-entry parent=$body index=0 key=$first_key value=$first_value\n\
             create.module as=$module name=maps\n\
             create.function as=$function module=$module name=values visibility=private result=@map effect=pure body=$body\n",
            revision()
        );
        let decoded = decode_compact_change("map.lkjc", input.as_bytes()).unwrap();
        let AuthoredChange::CreateFunction { body, .. } = &decoded.semantic.changes[1] else {
            panic!("function operation")
        };
        let AuthoredExpressionOperation::Map {
            key_type,
            value_type,
            entries,
        } = &body.operation
        else {
            panic!("map body")
        };
        assert_eq!(*key_type, AuthoredType::Text {});
        assert_eq!(*value_type, AuthoredType::I64 {});
        assert_eq!(entries.len(), 2);
        assert!(matches!(
            &entries[0].key.operation,
            AuthoredExpressionOperation::Text { value } if value == "first"
        ));
        assert!(matches!(
            &entries[1].value.operation,
            AuthoredExpressionOperation::I64 { value: 2 }
        ));
    }

    #[test]
    fn task_capability_and_structural_slice_decodes_from_flat_records() {
        let package = PackageId::migrate(b"compact-task-package", 1);
        let component = DeclarationId::migrate(b"compact-task-component", 1);
        let handler = DeclarationId::migrate(b"compact-task-handler", 1);
        let interface = DeclarationId::migrate(b"compact-task-interface", 1);
        let operation = OperationId::migrate(b"compact-task-operation", 1);
        let case = CaseId::migrate(b"compact-task-case", 1);
        let input = format!(
            "request base={}\n\
             type.structural-record as=@response\n\
             type.field parent=@response index=0 name=body type=bytes\n\
             type.field parent=@response index=1 name=status type=i64\n\
             create.module as=$module name=application_state\n\
             add.requirement as=$store component={component} name=store interface={package}/{interface}\n\
             requirement.operation parent=$store index=0 operation={package}/{operation}\n\
             requirement.limit parent=$store index=0 name=calls maximum=32 unit=calls\n\
             expression.text as=$let-value value=value\n\
             expression.local as=$let-body value=$binding\n\
             expression.let as=$let body=$let-body\n\
             expression.binding parent=$let index=0 as=$binding name=binding value=$let-value type=text\n\
             expression.i64 as=$record-value value=200\n\
             expression.record as=$record\n\
             expression.record-field parent=$record index=0 name=status value=$record-value\n\
             expression.i64 as=$field-value value=201\n\
             expression.record as=$field-record\n\
             expression.record-field parent=$field-record index=0 name=status value=$field-value\n\
             expression.field as=$field value=$field-record name=status\n\
             expression.unit as=$list-item\n\
             expression.list as=$list item=unit\n\
             expression.argument parent=$list index=0 expression=$list-item\n\
             expression.text as=$match-payload value=payload\n\
             expression.variant as=$match-value case={package}/{case} payload=$match-payload\n\
             expression.local as=$match-body value=$matched\n\
             expression.match as=$match value=$match-value\n\
             expression.match-arm parent=$match index=0 case={package}/{case} as=$matched name=matched type=text body=$match-body\n\
             expression.unit as=$call-argument\n\
             expression.capability-call as=$capability requirement=$store operation={package}/{operation}\n\
             expression.argument parent=$capability index=0 expression=$call-argument\n\
             expression.unit as=$transaction-body\n\
             expression.transaction as=$transaction requirement=$store binding=$transaction-binding name=transaction body=$transaction-body\n\
             expression.sequence as=$body\n\
             expression.argument parent=$body index=0 expression=$let\n\
             expression.argument parent=$body index=1 expression=$record\n\
             expression.argument parent=$body index=2 expression=$field\n\
             expression.argument parent=$body index=3 expression=$list\n\
             expression.argument parent=$body index=4 expression=$match\n\
             expression.argument parent=$body index=5 expression=$capability\n\
             expression.argument parent=$body index=6 expression=$transaction\n\
             create.function as=$task module=$module name=task visibility=private result=unit effect=task body=$body\n\
             effect.requirement parent=$task index=0 requirement=$store\n\
             set.function-contract as=%handler function={handler} result=@response effect=task\n\
             effect.requirement parent=%handler index=0 requirement=$store\n",
            revision()
        );
        let decoded = decode_compact_change("task.lkjc", input.as_bytes()).unwrap();
        assert_eq!(decoded.semantic.changes.len(), 4);
        let AuthoredChange::AddRequirement { requirement, .. } = &decoded.semantic.changes[1]
        else {
            panic!("requirement change")
        };
        assert_eq!(requirement.operations.len(), 1);
        assert_eq!(requirement.limits.len(), 1);
        let AuthoredChange::CreateFunction { effect, body, .. } = &decoded.semantic.changes[2]
        else {
            panic!("task function")
        };
        assert!(matches!(
            effect,
            AuthoredFunctionEffect::Task { requirements } if requirements.len() == 1
        ));
        assert!(matches!(
            body.operation,
            AuthoredExpressionOperation::Sequence { ref items } if items.len() == 7
        ));
        let AuthoredChange::SetFunctionContract { result, effect, .. } =
            &decoded.semantic.changes[3]
        else {
            panic!("function contract")
        };
        assert!(matches!(result, AuthoredType::StructuralRecord { fields } if fields.len() == 2));
        assert!(matches!(
            effect,
            AuthoredFunctionEffect::Task { requirements } if requirements.len() == 1
        ));
    }

    #[test]
    fn task_edges_and_contract_fragments_fail_closed() {
        let pure_with_requirement = format!(
            "request base={}\n\
             expression.unit as=$body\n\
             create.module as=$module name=module\n\
             create.function as=$function module=$module name=function visibility=private result=unit effect=pure body=$body\n\
             effect.requirement parent=$function index=0 requirement=$requirement\n",
            revision()
        );
        assert_eq!(
            decode_compact_change("pure.lkjc", pure_with_requirement.as_bytes()).unwrap_err()[0]
                .code,
            "change_edge_unconsumed"
        );

        let handler = DeclarationId::migrate(b"duplicate-contract-handler", 1);
        let duplicate = format!(
            "request base={}\n\
             set.function-contract as=%contract function={handler} result=unit effect=pure\n\
             set.function-contract as=%contract function={handler} result=unit effect=pure\n",
            revision()
        );
        assert_eq!(
            decode_compact_change("duplicate.lkjc", duplicate.as_bytes()).unwrap_err()[0].code,
            "change_fragment_duplicate"
        );
    }

    #[test]
    fn malformed_records_report_exact_independent_locations() {
        let input = format!(
            "request base={}\ncreate.module as=$m name=m extra=no\n",
            revision()
        );
        let error = decode_compact_change("change.lk", input.as_bytes()).unwrap_err();
        assert_eq!(error[0].code, "change_field_unknown");
        assert_eq!(error[0].location.as_ref().unwrap().line, 2);
    }

    #[test]
    fn deletion_policies_are_exact_and_predecessor_forms_are_unknown() {
        let owner = OwnerKey::Module(ModuleId::migrate(b"compact-delete", 1));
        let input = format!(
            "request base={}\ndelete.owner owner={owner} policy=reject\n",
            revision()
        );
        let decoded = decode_compact_change("delete.lk", input.as_bytes()).unwrap();
        assert!(matches!(
            &decoded.semantic.changes[..],
            [AuthoredChange::DeleteOwner {
                owner: OwnerSelector::Exact { owner: observed },
                policy: AuthoredDeletePolicy::Reject,
            }] if *observed == owner
        ));

        let closure = format!(
            "request base={}\ndelete.owner owner={owner} policy=owned-closure\n",
            revision()
        );
        let decoded = decode_compact_change("delete.lk", closure.as_bytes()).unwrap();
        assert!(matches!(
            &decoded.semantic.changes[..],
            [AuthoredChange::DeleteOwner {
                owner: OwnerSelector::Exact { owner: observed },
                policy: AuthoredDeletePolicy::OwnedClosure,
            }] if *observed == owner
        ));
        for policy in ["cascade", "recursive", "deep"] {
            let unsupported = format!(
                "request base={}\ndelete.owner owner={owner} policy={policy}\n",
                revision()
            );
            assert_eq!(
                decode_compact_change("delete.lk", unsupported.as_bytes()).unwrap_err()[0].code,
                "change_delete_policy"
            );
        }
        let predecessor = format!(
            "request base={}\ndelete.owner owner={owner} cascade=true policy=reject\n",
            revision()
        );
        assert_eq!(
            decode_compact_change("delete.lk", predecessor.as_bytes()).unwrap_err()[0].code,
            "change_field_unknown"
        );
    }

    #[test]
    fn semantic_preconditions_are_typed_and_predecessor_forms_are_unknown() {
        let owner = OwnerKey::Module(ModuleId::migrate(b"compact-precondition-owner", 1));
        let target =
            OwnerKey::Declaration(DeclarationId::migrate(b"compact-precondition-target", 1));
        let package = PackageId::migrate(b"compact-precondition-package", 1);
        let semantic_revision = RevisionId::from_digest([0x44; 32]);
        let package_revision = PackageRevisionDigest::from_bytes([0x55; 32]);
        let input = format!(
            "request base={}\n\
             precondition.owner-exists owner={owner}\n\
             precondition.owner-absent owner={target}\n\
             precondition.owner-name owner={owner} name=module\n\
             precondition.owner-parent owner={owner} parent=package\n\
             precondition.namespace-absent parent=package class=module name=free\n\
             precondition.namespace-points-to parent={owner} class=declaration name=item owner={target}\n\
             precondition.dependency-binding package={package} semantic-revision={semantic_revision} package-revision={package_revision}\n\
             create.module as=$module name=created\n",
            revision()
        );
        let decoded = decode_compact_change("preconditions.lk", input.as_bytes()).unwrap();
        assert_eq!(decoded.semantic.preconditions.len(), 7);
        assert!(matches!(
            decoded.semantic.preconditions[0].clone(),
            AuthoredPrecondition::OwnerExists { owner: observed } if observed == owner
        ));
        assert!(matches!(
            decoded.semantic.preconditions[3].clone(),
            AuthoredPrecondition::OwnerParent {
                owner: observed,
                equals: AuthoredOwnerParent::Package,
            } if observed == owner
        ));
        assert!(matches!(
            decoded.semantic.preconditions[5].clone(),
            AuthoredPrecondition::NamespacePointsTo {
                parent: Some(observed_parent),
                class: NamespaceClass::Declaration,
                ref name,
                owner: observed_owner,
            } if observed_parent == owner && name.as_str() == "item" && observed_owner == target
        ));
        assert!(matches!(
            decoded.semantic.preconditions[6].clone(),
            AuthoredPrecondition::DependencyBinding {
                package: observed_package,
                semantic_revision: observed_semantic,
                package_revision: observed_package_revision,
            } if observed_package == package
                && observed_semantic == semantic_revision
                && observed_package_revision == package_revision
        ));

        let invalid_class = format!(
            "request base={}\nprecondition.namespace-absent parent=package class=unknown name=free\ncreate.module as=$module name=created\n",
            revision()
        );
        assert_eq!(
            decode_compact_change("preconditions.lk", invalid_class.as_bytes()).unwrap_err()[0]
                .code,
            "change_precondition_namespace_class"
        );
        let missing_owner = format!(
            "request base={}\nprecondition.owner-exists\ncreate.module as=$module name=created\n",
            revision()
        );
        assert_eq!(
            decode_compact_change("preconditions.lk", missing_owner.as_bytes()).unwrap_err()[0]
                .code,
            "change_field_missing"
        );

        for predecessor in [
            "precondition.semantic-root equals=old",
            "precondition.owner-digest owner=old equals=old",
            "precondition.owner-summary-digest owner=old equals=old",
            "precondition.dependency-digest package=old equals=old",
            "precondition.retirement-digest owner=old equals=old",
        ] {
            let input = format!(
                "request base={}\n{predecessor}\ncreate.module as=$module name=created\n",
                revision()
            );
            assert_eq!(
                decode_compact_change("predecessor.lk", input.as_bytes()).unwrap_err()[0].code,
                "change_precondition_unknown"
            );
        }
    }

    #[test]
    fn cycles_duplicate_edges_and_unused_expressions_fail_closed() {
        let cycle = format!(
            "request base={}\nexpression.if as=$body condition=$body when-true=$body when-false=$body\ncreate.module as=$m name=m\ncreate.function as=$f module=$m name=f visibility=private result=unit effect=pure body=$body\n",
            revision()
        );
        assert_eq!(
            decode_compact_change("cycle.lk", cycle.as_bytes()).unwrap_err()[0].code,
            "change_expression_cycle"
        );

        let unused = format!(
            "request base={}\nexpression.unit as=$unused\ncreate.module as=$m name=m\n",
            revision()
        );
        assert_eq!(
            decode_compact_change("unused.lk", unused.as_bytes()).unwrap_err()[0].code,
            "change_expression_unused"
        );
    }

    #[test]
    fn json_and_unknown_operations_are_not_alternate_inputs() {
        let json = br#"{"base":"rev_dead"}"#;
        assert_eq!(
            decode_compact_change("change.lk", json).unwrap_err()[0].code,
            "control_operation"
        );
        let unknown = format!("request base={}\ncreate.unknown as=$x\n", revision());
        assert_eq!(
            decode_compact_change("change.lk", unknown.as_bytes()).unwrap_err()[0].code,
            "change_operation_unknown"
        );
    }

    #[test]
    fn reviewed_change_plan_request_commitment_binds_budget_and_operational_options() {
        let input = format!(
            "request base={}\ncreate.module as=$module name=module\n",
            revision()
        );
        let decoded = decode_compact_change("change.lk", input.as_bytes()).unwrap();

        let mut budget_changed = decoded.semantic.clone();
        budget_changed.budget.canonical_reads.maximum_bytes -= 1;
        assert_ne!(
            decoded.request_commitment,
            change_request_commitment(&budget_changed, &decoded.options).unwrap()
        );

        let options_changed = PublicationOptions {
            idempotency_key: Some("reviewed-plan-option".to_owned()),
            intent: None,
        };
        assert_ne!(
            decoded.request_commitment,
            change_request_commitment(&decoded.semantic, &options_changed).unwrap()
        );
    }
}
