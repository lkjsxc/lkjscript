//! Closed compact-record adapter for normalized semantic changes.

use super::{CompactField, CompactRecord, parse_records};
use crate::platform::change::{
    AuthoredCase, AuthoredChange, AuthoredChangeSet, AuthoredDeclarationReference,
    AuthoredDeletePolicy, AuthoredExpression, AuthoredExpressionOperation, AuthoredField,
    AuthoredFunctionEffect, AuthoredLocalReference, AuthoredOwnerParent, AuthoredParameter,
    AuthoredPrecondition, AuthoredType, AuthoredTypeParameterReference, DeclarationSelector,
    ModuleSelector, OwnerSelector, ParameterParentSelector,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass, SourceLocation};
use crate::platform::kernel::{
    DeclarationVisibility, Name, NamespaceClass, OwnerKey, PackageId, PackageRevisionDigest,
};
use crate::platform::publication::{PublicationOptions, idempotency_key_is_valid};
use crate::platform::semantic_id::{BindingId, DeclarationId, ModuleId, ParameterId, RevisionId};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;

pub const COMPACT_CHANGE_CONTRACT_IDENTITY: &str = "lkjscript-change-records-3";
pub const AUTHORED_CHANGE_CODEC_IDENTITY: &str = "lkjscript-authored-change-codec-4";
pub const AUTHORED_CHANGE_CODEC_VERSION: u16 = 4;
pub const CHANGE_REQUEST_COMMITMENT_DOMAIN: &str = "lkjscript.change-request-commitment.v1";
pub const COMPACT_DELETE_POLICIES: &[&str] = &["reject"];
pub(crate) const COMPACT_DECLARATION_VISIBILITIES: &[(&str, DeclarationVisibility)] = &[
    ("private", DeclarationVisibility::Private),
    ("package", DeclarationVisibility::Package),
    ("public", DeclarationVisibility::Public),
];
pub(crate) const COMPACT_FUNCTION_EFFECTS: &[&str] = &["pure"];
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
    CreateFunction,
    CreateConstant,
    CreateTest,
    AddField,
    AddCase,
    AddParameter,
    DeleteOwner,
    RenameOwner,
    MoveDeclaration,
    ReplaceBody,
}

impl CompactChangeOperation {
    pub(crate) const ALL: [Self; 13] = [
        Self::CreateModule,
        Self::CreateRecord,
        Self::CreateVariant,
        Self::CreateFunction,
        Self::CreateConstant,
        Self::CreateTest,
        Self::AddField,
        Self::AddCase,
        Self::AddParameter,
        Self::DeleteOwner,
        Self::RenameOwner,
        Self::MoveDeclaration,
        Self::ReplaceBody,
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
}

impl CompactChangeFieldForm {
    pub(crate) const ALL: [Self; 16] = [
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
            Self::FunctionEffect => "pure",
            Self::TypeReference => "unit|bool|i64|bytes|text|static-text|secret|@NAME",
            Self::ExpressionReference => "$NAME",
            Self::DeletePolicy => "reject",
            Self::OwnerParent => "package|DOMAIN_HEX",
            Self::NamespaceClass => "change.namespace-class.name",
            Self::ExactPackage => "pkg_HEX",
            Self::ExactRevision => "rev_HEX",
            Self::ExactPackageRevision => "package_revision_HEX",
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

struct Decoder {
    records: Vec<CompactRecord>,
    types: BTreeMap<String, CompactRecord>,
    expressions: BTreeMap<String, CompactRecord>,
    arguments: BTreeMap<String, Vec<IndexedValue>>,
    type_parameters: BTreeMap<String, Vec<IndexedValue>>,
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
            CompactChangeOperation::CreateFunction => {
                let effect = required(record, "effect")?;
                if !COMPACT_FUNCTION_EFFECTS.contains(&effect) {
                    return Err(field_error(
                        record,
                        "effect",
                        "change_effect_unsupported",
                        "the current compact create.function record supports effect=pure",
                    ));
                }
                let body = required(record, "body")?.to_owned();
                Ok(AuthoredChange::CreateFunction {
                    symbol: symbol(record, "as")?,
                    module: parse_module_selector(record, "module")?,
                    name: parse_name(record, "name")?,
                    visibility: parse_visibility(record, "visibility")?,
                    type_parameters: Vec::new(),
                    parameters: Vec::new(),
                    result: self.decode_type(required(record, "result")?)?,
                    effect: AuthoredFunctionEffect::Pure {},
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
            CompactChangeOperation::AddParameter => Ok(AuthoredChange::AddParameter {
                parent: ParameterParentSelector::Declaration {
                    declaration: parse_declaration_selector(record, "function")?,
                },
                parameter: AuthoredParameter {
                    symbol: symbol(record, "as")?,
                    name: parse_name(record, "name")?,
                    ty: self.decode_type(required(record, "type")?)?,
                },
            }),
            CompactChangeOperation::DeleteOwner => {
                let policy = required(record, "policy")?;
                if policy != "reject" {
                    return Err(field_error(
                        record,
                        "policy",
                        "change_delete_policy",
                        format!("deletion policy must be reject; observed '{policy}'"),
                    ));
                }
                Ok(AuthoredChange::DeleteOwner {
                    owner: OwnerSelector::Exact {
                        owner: parse_field::<OwnerKey>(record, "owner")?,
                    },
                    policy: AuthoredDeletePolicy::Reject,
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
        &self,
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
    fn deletion_is_exact_and_reject_only() {
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

        let unsupported = format!(
            "request base={}\ndelete.owner owner={owner} policy=owned-closure\n",
            revision()
        );
        assert_eq!(
            decode_compact_change("delete.lk", unsupported.as_bytes()).unwrap_err()[0].code,
            "change_delete_policy"
        );
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
