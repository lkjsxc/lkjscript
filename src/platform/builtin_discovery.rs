//! Bounded, implementation-free discovery over the exact embedded standard package interface.

use super::builtin_standard::BuiltinStandard;
use super::control::render_record;
use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::kernel::{
    EncodedOwnerKey, ExternalVisibility, FunctionEffect, Idempotency, OwnerKey, OwnerKind,
    PackageId, PackageInterfaceDeclarationPayload, PackageInterfaceRecord, ParameterParent,
    TypeForm, TypeObjectDigest,
};
use base64::Engine;

pub(crate) const BUILTIN_QUERY_DEFAULT_ITEMS: usize = 50;
pub(crate) const BUILTIN_QUERY_MAXIMUM_ITEMS: usize = 10_000;
pub(crate) const BUILTIN_QUERY_MINIMUM_BYTES: usize = 1_536;
pub(crate) const BUILTIN_QUERY_DEFAULT_BYTES: usize = 64 * 1_024;
pub(crate) const BUILTIN_QUERY_MAXIMUM_BYTES: usize = 4 * 1_048_576;
pub(crate) const BUILTIN_CONTINUATION_MAXIMUM_BYTES: usize = 320;
pub(crate) const BUILTIN_QUERY_ORDERING: &str = "package-interface-owner-key-v1";
pub(crate) const BUILTIN_CONTINUATION_PREFIX: &str = "bcont_";
const BUILTIN_CONTINUATION_MAGIC: [u8; 8] = *b"LKJBCT01";
const BUILTIN_CONTINUATION_VERSION: u16 = 1;
const BUILTIN_SELECTOR_DOMAIN: &str = "lkjscript.builtin-owner-selector.v1";
const BUILTIN_CONTINUATION_DOMAIN: &str = "lkjscript.builtin-owner-continuation.v1";
const BUILTIN_CONTINUATION_RAW_BYTES: usize = 8 + 2 + 16 + 32 + 32 + 17 + 32;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DiscoveryRecord {
    pub(crate) operation: String,
    pub(crate) fields: Vec<(String, String)>,
}

impl DiscoveryRecord {
    fn new(operation: &str, fields: impl IntoIterator<Item = (&'static str, String)>) -> Self {
        Self {
            operation: operation.to_owned(),
            fields: fields
                .into_iter()
                .map(|(name, value)| (name.to_owned(), value))
                .collect(),
        }
    }

    pub(crate) fn rendered_bytes(&self) -> Result<usize, Diagnostic> {
        self.render().map(|record| record.len())
    }

    pub(crate) fn render(&self) -> Result<String, Diagnostic> {
        let borrowed = self
            .fields
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect::<Vec<_>>();
        render_record(&self.operation, &borrowed)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct BuiltinOwnerSelector {
    pub(crate) kind: Option<OwnerKind>,
    pub(crate) name: Option<String>,
    pub(crate) parent: Option<OwnerKey>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BuiltinOwnerPage {
    pub(crate) selector_digest: String,
    pub(crate) records: Vec<DiscoveryRecord>,
    pub(crate) matched: usize,
    pub(crate) returned: usize,
    pub(crate) truncated: bool,
    pub(crate) continuation: Option<String>,
    pub(crate) rendered_owner_bytes: usize,
}

pub(crate) fn parse_interface_owner_kind(value: &str) -> Result<OwnerKind, Diagnostic> {
    let kind = OwnerKind::parse(value)?;
    if interface_owner_kinds().contains(&kind) {
        Ok(kind)
    } else {
        Err(discovery_error(
            DiagnosticClass::Source,
            "builtin_owner_kind",
            format!("owner kind '{value}' is not present in a public package interface"),
        ))
    }
}

pub(crate) const fn interface_owner_kinds() -> [OwnerKind; 14] {
    [
        OwnerKind::Record,
        OwnerKind::Variant,
        OwnerKind::Interface,
        OwnerKind::External,
        OwnerKind::PureFunction,
        OwnerKind::TaskFunction,
        OwnerKind::Constant,
        OwnerKind::Component,
        OwnerKind::TypeParameter,
        OwnerKind::Field,
        OwnerKind::Case,
        OwnerKind::Operation,
        OwnerKind::Parameter,
        OwnerKind::Requirement,
    ]
}

pub(crate) fn query_builtin_owners(
    standard: &BuiltinStandard,
    selector: &BuiltinOwnerSelector,
    limit: usize,
    output_bytes: usize,
    continuation: Option<&str>,
) -> Result<BuiltinOwnerPage, Diagnostic> {
    if limit == 0 || limit > BUILTIN_QUERY_MAXIMUM_ITEMS {
        return Err(discovery_error(
            DiagnosticClass::Resource,
            "builtin_query_limit",
            format!("built-in owner limit must be 1 through {BUILTIN_QUERY_MAXIMUM_ITEMS} items"),
        ));
    }
    if !(BUILTIN_QUERY_MINIMUM_BYTES..=BUILTIN_QUERY_MAXIMUM_BYTES).contains(&output_bytes) {
        return Err(discovery_error(
            DiagnosticClass::Resource,
            "builtin_query_bytes",
            format!(
                "built-in owner output budget must be {BUILTIN_QUERY_MINIMUM_BYTES} through {BUILTIN_QUERY_MAXIMUM_BYTES} bytes"
            ),
        ));
    }
    let selector_bytes = selector_bytes(selector)?;
    let selector_digest = selector_digest(&selector_bytes);
    let resume = continuation
        .map(|token| decode_continuation(standard, &selector_digest, token))
        .transpose()?;
    let matching = standard
        .interface_owners
        .iter()
        .filter(|(owner, _value)| resume.is_none_or(|resume| **owner > resume))
        .filter(|(_, value)| selector_matches(selector, &value.record))
        .collect::<Vec<_>>();
    let matched = matching.len();
    let mut records = Vec::new();
    let mut rendered_owner_bytes = 0_usize;
    let mut last = None;
    for (owner, value) in matching {
        if records.len() == limit {
            break;
        }
        let record = owner_summary(standard.package, &value.record)?;
        let record_bytes = record.rendered_bytes()?;
        if rendered_owner_bytes
            .checked_add(record_bytes)
            .is_none_or(|needed| needed > output_bytes)
        {
            break;
        }
        rendered_owner_bytes += record_bytes;
        records.push(record);
        last = Some(*owner);
    }
    let returned = records.len();
    let truncated = returned < matched;
    if truncated && returned == 0 {
        return Err(discovery_error(
            DiagnosticClass::Resource,
            "builtin_query_record_bytes",
            "built-in owner output budget cannot admit the next complete owner record",
        ));
    }
    let continuation = if truncated {
        last.map(|owner| encode_continuation(standard, &selector_digest, owner))
            .transpose()?
    } else {
        None
    };
    Ok(BuiltinOwnerPage {
        selector_digest: hex_digest(&selector_digest),
        records,
        matched,
        returned,
        truncated,
        continuation,
        rendered_owner_bytes,
    })
}

pub(crate) fn inspect_builtin_owner(
    standard: &BuiltinStandard,
    kind: OwnerKind,
    owner: OwnerKey,
) -> Result<Vec<DiscoveryRecord>, Diagnostic> {
    if !kind.accepts_owner(owner) {
        let observed = OwnerKind::ALL
            .into_iter()
            .find(|candidate| candidate.accepts_owner(owner))
            .map(OwnerKind::name)
            .unwrap_or("unknown");
        return Err(discovery_error(
            DiagnosticClass::Source,
            "builtin_owner_identity_kind",
            format!(
                "owner identity '{owner}' belongs to kind '{}', not '{}'",
                observed,
                kind.name()
            ),
        ));
    }
    let value = standard.interface_owners.get(&owner).ok_or_else(|| {
        discovery_error(
            DiagnosticClass::Source,
            "builtin_owner_missing",
            format!(
                "built-in package interface has no exact {} '{owner}'",
                kind.name()
            ),
        )
    })?;
    let mut records = vec![owner_summary(standard.package, &value.record)?];
    append_owner_detail(standard, &value.record, &mut records)?;
    Ok(records)
}

fn owner_summary(
    package: PackageId,
    record: &PackageInterfaceRecord,
) -> Result<DiscoveryRecord, Diagnostic> {
    let owner = record.header().owner;
    let parent = owner_parent(record)
        .map(|owner| owner.to_string())
        .unwrap_or_else(|| "package".to_owned());
    Ok(DiscoveryRecord::new(
        "owner",
        [
            ("kind", record.header().kind.name().to_owned()),
            ("id", owner.to_string()),
            ("name", owner_name(record).to_owned()),
            ("parent", parent),
            ("reference", exact_owner_reference(package, owner)),
        ],
    ))
}

fn append_owner_detail(
    standard: &BuiltinStandard,
    record: &PackageInterfaceRecord,
    records: &mut Vec<DiscoveryRecord>,
) -> Result<(), Diagnostic> {
    match record {
        PackageInterfaceRecord::Declaration(declaration) => match &declaration.payload {
            PackageInterfaceDeclarationPayload::Record { fields } => {
                records.push(declaration_detail(
                    declaration,
                    "record",
                    fields.len(),
                    None,
                ));
                for (index, field) in fields.iter().enumerate() {
                    append_child_owner(standard, OwnerKey::Field(*field), index, records)?;
                }
            }
            PackageInterfaceDeclarationPayload::Variant { cases } => {
                records.push(declaration_detail(
                    declaration,
                    "variant",
                    cases.len(),
                    None,
                ));
                for (index, case) in cases.iter().enumerate() {
                    append_child_owner(standard, OwnerKey::Case(*case), index, records)?;
                }
            }
            PackageInterfaceDeclarationPayload::Interface { operations } => {
                records.push(declaration_detail(
                    declaration,
                    "interface",
                    operations.len(),
                    None,
                ));
                for (index, operation) in operations.iter().enumerate() {
                    append_child_owner(standard, OwnerKey::Operation(*operation), index, records)?;
                }
            }
            PackageInterfaceDeclarationPayload::External(signature) => {
                records.push(declaration_detail(
                    declaration,
                    "external",
                    signature.parameters.len(),
                    Some("pure"),
                ));
                append_type_parameters(standard, &signature.type_parameters, records)?;
                append_parameters(standard, &signature.parameters, records)?;
                append_type(standard, "result", signature.result, records, 0)?;
            }
            PackageInterfaceDeclarationPayload::Function(signature) => {
                let effect = match signature.effect {
                    FunctionEffect::Pure => "pure",
                    FunctionEffect::Task { .. } => "task",
                };
                records.push(declaration_detail(
                    declaration,
                    declaration.header.kind.name(),
                    signature.parameters.len(),
                    Some(effect),
                ));
                if let FunctionEffect::Task { requirements } = &signature.effect {
                    for (index, requirement) in requirements.iter().enumerate() {
                        records.push(DiscoveryRecord::new(
                            "effect.requirement",
                            [
                                ("index", index.to_string()),
                                (
                                    "reference",
                                    format!("{}/{}", requirement.package, requirement.requirement),
                                ),
                            ],
                        ));
                    }
                }
                append_type_parameters(standard, &signature.type_parameters, records)?;
                append_parameters(standard, &signature.parameters, records)?;
                append_type(standard, "result", signature.result, records, 0)?;
            }
            PackageInterfaceDeclarationPayload::Constant { ty } => {
                records.push(declaration_detail(declaration, "constant", 0, Some("pure")));
                append_type(standard, "value", *ty, records, 0)?;
            }
            PackageInterfaceDeclarationPayload::Component {
                requirements,
                ports,
            } => {
                records.push(DiscoveryRecord::new(
                    "declaration",
                    [
                        ("kind", "component".to_owned()),
                        ("requirements", requirements.len().to_string()),
                        ("ports", ports.len().to_string()),
                    ],
                ));
                for (index, requirement) in requirements.iter().enumerate() {
                    append_child_owner(
                        standard,
                        OwnerKey::Requirement(*requirement),
                        index,
                        records,
                    )?;
                }
                for (index, port) in ports.iter().enumerate() {
                    append_child_owner(standard, OwnerKey::Port(*port), index, records)?;
                }
            }
        },
        PackageInterfaceRecord::TypeParameter(_)
        | PackageInterfaceRecord::Field(_)
        | PackageInterfaceRecord::Case(_)
        | PackageInterfaceRecord::Operation(_)
        | PackageInterfaceRecord::Parameter(_)
        | PackageInterfaceRecord::Requirement(_)
        | PackageInterfaceRecord::Port(_) => append_child_detail(standard, record, None, records)?,
    }
    Ok(())
}

fn declaration_detail(
    declaration: &super::kernel::PackageInterfaceDeclaration,
    kind: &str,
    children: usize,
    effect: Option<&str>,
) -> DiscoveryRecord {
    let mut fields = vec![
        ("kind".to_owned(), kind.to_owned()),
        ("name".to_owned(), declaration.name.as_str().to_owned()),
        ("children".to_owned(), children.to_string()),
    ];
    if let Some(effect) = effect {
        fields.push(("effect".to_owned(), effect.to_owned()));
    }
    DiscoveryRecord {
        operation: "declaration".to_owned(),
        fields,
    }
}

fn append_child_owner(
    standard: &BuiltinStandard,
    owner: OwnerKey,
    index: usize,
    records: &mut Vec<DiscoveryRecord>,
) -> Result<(), Diagnostic> {
    let value = standard.interface_owners.get(&owner).ok_or_else(|| {
        discovery_error(
            DiagnosticClass::Corrupt,
            "builtin_interface_child",
            "built-in package interface declaration names a missing public child owner",
        )
    })?;
    records.push(owner_summary(standard.package, &value.record)?);
    append_child_detail(standard, &value.record, Some(index), records)
}

fn append_child_detail(
    standard: &BuiltinStandard,
    record: &PackageInterfaceRecord,
    index: Option<usize>,
    records: &mut Vec<DiscoveryRecord>,
) -> Result<(), Diagnostic> {
    let mut common = Vec::new();
    if let Some(index) = index {
        common.push(("index".to_owned(), index.to_string()));
    }
    match record {
        PackageInterfaceRecord::TypeParameter(parameter) => {
            common.push(("name".to_owned(), parameter.name.as_str().to_owned()));
            records.push(DiscoveryRecord {
                operation: "type-parameter".to_owned(),
                fields: common,
            });
        }
        PackageInterfaceRecord::Field(field) => {
            common.push(("name".to_owned(), field.name.as_str().to_owned()));
            common.push(("type-path".to_owned(), format!("field.{}", field.name)));
            records.push(DiscoveryRecord {
                operation: "field".to_owned(),
                fields: common,
            });
            append_type(
                standard,
                &format!("field.{}", field.name),
                field.ty,
                records,
                0,
            )?;
        }
        PackageInterfaceRecord::Case(case) => {
            common.push(("name".to_owned(), case.name.as_str().to_owned()));
            common.push(("payload".to_owned(), case.payload.is_some().to_string()));
            records.push(DiscoveryRecord {
                operation: "case".to_owned(),
                fields: common,
            });
            if let Some(payload) = case.payload {
                append_type(
                    standard,
                    &format!("case.{}", case.name),
                    payload,
                    records,
                    0,
                )?;
            }
        }
        PackageInterfaceRecord::Operation(operation) => {
            common.extend([
                ("name".to_owned(), operation.name.as_str().to_owned()),
                (
                    "reference".to_owned(),
                    format!("{}/{}", standard.package, operation.header.owner),
                ),
                (
                    "idempotency".to_owned(),
                    idempotency_name(operation.idempotency).to_owned(),
                ),
                (
                    "external-visibility".to_owned(),
                    external_visibility_name(operation.external_visibility).to_owned(),
                ),
                (
                    "parameters".to_owned(),
                    operation.parameters.len().to_string(),
                ),
                ("result-path".to_owned(), "result".to_owned()),
            ]);
            records.push(DiscoveryRecord {
                operation: "operation".to_owned(),
                fields: common,
            });
            append_parameters(standard, &operation.parameters, records)?;
            append_type(standard, "result", operation.result, records, 0)?;
        }
        PackageInterfaceRecord::Parameter(parameter) => {
            common.extend([
                ("name".to_owned(), parameter.name.as_str().to_owned()),
                (
                    "use".to_owned(),
                    parameter_use_name(parameter.use_mode).to_owned(),
                ),
                (
                    "type-path".to_owned(),
                    format!("parameter.{}", parameter.name),
                ),
            ]);
            records.push(DiscoveryRecord {
                operation: "parameter".to_owned(),
                fields: common,
            });
            append_type(
                standard,
                &format!("parameter.{}", parameter.name),
                parameter.ty,
                records,
                0,
            )?;
        }
        PackageInterfaceRecord::Requirement(requirement) => {
            common.extend([
                ("name".to_owned(), requirement.name.as_str().to_owned()),
                (
                    "interface".to_owned(),
                    format!(
                        "{}/{}",
                        requirement.interface.package, requirement.interface.declaration
                    ),
                ),
                (
                    "operations".to_owned(),
                    requirement.operations.len().to_string(),
                ),
                ("limits".to_owned(), requirement.limits.len().to_string()),
            ]);
            records.push(DiscoveryRecord {
                operation: "requirement".to_owned(),
                fields: common,
            });
            for (index, operation) in requirement.operations.iter().enumerate() {
                records.push(DiscoveryRecord::new(
                    "requirement.operation",
                    [
                        ("index", index.to_string()),
                        (
                            "reference",
                            format!("{}/{}", operation.package, operation.operation),
                        ),
                    ],
                ));
            }
            for (index, limit) in requirement.limits.iter().enumerate() {
                records.push(DiscoveryRecord::new(
                    "requirement.limit",
                    [
                        ("index", index.to_string()),
                        ("name", limit.name.as_str().to_owned()),
                        ("maximum", limit.maximum.to_string()),
                        ("unit", format!("{:?}", limit.unit).to_ascii_lowercase()),
                    ],
                ));
            }
        }
        PackageInterfaceRecord::Port(port) => {
            common.extend([
                ("name".to_owned(), port.name.as_str().to_owned()),
                ("type-path".to_owned(), "function".to_owned()),
            ]);
            records.push(DiscoveryRecord {
                operation: "port".to_owned(),
                fields: common,
            });
            append_type(standard, "function", port.function_type, records, 0)?;
        }
        PackageInterfaceRecord::Declaration(_) => {
            return Err(discovery_error(
                DiagnosticClass::Corrupt,
                "builtin_interface_child_kind",
                "declaration reached the child-owner detail path",
            ));
        }
    }
    Ok(())
}

fn append_type_parameters(
    standard: &BuiltinStandard,
    parameters: &[super::semantic_id::TypeParameterId],
    records: &mut Vec<DiscoveryRecord>,
) -> Result<(), Diagnostic> {
    for (index, parameter) in parameters.iter().enumerate() {
        append_child_owner(
            standard,
            OwnerKey::TypeParameter(*parameter),
            index,
            records,
        )?;
    }
    Ok(())
}

fn append_parameters(
    standard: &BuiltinStandard,
    parameters: &[super::semantic_id::ParameterId],
    records: &mut Vec<DiscoveryRecord>,
) -> Result<(), Diagnostic> {
    for (index, parameter) in parameters.iter().enumerate() {
        append_child_owner(standard, OwnerKey::Parameter(*parameter), index, records)?;
    }
    Ok(())
}

fn append_type(
    standard: &BuiltinStandard,
    path: &str,
    digest: TypeObjectDigest,
    records: &mut Vec<DiscoveryRecord>,
    depth: usize,
) -> Result<(), Diagnostic> {
    if depth > 128 || records.len() >= 10_000 {
        return Err(discovery_error(
            DiagnosticClass::Resource,
            "builtin_type_projection_limit",
            "built-in type projection exceeded its depth or record bound",
        ));
    }
    let ty = standard.interface_types.get(&digest).ok_or_else(|| {
        discovery_error(
            DiagnosticClass::Corrupt,
            "builtin_interface_type",
            "built-in package interface names a missing exact type object",
        )
    })?;
    let mut fields = vec![
        ("path".to_owned(), path.to_owned()),
        ("digest".to_owned(), digest.to_string()),
        ("form".to_owned(), type_form_name(&ty.form).to_owned()),
    ];
    match &ty.form {
        TypeForm::TypeParameter { parameter } => {
            fields.push(("parameter".to_owned(), parameter.to_string()));
        }
        TypeForm::Named { declaration } => {
            fields.push((
                "reference".to_owned(),
                format!("{}/{}", declaration.package, declaration.declaration),
            ));
        }
        TypeForm::CapabilityResource { interface } => {
            fields.push((
                "interface".to_owned(),
                format!("{}/{}", interface.package, interface.declaration),
            ));
        }
        TypeForm::StructuralRecord { fields: structural } => {
            fields.push(("fields".to_owned(), structural.len().to_string()));
        }
        TypeForm::Function { parameters, .. } => {
            fields.push(("parameters".to_owned(), parameters.len().to_string()));
        }
        TypeForm::Unit
        | TypeForm::Bool
        | TypeForm::I64
        | TypeForm::Bytes
        | TypeForm::Text
        | TypeForm::StaticText
        | TypeForm::Secret
        | TypeForm::List { .. }
        | TypeForm::Map { .. }
        | TypeForm::Option { .. }
        | TypeForm::Result { .. }
        | TypeForm::Stream { .. } => {}
    }
    records.push(DiscoveryRecord {
        operation: "type".to_owned(),
        fields,
    });
    match &ty.form {
        TypeForm::StructuralRecord { fields } => {
            for (index, field) in fields.iter().enumerate() {
                let child = format!("{path}.{}", field.name);
                records.push(DiscoveryRecord::new(
                    "type.field",
                    [
                        ("parent", path.to_owned()),
                        ("index", index.to_string()),
                        ("name", field.name.as_str().to_owned()),
                        ("type-path", child.clone()),
                    ],
                ));
                append_type(standard, &child, field.ty, records, depth + 1)?;
            }
        }
        TypeForm::List { item } | TypeForm::Option { item } | TypeForm::Stream { item } => {
            append_type(standard, &format!("{path}.item"), *item, records, depth + 1)?;
        }
        TypeForm::Map { key, value } => {
            append_type(standard, &format!("{path}.key"), *key, records, depth + 1)?;
            append_type(
                standard,
                &format!("{path}.value"),
                *value,
                records,
                depth + 1,
            )?;
        }
        TypeForm::Result { ok, error } => {
            append_type(standard, &format!("{path}.ok"), *ok, records, depth + 1)?;
            append_type(
                standard,
                &format!("{path}.error"),
                *error,
                records,
                depth + 1,
            )?;
        }
        TypeForm::Function { parameters, result } => {
            for (index, parameter) in parameters.iter().enumerate() {
                append_type(
                    standard,
                    &format!("{path}.parameter.{index}"),
                    *parameter,
                    records,
                    depth + 1,
                )?;
            }
            append_type(
                standard,
                &format!("{path}.result"),
                *result,
                records,
                depth + 1,
            )?;
        }
        TypeForm::Unit
        | TypeForm::Bool
        | TypeForm::I64
        | TypeForm::Bytes
        | TypeForm::Text
        | TypeForm::StaticText
        | TypeForm::Secret
        | TypeForm::TypeParameter { .. }
        | TypeForm::Named { .. }
        | TypeForm::CapabilityResource { .. } => {}
    }
    Ok(())
}

fn selector_matches(selector: &BuiltinOwnerSelector, record: &PackageInterfaceRecord) -> bool {
    selector
        .kind
        .is_none_or(|kind| record.header().kind == kind)
        && selector
            .name
            .as_deref()
            .is_none_or(|name| owner_name(record) == name)
        && selector
            .parent
            .is_none_or(|parent| owner_parent(record) == Some(parent))
}

fn owner_name(record: &PackageInterfaceRecord) -> &str {
    match record {
        PackageInterfaceRecord::Declaration(record) => record.name.as_str(),
        PackageInterfaceRecord::TypeParameter(record) => record.name.as_str(),
        PackageInterfaceRecord::Field(record) => record.name.as_str(),
        PackageInterfaceRecord::Case(record) => record.name.as_str(),
        PackageInterfaceRecord::Operation(record) => record.name.as_str(),
        PackageInterfaceRecord::Parameter(record) => record.name.as_str(),
        PackageInterfaceRecord::Requirement(record) => record.name.as_str(),
        PackageInterfaceRecord::Port(record) => record.name.as_str(),
    }
}

fn owner_parent(record: &PackageInterfaceRecord) -> Option<OwnerKey> {
    match record {
        PackageInterfaceRecord::Declaration(_) => None,
        PackageInterfaceRecord::TypeParameter(record) => {
            Some(OwnerKey::Declaration(record.declaration))
        }
        PackageInterfaceRecord::Field(record) => Some(OwnerKey::Declaration(record.declaration)),
        PackageInterfaceRecord::Case(record) => Some(OwnerKey::Declaration(record.declaration)),
        PackageInterfaceRecord::Operation(record) => {
            Some(OwnerKey::Declaration(record.declaration))
        }
        PackageInterfaceRecord::Parameter(record) => Some(match record.parent {
            ParameterParent::Function(declaration) => OwnerKey::Declaration(declaration),
            ParameterParent::Operation(operation) => OwnerKey::Operation(operation),
        }),
        PackageInterfaceRecord::Requirement(record) => {
            Some(OwnerKey::Declaration(record.declaration))
        }
        PackageInterfaceRecord::Port(record) => Some(OwnerKey::Declaration(record.declaration)),
    }
}

fn exact_owner_reference(package: PackageId, owner: OwnerKey) -> String {
    format!("{package}/{owner}")
}

fn type_form_name(form: &TypeForm) -> &'static str {
    match form {
        TypeForm::Unit => "unit",
        TypeForm::Bool => "bool",
        TypeForm::I64 => "i64",
        TypeForm::Bytes => "bytes",
        TypeForm::Text => "text",
        TypeForm::StaticText => "static-text",
        TypeForm::Secret => "secret",
        TypeForm::TypeParameter { .. } => "parameter",
        TypeForm::Named { .. } => "named",
        TypeForm::CapabilityResource { .. } => "capability-resource",
        TypeForm::StructuralRecord { .. } => "structural-record",
        TypeForm::List { .. } => "list",
        TypeForm::Map { .. } => "map",
        TypeForm::Option { .. } => "option",
        TypeForm::Result { .. } => "result",
        TypeForm::Stream { .. } => "stream",
        TypeForm::Function { .. } => "function",
    }
}

fn parameter_use_name(value: crate::platform::kernel::ParameterUse) -> &'static str {
    match value {
        crate::platform::kernel::ParameterUse::Unrestricted => "unrestricted",
        crate::platform::kernel::ParameterUse::Borrow => "borrow",
        crate::platform::kernel::ParameterUse::Consume => "consume",
    }
}

fn idempotency_name(value: Idempotency) -> &'static str {
    match value {
        Idempotency::Idempotent => "idempotent",
        Idempotency::IdempotentWithKey => "idempotent-with-key",
        Idempotency::NonIdempotent => "non-idempotent",
    }
}

fn external_visibility_name(value: ExternalVisibility) -> &'static str {
    match value {
        ExternalVisibility::None => "none",
        ExternalVisibility::Possible => "possible",
    }
}

fn selector_bytes(selector: &BuiltinOwnerSelector) -> Result<Vec<u8>, Diagnostic> {
    let mut bytes = vec![1_u8];
    bytes.push(selector.kind.map_or(0, |kind| kind.tag()));
    match selector.name.as_deref() {
        None => bytes.push(0),
        Some(name) => {
            bytes.push(1);
            let length = u16::try_from(name.len()).map_err(|_| {
                discovery_error(
                    DiagnosticClass::Resource,
                    "builtin_selector_name",
                    "built-in owner name selector exceeds its encoded bound",
                )
            })?;
            bytes.extend_from_slice(&length.to_be_bytes());
            bytes.extend_from_slice(name.as_bytes());
        }
    }
    match selector.parent {
        None => bytes.push(0),
        Some(parent) => {
            bytes.push(1);
            bytes.extend_from_slice(&EncodedOwnerKey::new(parent).bytes());
        }
    }
    Ok(bytes)
}

fn selector_digest(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(BUILTIN_SELECTOR_DOMAIN);
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
    *hasher.finalize().as_bytes()
}

fn encode_continuation(
    standard: &BuiltinStandard,
    selector: &[u8; 32],
    resume: OwnerKey,
) -> Result<String, Diagnostic> {
    let mut raw = Vec::with_capacity(BUILTIN_CONTINUATION_RAW_BYTES);
    raw.extend_from_slice(&BUILTIN_CONTINUATION_MAGIC);
    raw.extend_from_slice(&BUILTIN_CONTINUATION_VERSION.to_be_bytes());
    raw.extend_from_slice(&standard.package.bytes());
    raw.extend_from_slice(&standard.package_revision.bytes());
    raw.extend_from_slice(selector);
    raw.extend_from_slice(&EncodedOwnerKey::new(resume).bytes());
    let mut hasher = blake3::Hasher::new_derive_key(BUILTIN_CONTINUATION_DOMAIN);
    hasher.update(&(raw.len() as u64).to_be_bytes());
    hasher.update(&raw);
    raw.extend_from_slice(hasher.finalize().as_bytes());
    let token = format!(
        "{BUILTIN_CONTINUATION_PREFIX}{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw)
    );
    if token.len() > BUILTIN_CONTINUATION_MAXIMUM_BYTES {
        return Err(discovery_error(
            DiagnosticClass::Infrastructure,
            "builtin_continuation_bound",
            "built-in continuation encoder exceeded its public bound",
        ));
    }
    Ok(token)
}

fn decode_continuation(
    standard: &BuiltinStandard,
    selector: &[u8; 32],
    token: &str,
) -> Result<OwnerKey, Diagnostic> {
    if token.len() > BUILTIN_CONTINUATION_MAXIMUM_BYTES {
        return Err(discovery_error(
            DiagnosticClass::Resource,
            "builtin_continuation_bytes",
            format!(
                "built-in continuation exceeds {BUILTIN_CONTINUATION_MAXIMUM_BYTES} encoded bytes"
            ),
        ));
    }
    let encoded = token
        .strip_prefix(BUILTIN_CONTINUATION_PREFIX)
        .ok_or_else(|| {
            discovery_error(
                DiagnosticClass::Source,
                "builtin_continuation_prefix",
                "built-in continuation has a foreign prefix",
            )
        })?;
    let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|_| {
            discovery_error(
                DiagnosticClass::Source,
                "builtin_continuation_encoding",
                "built-in continuation is not canonical base64url",
            )
        })?;
    if base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&raw) != encoded {
        return Err(discovery_error(
            DiagnosticClass::Source,
            "builtin_continuation_canonical",
            "built-in continuation is not canonically encoded",
        ));
    }
    if raw.len() != BUILTIN_CONTINUATION_RAW_BYTES {
        return Err(discovery_error(
            DiagnosticClass::Source,
            "builtin_continuation_length",
            "built-in continuation has a foreign decoded length",
        ));
    }
    let payload_bytes = raw.len() - 32;
    let mut hasher = blake3::Hasher::new_derive_key(BUILTIN_CONTINUATION_DOMAIN);
    hasher.update(&(payload_bytes as u64).to_be_bytes());
    hasher.update(&raw[..payload_bytes]);
    if hasher.finalize().as_bytes() != &raw[payload_bytes..] {
        return Err(discovery_error(
            DiagnosticClass::Source,
            "builtin_continuation_integrity",
            "built-in continuation integrity check failed",
        ));
    }
    if raw[..8] != BUILTIN_CONTINUATION_MAGIC
        || u16::from_be_bytes([raw[8], raw[9]]) != BUILTIN_CONTINUATION_VERSION
    {
        return Err(discovery_error(
            DiagnosticClass::Source,
            "builtin_continuation_contract",
            "built-in continuation uses a predecessor or foreign contract",
        ));
    }
    let package_bytes: [u8; 16] = raw[10..26].try_into().map_err(|_| {
        discovery_error(
            DiagnosticClass::Source,
            "builtin_continuation_length",
            "built-in continuation package identity is truncated",
        )
    })?;
    let package = PackageId::from_bytes(package_bytes).ok_or_else(|| {
        discovery_error(
            DiagnosticClass::Source,
            "builtin_continuation_foreign",
            "built-in continuation contains the reserved package identity",
        )
    })?;
    if package != standard.package {
        return Err(discovery_error(
            DiagnosticClass::Source,
            "builtin_continuation_foreign",
            "built-in continuation belongs to a foreign package",
        ));
    }
    if raw[26..58] != standard.package_revision.bytes() {
        return Err(discovery_error(
            DiagnosticClass::Source,
            "builtin_continuation_stale",
            "built-in continuation belongs to a different package revision",
        ));
    }
    if raw[58..90] != selector[..] {
        return Err(discovery_error(
            DiagnosticClass::Source,
            "builtin_continuation_selector",
            "built-in continuation selector does not match the normalized query",
        ));
    }
    EncodedOwnerKey::decode(&raw[90..107]).map_err(|error| {
        discovery_error(
            DiagnosticClass::Source,
            "builtin_continuation_resume",
            format!(
                "built-in continuation resume key is invalid: {}",
                error.message
            ),
        )
    })
}

fn hex_digest(bytes: &[u8; 32]) -> String {
    super::semantic_id::encode_hex(bytes)
}

fn discovery_error(
    class: DiagnosticClass,
    code: &'static str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn owner_pages_are_deterministic_revision_bound_and_selector_bound() {
        let standard = BuiltinStandard::load().expect("built-in standard");
        let selector = BuiltinOwnerSelector::default();
        let first = query_builtin_owners(standard, &selector, 1, 64 * 1024, None)
            .expect("first owner page");
        assert_eq!(first.returned, 1);
        assert!(first.truncated);
        let token = first.continuation.as_deref().expect("continuation");
        let second = query_builtin_owners(standard, &selector, 1, 64 * 1024, Some(token))
            .expect("second owner page");
        assert_ne!(first.records, second.records);
        assert_eq!(
            first,
            query_builtin_owners(standard, &selector, 1, 64 * 1024, None)
                .expect("repeated first page")
        );

        let filtered = BuiltinOwnerSelector {
            kind: Some(OwnerKind::External),
            ..BuiltinOwnerSelector::default()
        };
        assert_eq!(
            query_builtin_owners(standard, &filtered, 1, 64 * 1024, Some(token))
                .expect_err("selector mismatch")
                .code,
            "builtin_continuation_selector"
        );
        let mut malformed = token.as_bytes().to_vec();
        let last = malformed.len() - 1;
        malformed[last] = if malformed[last] == b'A' { b'B' } else { b'A' };
        let malformed = String::from_utf8(malformed).expect("ASCII token");
        assert!(matches!(
            query_builtin_owners(standard, &selector, 1, 64 * 1024, Some(&malformed))
                .expect_err("corrupt token")
                .code
                .as_str(),
            "builtin_continuation_integrity" | "builtin_continuation_encoding"
        ));
    }

    #[test]
    fn every_interface_owner_has_bounded_exact_detail() {
        let standard = BuiltinStandard::load().expect("built-in standard");
        let mut observed = BTreeSet::new();
        for (owner, value) in &standard.interface_owners {
            let detail = inspect_builtin_owner(standard, value.kind(), *owner)
                .expect("exact interface owner detail");
            assert!(!detail.is_empty());
            assert!(detail.len() < 10_000);
            assert!(observed.insert(*owner));
        }
        assert_eq!(observed.len(), standard.interface_owners.len());
    }

    #[test]
    fn continuations_reject_foreign_stale_and_oversized_authority() {
        let standard = BuiltinStandard::load().expect("built-in standard");
        let selector = BuiltinOwnerSelector::default();
        let owner = *standard
            .interface_owners
            .keys()
            .next()
            .expect("standard owner");

        let mut stale = standard.clone();
        stale.package_revision =
            "package_revision_0101010101010101010101010101010101010101010101010101010101010101"
                .parse()
                .expect("foreign nonzero package revision");
        let token = encode_continuation(
            &stale,
            &selector_digest(&selector_bytes(&selector).expect("selector bytes")),
            owner,
        )
        .expect("stale token");
        assert_eq!(
            query_builtin_owners(standard, &selector, 1, 64 * 1024, Some(&token))
                .expect_err("stale continuation")
                .code,
            "builtin_continuation_stale"
        );

        let mut foreign = standard.clone();
        foreign.package = "pkg_20000000000000000000000000000001"
            .parse()
            .expect("foreign package");
        let token = encode_continuation(
            &foreign,
            &selector_digest(&selector_bytes(&selector).expect("selector bytes")),
            owner,
        )
        .expect("foreign token");
        assert_eq!(
            query_builtin_owners(standard, &selector, 1, 64 * 1024, Some(&token))
                .expect_err("foreign continuation")
                .code,
            "builtin_continuation_foreign"
        );

        let oversized = "x".repeat(BUILTIN_CONTINUATION_MAXIMUM_BYTES + 1);
        assert_eq!(
            query_builtin_owners(standard, &selector, 1, 64 * 1024, Some(&oversized))
                .expect_err("oversized continuation")
                .code,
            "builtin_continuation_bytes"
        );
    }
}
