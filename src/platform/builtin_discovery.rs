//! Bounded, implementation-free discovery over the exact embedded standard package interface.

use super::builtin_standard::BuiltinStandard;
use super::control::render_record;
use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::kernel::{
    AssociatedTypeBindingValue, AssociatedTypeBound, CallableResult, DeclarationReference,
    EncodedOwnerKey, ExternalVisibility, FunctionEffect, Idempotency, OwnerKey, OwnerKind,
    PackageId, PackageInterfaceDeclarationPayload, PackageInterfaceRecord, ParameterParent,
    ReadScopeArgument, ReadScopeBinder, ReadScopeParameter, ReadScopeReference, ReadScopeSource,
    ReadViewFieldKind, ResultRelation, TypeForm, TypeObjectDigest,
};
use base64::Engine;

pub(crate) struct PackageInterfaceView<'a> {
    pub package: PackageId,
    pub package_revision: super::kernel::PackageRevisionDigest,
    pub interface_owners:
        &'a std::collections::BTreeMap<OwnerKey, super::package_interface::PackageInterfaceOwner>,
    pub interface_types:
        &'a std::collections::BTreeMap<TypeObjectDigest, super::kernel::TypeObject>,
}

impl<'a> From<&'a BuiltinStandard> for PackageInterfaceView<'a> {
    fn from(standard: &'a BuiltinStandard) -> Self {
        Self {
            package: standard.package,
            package_revision: standard.package_revision,
            interface_owners: &standard.interface_owners,
            interface_types: &standard.interface_types,
        }
    }
}

pub(crate) fn query_builtin_owners(
    standard: &BuiltinStandard,
    selector: &BuiltinOwnerSelector,
    limit: usize,
    output_bytes: usize,
    continuation: Option<&str>,
) -> Result<BuiltinOwnerPage, Diagnostic> {
    query_interface_owners(
        &standard.into(),
        selector,
        limit,
        output_bytes,
        continuation,
    )
}

pub(crate) fn inspect_builtin_owner(
    standard: &BuiltinStandard,
    kind: OwnerKind,
    owner: OwnerKey,
) -> Result<Vec<DiscoveryRecord>, Diagnostic> {
    inspect_interface_owner(&standard.into(), kind, owner)
}

pub(crate) const BUILTIN_QUERY_DEFAULT_ITEMS: usize = 50;
pub(crate) const BUILTIN_QUERY_MAXIMUM_ITEMS: usize = 10_000;
pub(crate) const BUILTIN_QUERY_MINIMUM_BYTES: usize = 1_536;
pub(crate) const BUILTIN_QUERY_DEFAULT_BYTES: usize = 64 * 1_024;
pub(crate) const BUILTIN_QUERY_MAXIMUM_BYTES: usize = 4 * 1_048_576;
pub(crate) const BUILTIN_CONTINUATION_MAXIMUM_BYTES: usize = 320;
pub(crate) const BUILTIN_QUERY_ORDERING: &str = "package-interface-owner-key-v1";
pub(crate) const BUILTIN_CONTINUATION_PREFIX: &str = "bcont_";
const BUILTIN_CONTINUATION_MAGIC: [u8; 8] = *b"LKJBCT01";
const BUILTIN_CONTINUATION_VERSION: u16 = crate::platform::canonical_schema::SCHEMA_EPOCH;
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

pub(crate) const fn interface_owner_kinds() -> [OwnerKind; 18] {
    [
        OwnerKind::OwnedContract,
        OwnerKind::OwnedImplementation,
        OwnerKind::Record,
        OwnerKind::OwnedRecord,
        OwnerKind::ReadView,
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

pub(crate) fn query_interface_owners(
    standard: &PackageInterfaceView<'_>,
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
        .filter(|(_, value)| public_interface_owner(standard, &value.record))
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

pub(crate) fn inspect_interface_owner(
    standard: &PackageInterfaceView<'_>,
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
    let value = standard
        .interface_owners
        .get(&owner)
        .filter(|value| {
            value.record.header().kind == kind && public_interface_owner(standard, &value.record)
        })
        .ok_or_else(|| {
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
    let mut summary = DiscoveryRecord::new(
        "owner",
        [
            ("kind", record.header().kind.name().to_owned()),
            ("id", owner.to_string()),
            ("name", owner_name(record).to_owned()),
            ("parent", parent),
            ("reference", exact_owner_reference(package, owner)),
        ],
    );
    if let PackageInterfaceRecord::TypeParameter(parameter) = record {
        summary.fields.push((
            "constraint".to_owned(),
            parameter.constraints.name().to_owned(),
        ));
    }
    Ok(summary)
}

fn append_record(
    records: &mut Vec<DiscoveryRecord>,
    record: DiscoveryRecord,
) -> Result<(), Diagnostic> {
    if records.len() >= BUILTIN_QUERY_MAXIMUM_ITEMS {
        return Err(discovery_error(
            DiagnosticClass::Resource,
            "builtin_type_projection_limit",
            "package interface projection exceeded its record bound",
        ));
    }
    records.push(record);
    Ok(())
}

fn declaration_id(owner: OwnerKey) -> Result<super::semantic_id::DeclarationId, Diagnostic> {
    if let OwnerKey::Declaration(id) = owner {
        Ok(id)
    } else {
        Err(discovery_error(
            DiagnosticClass::Corrupt,
            "builtin_interface_child_kind",
            "interface declaration has a non-declaration identity",
        ))
    }
}

fn append_scope_parameters(
    parent: &str,
    binder: ReadScopeBinder,
    parameters: &[ReadScopeParameter],
    records: &mut Vec<DiscoveryRecord>,
) -> Result<(), Diagnostic> {
    for (index, parameter) in parameters.iter().enumerate() {
        append_record(
            records,
            DiscoveryRecord::new(
                "read-scope-parameter",
                [
                    ("parent", parent.to_owned()),
                    ("index", index.to_string()),
                    ("id", parameter.id.to_string()),
                    ("name", parameter.name.to_string()),
                    (
                        "scope",
                        super::cli::read_scope_coordinate(ReadScopeArgument::Parameter(
                            ReadScopeReference {
                                binder,
                                parameter: parameter.id,
                            },
                        )),
                    ),
                ],
            ),
        )?;
    }
    Ok(())
}

fn append_scope_sources<S: std::fmt::Display>(
    parent: &str,
    sources: &[ReadScopeSource<S>],
    records: &mut Vec<DiscoveryRecord>,
) -> Result<(), Diagnostic> {
    for (index, source) in sources.iter().enumerate() {
        append_record(
            records,
            DiscoveryRecord::new(
                "read-scope-source",
                [
                    ("parent", parent.to_owned()),
                    ("index", index.to_string()),
                    ("source", source.source.to_string()),
                    ("scope", super::cli::read_scope_coordinate(source.scope)),
                ],
            ),
        )?;
    }
    Ok(())
}

fn append_callable_result<S: std::fmt::Display>(
    standard: &PackageInterfaceView<'_>,
    operation: &str,
    parent: &str,
    path: &str,
    result: &CallableResult<S>,
    records: &mut Vec<DiscoveryRecord>,
) -> Result<(), Diagnostic> {
    let (kind, source, scope) = match &result.relation {
        ResultRelation::Value => ("value", "-".to_owned(), "-".to_owned()),
        ResultRelation::ReadFrom { source } => ("read-from", source.to_string(), "-".to_owned()),
        ResultRelation::ViewFrom { source, scope } => (
            "view-from",
            source.to_string(),
            super::cli::read_scope_coordinate(*scope),
        ),
    };
    append_record(
        records,
        DiscoveryRecord::new(
            operation,
            [
                ("parent", parent.to_owned()),
                ("type-path", path.to_owned()),
                ("result-kind", kind.to_owned()),
                ("result-source", source),
                ("result-scope", scope),
            ],
        ),
    )?;
    append_type(standard, path, result.ty, records, 0)
}

fn append_owner_detail(
    standard: &PackageInterfaceView<'_>,
    record: &PackageInterfaceRecord,
    records: &mut Vec<DiscoveryRecord>,
) -> Result<(), Diagnostic> {
    match record {
        PackageInterfaceRecord::Declaration(declaration) => match &declaration.payload {
            PackageInterfaceDeclarationPayload::OwnedContract(c) => {
                for (index, parameter) in std::iter::once(c.self_parameter)
                    .chain(c.type_parameters.iter().copied())
                    .enumerate()
                {
                    append_child_owner(
                        standard,
                        OwnerKey::TypeParameter(parameter),
                        index,
                        records,
                    )?;
                }
                let mut detail =
                    declaration_detail(declaration, "owned-contract", c.methods.len(), None);
                detail.fields.extend([
                    ("self".to_owned(), c.self_parameter.to_string()),
                    (
                        "associated-types".to_owned(),
                        c.associated_types.len().to_string(),
                    ),
                    (
                        "type-parameters".to_owned(),
                        c.type_parameters.len().to_string(),
                    ),
                ]);
                append_record(records, detail)?;
                let contract = DeclarationReference {
                    package: standard.package,
                    declaration: declaration_id(declaration.header.owner)?,
                };
                for (index, member) in c.associated_types.iter().enumerate() {
                    let mut fields = vec![
                        ("owner".to_owned(), declaration.header.owner.to_string()),
                        ("index".to_owned(), index.to_string()),
                        ("id".to_owned(), member.id.to_string()),
                        ("name".to_owned(), member.name.to_string()),
                        ("arity".to_owned(), member.bound.scope_arity().to_string()),
                    ];
                    match &member.bound {
                        AssociatedTypeBound::Owned => {
                            fields.push(("kind".to_owned(), "owned".to_owned()))
                        }
                        AssociatedTypeBound::ReadViewFamily { scope_parameter } => {
                            fields.extend([
                                ("kind".to_owned(), "read-view-family".to_owned()),
                                ("scope-parameter".to_owned(), scope_parameter.id.to_string()),
                                ("scope-name".to_owned(), scope_parameter.name.to_string()),
                            ]);
                            append_scope_parameters(
                                &format!("{}/{}", declaration.header.owner, member.id),
                                ReadScopeBinder::AssociatedMember {
                                    contract,
                                    member: member.id,
                                },
                                std::slice::from_ref(scope_parameter),
                                records,
                            )?;
                        }
                    }
                    append_record(
                        records,
                        DiscoveryRecord {
                            operation: "owned.associated-type".to_owned(),
                            fields,
                        },
                    )?;
                }
                for (index, method) in c.methods.iter().enumerate() {
                    let parent = format!("{}/{}", declaration.header.owner, method.id);
                    let path = format!("method.{}", method.id);
                    let (effect, effect_parameters, requirements) = match &method.effect {
                        FunctionEffect::Pure => ("pure", &[][..], &[][..]),
                        FunctionEffect::Task {
                            effect_parameters,
                            requirements,
                        } => (
                            "task",
                            effect_parameters.as_slice(),
                            requirements.as_slice(),
                        ),
                    };
                    append_record(
                        records,
                        DiscoveryRecord::new(
                            "owned.method",
                            [
                                ("owner", declaration.header.owner.to_string()),
                                ("index", index.to_string()),
                                ("id", method.id.to_string()),
                                ("name", method.name.to_string()),
                                ("parameters", method.parameters.len().to_string()),
                                ("effect", effect.to_owned()),
                                ("effect-parameters", effect_parameters.len().to_string()),
                                ("requirements", requirements.len().to_string()),
                                (
                                    "read-scope-parameters",
                                    method.read_scope_parameters.len().to_string(),
                                ),
                                (
                                    "read-scope-sources",
                                    method.read_scope_sources.len().to_string(),
                                ),
                                ("result-path", format!("{path}.result")),
                            ],
                        ),
                    )?;
                    for (index, parameter) in effect_parameters.iter().enumerate() {
                        append_record(
                            records,
                            DiscoveryRecord::new(
                                "owned.method-effect-parameter",
                                [
                                    ("parent", parent.clone()),
                                    ("index", index.to_string()),
                                    (
                                        "reference",
                                        format!("{}/{}", parameter.package, parameter.parameter),
                                    ),
                                ],
                            ),
                        )?;
                    }
                    for (index, requirement) in requirements.iter().enumerate() {
                        append_record(
                            records,
                            DiscoveryRecord::new(
                                "owned.method-requirement",
                                [
                                    ("parent", parent.clone()),
                                    ("index", index.to_string()),
                                    (
                                        "reference",
                                        format!(
                                            "{}/{}",
                                            requirement.package(),
                                            requirement.owner()
                                        ),
                                    ),
                                ],
                            ),
                        )?;
                    }
                    append_scope_parameters(
                        &parent,
                        ReadScopeBinder::ContractMethod {
                            contract,
                            method: method.id,
                        },
                        &method.read_scope_parameters,
                        records,
                    )?;
                    append_scope_sources(&parent, &method.read_scope_sources, records)?;
                    for (index, parameter) in method.parameters.iter().enumerate() {
                        let type_path = format!("{path}.parameter.{index}");
                        append_record(
                            records,
                            DiscoveryRecord::new(
                                "owned.method-parameter",
                                [
                                    ("parent", parent.clone()),
                                    ("index", index.to_string()),
                                    ("use", parameter_use_name(parameter.use_mode).to_owned()),
                                    ("type-path", type_path.clone()),
                                ],
                            ),
                        )?;
                        append_type(standard, &type_path, parameter.ty, records, 0)?;
                    }
                    append_callable_result(
                        standard,
                        "owned.method-result",
                        &parent,
                        &format!("{path}.result"),
                        &method.result,
                        records,
                    )?;
                }
            }
            PackageInterfaceDeclarationPayload::OwnedImplementation(i) => {
                for (index, parameter) in i.type_parameters.iter().enumerate() {
                    append_child_owner(
                        standard,
                        OwnerKey::TypeParameter(*parameter),
                        index,
                        records,
                    )?;
                }
                let mut detail =
                    declaration_detail(declaration, "owned-implementation", i.methods.len(), None);
                detail.fields.extend([
                    (
                        "contract".to_owned(),
                        format!("{}/{}", i.contract.package, i.contract.declaration),
                    ),
                    ("self".to_owned(), i.self_type.to_string()),
                    (
                        "associated-types".to_owned(),
                        i.associated_types.len().to_string(),
                    ),
                    (
                        "type-parameters".to_owned(),
                        i.type_parameters.len().to_string(),
                    ),
                    (
                        "type-arguments".to_owned(),
                        i.type_arguments.len().to_string(),
                    ),
                    (
                        "implementation-parameters".to_owned(),
                        i.implementation_parameters.len().to_string(),
                    ),
                ]);
                append_record(records, detail)?;
                let implementation = DeclarationReference {
                    package: standard.package,
                    declaration: declaration_id(declaration.header.owner)?,
                };
                for (index, binding) in i.associated_types.iter().enumerate() {
                    let type_path = format!("associated-binding.{}", binding.member);
                    let mut fields = vec![
                        ("owner".to_owned(), declaration.header.owner.to_string()),
                        ("index".to_owned(), index.to_string()),
                        ("member".to_owned(), binding.member.to_string()),
                        ("type-path".to_owned(), type_path.clone()),
                    ];
                    match &binding.value {
                        AssociatedTypeBindingValue::Owned { .. } => fields.extend([
                            ("kind".to_owned(), "owned".to_owned()),
                            ("arity".to_owned(), "0".to_owned()),
                        ]),
                        AssociatedTypeBindingValue::ReadViewFamily {
                            scope_parameter, ..
                        } => {
                            fields.extend([
                                ("kind".to_owned(), "read-view-family".to_owned()),
                                ("arity".to_owned(), "1".to_owned()),
                                ("scope-parameter".to_owned(), scope_parameter.id.to_string()),
                                ("scope-name".to_owned(), scope_parameter.name.to_string()),
                            ]);
                            append_scope_parameters(
                                &format!("{}/{}", declaration.header.owner, binding.member),
                                ReadScopeBinder::AssociatedBinding {
                                    implementation,
                                    member: binding.member,
                                },
                                std::slice::from_ref(scope_parameter),
                                records,
                            )?;
                        }
                    }
                    append_record(
                        records,
                        DiscoveryRecord {
                            operation: "owned.associated-binding".to_owned(),
                            fields,
                        },
                    )?;
                    append_type(standard, &type_path, binding.ty(), records, 0)?;
                }
                append_type(standard, "self", i.self_type, records, 0)?;
                for (index, ty) in i.type_arguments.iter().enumerate() {
                    append_type(standard, &format!("type-argument.{index}"), *ty, records, 0)?;
                }
                for (index, prerequisite) in i.implementation_parameters.iter().enumerate() {
                    append_record(
                        records,
                        DiscoveryRecord::new(
                            "owned.prerequisite",
                            [
                                ("owner", declaration.header.owner.to_string()),
                                ("id", prerequisite.id.to_string()),
                                ("index", index.to_string()),
                                ("name", prerequisite.name.to_string()),
                                (
                                    "contract",
                                    format!(
                                        "{}/{}",
                                        prerequisite.contract.package,
                                        prerequisite.contract.declaration
                                    ),
                                ),
                                ("self", prerequisite.self_type.to_string()),
                                (
                                    "type-arguments",
                                    prerequisite.type_arguments.len().to_string(),
                                ),
                            ],
                        ),
                    )?;
                    append_type(
                        standard,
                        &format!("prerequisite.{}.self", prerequisite.id),
                        prerequisite.self_type,
                        records,
                        0,
                    )?;
                    for (index, ty) in prerequisite.type_arguments.iter().enumerate() {
                        append_type(
                            standard,
                            &format!("prerequisite.{}.type-argument.{index}", prerequisite.id),
                            *ty,
                            records,
                            0,
                        )?;
                    }
                }
                for mapping in &i.methods {
                    append_record(
                        records,
                        DiscoveryRecord::new(
                            "owned.method-implementation",
                            [
                                ("owner", declaration.header.owner.to_string()),
                                ("method", mapping.method.to_string()),
                                (
                                    "function",
                                    format!(
                                        "{}/{}",
                                        mapping.function.package, mapping.function.declaration
                                    ),
                                ),
                                ("type-arguments", mapping.type_arguments.len().to_string()),
                                ("implementations", mapping.implementations.len().to_string()),
                                (
                                    "read-scope-arguments",
                                    mapping.read_scope_arguments.len().to_string(),
                                ),
                            ],
                        ),
                    )?;
                    for (index, ty) in mapping.type_arguments.iter().enumerate() {
                        append_type(
                            standard,
                            &format!("method.{}.type-argument.{index}", mapping.method),
                            *ty,
                            records,
                            0,
                        )?;
                    }
                    for (index, scope) in mapping.read_scope_arguments.iter().enumerate() {
                        append_record(
                            records,
                            DiscoveryRecord::new(
                                "read-scope-argument",
                                [
                                    (
                                        "parent",
                                        format!("{}/{}", declaration.header.owner, mapping.method),
                                    ),
                                    ("slot", "method-mapping".to_owned()),
                                    ("index", index.to_string()),
                                    ("scope", super::cli::read_scope_coordinate(*scope)),
                                ],
                            ),
                        )?;
                    }
                    for (index, operand) in mapping.implementations.iter().enumerate() {
                        append_implementation_operand(
                            standard,
                            declaration.header.owner,
                            mapping.method,
                            index,
                            operand,
                            records,
                        )?;
                    }
                }
            }
            PackageInterfaceDeclarationPayload::Record {
                fields,
                type_parameters,
            }
            | PackageInterfaceDeclarationPayload::OwnedRecord {
                fields,
                type_parameters,
            } => {
                for (index, parameter) in type_parameters.iter().enumerate() {
                    append_child_owner(
                        standard,
                        OwnerKey::TypeParameter(*parameter),
                        index,
                        records,
                    )?;
                }
                append_record(
                    records,
                    declaration_detail(
                        declaration,
                        if matches!(
                            declaration.payload,
                            PackageInterfaceDeclarationPayload::OwnedRecord { .. }
                        ) {
                            "owned-record"
                        } else {
                            "record"
                        },
                        fields.len(),
                        None,
                    ),
                )?;
                for (index, field) in fields.iter().enumerate() {
                    append_child_owner(standard, OwnerKey::Field(*field), index, records)?;
                }
            }
            PackageInterfaceDeclarationPayload::ReadView(view) => {
                append_type_parameters(standard, &view.type_parameters, records)?;
                let mut detail =
                    declaration_detail(declaration, "read-view", view.fields.len(), None);
                detail.fields.extend([
                    (
                        "type-parameters".to_owned(),
                        view.type_parameters.len().to_string(),
                    ),
                    ("read-scope-parameters".to_owned(), "1".to_owned()),
                ]);
                append_record(records, detail)?;
                let reference = DeclarationReference {
                    package: standard.package,
                    declaration: declaration_id(declaration.header.owner)?,
                };
                append_scope_parameters(
                    &declaration.header.owner.to_string(),
                    ReadScopeBinder::Declaration(reference),
                    std::slice::from_ref(&view.scope_parameter),
                    records,
                )?;
                for (index, field) in view.fields.iter().enumerate() {
                    let type_path = format!("view-field.{}", field.id);
                    let mut fields = vec![
                        ("owner".to_owned(), declaration.header.owner.to_string()),
                        ("index".to_owned(), index.to_string()),
                        ("id".to_owned(), field.id.to_string()),
                        ("name".to_owned(), field.name.to_string()),
                        (
                            "reference".to_owned(),
                            format!(
                                "{}/{}/{}",
                                reference.package, reference.declaration, field.id
                            ),
                        ),
                        ("type-path".to_owned(), type_path.clone()),
                    ];
                    match field.kind {
                        ReadViewFieldKind::Ordinary => {
                            fields.push(("kind".to_owned(), "ordinary".to_owned()))
                        }
                        ReadViewFieldKind::ReadReference { read_scope } => fields.extend([
                            ("kind".to_owned(), "read-reference".to_owned()),
                            (
                                "read-scope".to_owned(),
                                super::cli::read_scope_coordinate(read_scope),
                            ),
                        ]),
                    }
                    append_record(
                        records,
                        DiscoveryRecord {
                            operation: "read-view.field".to_owned(),
                            fields,
                        },
                    )?;
                    append_type(standard, &type_path, field.ty, records, 0)?;
                }
            }
            PackageInterfaceDeclarationPayload::Variant {
                cases,
                type_parameters,
            } => {
                for (index, parameter) in type_parameters.iter().enumerate() {
                    append_child_owner(
                        standard,
                        OwnerKey::TypeParameter(*parameter),
                        index,
                        records,
                    )?;
                }
                append_record(
                    records,
                    declaration_detail(declaration, "variant", cases.len(), None),
                )?;
                for (index, case) in cases.iter().enumerate() {
                    append_child_owner(standard, OwnerKey::Case(*case), index, records)?;
                }
            }
            PackageInterfaceDeclarationPayload::Interface { operations } => {
                append_record(
                    records,
                    declaration_detail(declaration, "interface", operations.len(), None),
                )?;
                for (index, operation) in operations.iter().enumerate() {
                    append_child_owner(standard, OwnerKey::Operation(*operation), index, records)?;
                }
            }
            PackageInterfaceDeclarationPayload::External(signature) => {
                append_record(
                    records,
                    declaration_detail(
                        declaration,
                        "external",
                        signature.parameters.len(),
                        Some("pure"),
                    ),
                )?;
                append_type_parameters(standard, &signature.type_parameters, records)?;
                append_parameters(standard, &signature.parameters, records)?;
                append_type(standard, "result", signature.result, records, 0)?;
            }
            PackageInterfaceDeclarationPayload::Function(signature) => {
                let effect = match signature.effect {
                    FunctionEffect::Pure => "pure",
                    FunctionEffect::Task { .. } => "task",
                };
                let mut detail = declaration_detail(
                    declaration,
                    declaration.header.kind.name(),
                    signature.parameters.len(),
                    Some(effect),
                );
                detail.fields.extend([
                    (
                        "read-scope-parameters".to_owned(),
                        signature.read_scope_parameters.len().to_string(),
                    ),
                    (
                        "read-scope-sources".to_owned(),
                        signature.read_scope_sources.len().to_string(),
                    ),
                ]);
                append_record(records, detail)?;
                if let FunctionEffect::Task {
                    effect_parameters,
                    requirements,
                } = &signature.effect
                {
                    for (index, requirement) in requirements.iter().enumerate() {
                        append_record(
                            records,
                            DiscoveryRecord::new(
                                "effect.requirement",
                                [
                                    ("index", index.to_string()),
                                    (
                                        "reference",
                                        format!(
                                            "{}/{}",
                                            requirement.package(),
                                            requirement.owner()
                                        ),
                                    ),
                                ],
                            ),
                        )?;
                    }
                    for (index, parameter) in effect_parameters.iter().enumerate() {
                        append_record(
                            records,
                            DiscoveryRecord::new(
                                "effect.parameter",
                                [
                                    ("index", index.to_string()),
                                    (
                                        "reference",
                                        format!("{}/{}", parameter.package, parameter.parameter),
                                    ),
                                ],
                            ),
                        )?;
                    }
                }
                append_type_parameters(standard, &signature.type_parameters, records)?;
                for (index, witness) in signature.implementation_parameters.iter().enumerate() {
                    append_record(
                        records,
                        DiscoveryRecord::new(
                            "implementation-parameter",
                            [
                                ("id", witness.id.to_string()),
                                ("index", index.to_string()),
                                ("name", witness.name.to_string()),
                                (
                                    "contract",
                                    format!(
                                        "{}/{}",
                                        witness.contract.package, witness.contract.declaration
                                    ),
                                ),
                                ("self", witness.self_type.to_string()),
                                ("type-arguments", witness.type_arguments.len().to_string()),
                            ],
                        ),
                    )?;
                    append_type(
                        standard,
                        &format!("implementation.{index}.self"),
                        witness.self_type,
                        records,
                        0,
                    )?;
                    for (argument_index, ty) in witness.type_arguments.iter().enumerate() {
                        append_type(
                            standard,
                            &format!("implementation.{index}.type-argument.{argument_index}"),
                            *ty,
                            records,
                            0,
                        )?;
                    }
                }
                for (index, parameter) in signature.requirement_parameters.iter().enumerate() {
                    append_child_owner(
                        standard,
                        OwnerKey::RequirementParameter(*parameter),
                        index,
                        records,
                    )?;
                }
                for (index, parameter) in signature.effect_parameters.iter().enumerate() {
                    append_child_owner(
                        standard,
                        OwnerKey::EffectParameter(*parameter),
                        index,
                        records,
                    )?;
                }
                let reference = DeclarationReference {
                    package: standard.package,
                    declaration: declaration_id(declaration.header.owner)?,
                };
                append_scope_parameters(
                    &declaration.header.owner.to_string(),
                    ReadScopeBinder::Declaration(reference),
                    &signature.read_scope_parameters,
                    records,
                )?;
                append_scope_sources(
                    &declaration.header.owner.to_string(),
                    &signature.read_scope_sources,
                    records,
                )?;
                append_parameters(standard, &signature.parameters, records)?;
                append_callable_result(
                    standard,
                    "function.result",
                    &declaration.header.owner.to_string(),
                    "result",
                    &signature.result,
                    records,
                )?;
            }
            PackageInterfaceDeclarationPayload::Constant { ty } => {
                append_record(
                    records,
                    declaration_detail(declaration, "constant", 0, Some("pure")),
                )?;
                append_type(standard, "value", *ty, records, 0)?;
            }
            PackageInterfaceDeclarationPayload::Component {
                requirements,
                ports,
            } => {
                append_record(
                    records,
                    DiscoveryRecord::new(
                        "declaration",
                        [
                            ("kind", "component".to_owned()),
                            ("requirements", requirements.len().to_string()),
                            ("ports", ports.len().to_string()),
                        ],
                    ),
                )?;
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
        PackageInterfaceRecord::RepresentationDeclaration(_) => {
            return Err(discovery_error(
                DiagnosticClass::Source,
                "builtin_owner_missing",
                "private representation evidence is outside the public owner discovery surface",
            ));
        }
        PackageInterfaceRecord::TypeParameter(_)
        | PackageInterfaceRecord::EffectParameter(_)
        | PackageInterfaceRecord::RequirementParameter(_)
        | PackageInterfaceRecord::Field(_)
        | PackageInterfaceRecord::Case(_)
        | PackageInterfaceRecord::Operation(_)
        | PackageInterfaceRecord::Parameter(_)
        | PackageInterfaceRecord::Requirement(_)
        | PackageInterfaceRecord::Port(_) => append_child_detail(standard, record, None, records)?,
    }
    Ok(())
}

fn append_implementation_operand(
    standard: &PackageInterfaceView<'_>,
    owner: OwnerKey,
    method: super::semantic_id::MethodId,
    index: usize,
    operand: &super::kernel::ImplementationOperand,
    records: &mut Vec<DiscoveryRecord>,
) -> Result<(), Diagnostic> {
    use super::kernel::ImplementationOperand;
    let mut pending = vec![(operand, String::from("-"), 0_usize)];
    while let Some((operand, path, depth)) = pending.pop() {
        if depth > 128 || records.len() >= BUILTIN_QUERY_MAXIMUM_ITEMS {
            return Err(discovery_error(
                DiagnosticClass::Resource,
                "builtin_type_projection_limit",
                "package witness projection exceeded its depth or record bound",
            ));
        }
        let (kind, reference, parameter, types, operands) = match operand {
            ImplementationOperand::Concrete {
                implementation,
                type_arguments,
                implementations,
            } => (
                "concrete",
                implementation,
                String::from("-"),
                type_arguments.as_slice(),
                implementations.as_slice(),
            ),
            ImplementationOperand::Parameter { scope, parameter } => {
                ("parameter", scope, parameter.to_string(), &[][..], &[][..])
            }
        };
        append_record(
            records,
            DiscoveryRecord::new(
                "owned.method-implementation-operand",
                [
                    ("owner", owner.to_string()),
                    ("method", method.to_string()),
                    ("index", index.to_string()),
                    ("path", path.clone()),
                    ("kind", kind.to_owned()),
                    (
                        "reference",
                        format!("{}/{}", reference.package, reference.declaration),
                    ),
                    ("parameter", parameter),
                    ("type-arguments", types.len().to_string()),
                    ("implementations", operands.len().to_string()),
                ],
            ),
        )?;
        for (ordinal, ty) in types.iter().enumerate() {
            append_type(
                standard,
                &format!("method.{method}.implementation.{index}.{path}.type-argument.{ordinal}"),
                *ty,
                records,
                0,
            )?;
        }
        for (child, operand) in operands.iter().enumerate().rev() {
            pending.push((
                operand,
                if path == "-" {
                    child.to_string()
                } else {
                    format!("{path}/{child}")
                },
                depth + 1,
            ));
        }
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
    standard: &PackageInterfaceView<'_>,
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
    append_record(records, owner_summary(standard.package, &value.record)?)?;
    append_child_detail(standard, &value.record, Some(index), records)
}

fn append_child_detail(
    standard: &PackageInterfaceView<'_>,
    record: &PackageInterfaceRecord,
    index: Option<usize>,
    records: &mut Vec<DiscoveryRecord>,
) -> Result<(), Diagnostic> {
    let mut common = Vec::new();
    if let Some(index) = index {
        common.push(("index".to_owned(), index.to_string()));
    }
    match record {
        PackageInterfaceRecord::RequirementParameter(parameter) => {
            common.push(("name".to_owned(), parameter.name.as_str().to_owned()));
            common.push(("identity".to_owned(), parameter.header.owner.to_string()));
            common.push(("declaration".to_owned(), parameter.declaration.to_string()));
            append_record(
                records,
                DiscoveryRecord {
                    operation: "requirement-parameter".to_owned(),
                    fields: {
                        common.push((
                            "interface".to_owned(),
                            format!(
                                "{}/{}",
                                parameter.constraint.interface.package,
                                parameter.constraint.interface.declaration
                            ),
                        ));
                        common.push((
                            "minimum-operations".to_owned(),
                            parameter.constraint.operations.len().to_string(),
                        ));
                        common
                    },
                },
            )?;
            for (index, operation) in parameter.constraint.operations.iter().enumerate() {
                append_record(
                    records,
                    DiscoveryRecord::new(
                        "requirement-parameter.operation",
                        [
                            ("parent", parameter.header.owner.to_string()),
                            ("index", index.to_string()),
                            (
                                "reference",
                                format!("{}/{}", operation.package, operation.operation),
                            ),
                        ],
                    ),
                )?;
            }
        }
        PackageInterfaceRecord::EffectParameter(parameter) => {
            common.push(("name".to_owned(), parameter.name.as_str().to_owned()));
            common.push(("identity".to_owned(), parameter.header.owner.to_string()));
            common.push(("declaration".to_owned(), parameter.declaration.to_string()));
            append_record(
                records,
                DiscoveryRecord {
                    operation: "effect-parameter".to_owned(),
                    fields: common,
                },
            )?;
        }
        PackageInterfaceRecord::TypeParameter(parameter) => {
            common.push(("name".to_owned(), parameter.name.as_str().to_owned()));
            common.push((
                "constraint".to_owned(),
                parameter.constraints.name().to_owned(),
            ));
            append_record(
                records,
                DiscoveryRecord {
                    operation: "type-parameter".to_owned(),
                    fields: common,
                },
            )?;
        }
        PackageInterfaceRecord::Field(field) => {
            common.push(("name".to_owned(), field.name.as_str().to_owned()));
            common.push(("type-path".to_owned(), format!("field.{}", field.name)));
            append_record(
                records,
                DiscoveryRecord {
                    operation: "field".to_owned(),
                    fields: common,
                },
            )?;
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
            append_record(
                records,
                DiscoveryRecord {
                    operation: "case".to_owned(),
                    fields: common,
                },
            )?;
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
            append_record(
                records,
                DiscoveryRecord {
                    operation: "operation".to_owned(),
                    fields: common,
                },
            )?;
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
            append_record(
                records,
                DiscoveryRecord {
                    operation: "parameter".to_owned(),
                    fields: common,
                },
            )?;
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
            append_record(
                records,
                DiscoveryRecord {
                    operation: "requirement".to_owned(),
                    fields: common,
                },
            )?;
            for (index, operation) in requirement.operations.iter().enumerate() {
                append_record(
                    records,
                    DiscoveryRecord::new(
                        "requirement.operation",
                        [
                            ("index", index.to_string()),
                            (
                                "reference",
                                format!("{}/{}", operation.package, operation.operation),
                            ),
                        ],
                    ),
                )?;
            }
            for (index, limit) in requirement.limits.iter().enumerate() {
                append_record(
                    records,
                    DiscoveryRecord::new(
                        "requirement.limit",
                        [
                            ("index", index.to_string()),
                            ("name", limit.name.as_str().to_owned()),
                            ("maximum", limit.maximum.to_string()),
                            ("unit", format!("{:?}", limit.unit).to_ascii_lowercase()),
                        ],
                    ),
                )?;
            }
        }
        PackageInterfaceRecord::Port(port) => {
            common.extend([
                ("name".to_owned(), port.name.as_str().to_owned()),
                ("type-path".to_owned(), "function".to_owned()),
            ]);
            append_record(
                records,
                DiscoveryRecord {
                    operation: "port".to_owned(),
                    fields: common,
                },
            )?;
            append_type(standard, "function", port.function_type, records, 0)?;
        }
        PackageInterfaceRecord::Declaration(_)
        | PackageInterfaceRecord::RepresentationDeclaration(_) => {
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
    standard: &PackageInterfaceView<'_>,
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
    standard: &PackageInterfaceView<'_>,
    parameters: &[super::semantic_id::ParameterId],
    records: &mut Vec<DiscoveryRecord>,
) -> Result<(), Diagnostic> {
    for (index, parameter) in parameters.iter().enumerate() {
        append_child_owner(standard, OwnerKey::Parameter(*parameter), index, records)?;
    }
    Ok(())
}

fn append_type(
    standard: &PackageInterfaceView<'_>,
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
        TypeForm::AssociatedContractMember { contract, member }
        | TypeForm::AssociatedReadViewMember {
            contract, member, ..
        } => {
            fields.extend([
                (
                    "contract".to_owned(),
                    format!("{}/{}", contract.package, contract.declaration),
                ),
                ("member".to_owned(), member.to_string()),
            ]);
        }
        TypeForm::AssociatedWitnessProjection {
            scope,
            parameter,
            contract,
            member,
        }
        | TypeForm::AssociatedReadViewProjection {
            scope,
            parameter,
            contract,
            member,
            ..
        } => {
            fields.extend([
                (
                    "scope".to_owned(),
                    format!("{}/{}", scope.package, scope.declaration),
                ),
                ("parameter".to_owned(), parameter.to_string()),
                (
                    "contract".to_owned(),
                    format!("{}/{}", contract.package, contract.declaration),
                ),
                ("member".to_owned(), member.to_string()),
            ]);
        }
        TypeForm::ReadViewApplied {
            declaration,
            arguments,
            ..
        } => fields.extend([
            (
                "reference".to_owned(),
                format!("{}/{}", declaration.package, declaration.declaration),
            ),
            ("arguments".to_owned(), arguments.len().to_string()),
        ]),
        TypeForm::TypeParameter { parameter } => {
            fields.push(("parameter".to_owned(), parameter.to_string()));
        }
        TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. } => {
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
        TypeForm::OwnedChoice { cases } => {
            fields.push(("cases".to_owned(), cases.len().to_string()));
        }
        TypeForm::StructuralRecord { fields: structural }
        | TypeForm::OwnedProduct { fields: structural } => {
            fields.push(("fields".to_owned(), structural.len().to_string()));
        }
        TypeForm::Function { parameters, .. } | TypeForm::TaskFunction { parameters, .. } => {
            fields.push(("parameters".to_owned(), parameters.len().to_string()));
        }
        TypeForm::Unit
        | TypeForm::Bool
        | TypeForm::I64
        | TypeForm::F64
        | TypeForm::ByteBuffer
        | TypeForm::OwnedI64Cell
        | TypeForm::Bytes
        | TypeForm::Text
        | TypeForm::StaticText
        | TypeForm::Secret
        | TypeForm::OwnedSequence { .. }
        | TypeForm::List { .. }
        | TypeForm::Map { .. }
        | TypeForm::Option { .. }
        | TypeForm::Result { .. }
        | TypeForm::Stream { .. } => {}
    }
    if let TypeForm::ReadViewApplied { read_scope, .. }
    | TypeForm::AssociatedReadViewMember { read_scope, .. }
    | TypeForm::AssociatedReadViewProjection { read_scope, .. } = &ty.form
    {
        fields.push((
            "read-scope".to_owned(),
            super::cli::read_scope_coordinate(*read_scope),
        ));
    }
    append_record(
        records,
        DiscoveryRecord {
            operation: "type".to_owned(),
            fields,
        },
    )?;
    if let TypeForm::TaskFunction { effect, .. } = &ty.form {
        append_record(
            records,
            DiscoveryRecord::new(
                "type.effect",
                [
                    ("path", path.to_owned()),
                    ("requirements", effect.requirements.len().to_string()),
                    ("parameters", effect.parameters.len().to_string()),
                ],
            ),
        )?;
        for (index, requirement) in effect.requirements.iter().enumerate() {
            append_record(
                records,
                DiscoveryRecord::new(
                    "type.effect-requirement",
                    [
                        ("path", path.to_owned()),
                        ("index", index.to_string()),
                        (
                            "reference",
                            format!("{}/{}", requirement.package(), requirement.owner()),
                        ),
                    ],
                ),
            )?;
        }
        for (index, parameter) in effect.parameters.iter().enumerate() {
            append_record(
                records,
                DiscoveryRecord::new(
                    "type.effect-parameter",
                    [
                        ("path", path.to_owned()),
                        ("index", index.to_string()),
                        (
                            "reference",
                            format!("{}/{}", parameter.package, parameter.parameter),
                        ),
                    ],
                ),
            )?;
        }
    }
    match &ty.form {
        TypeForm::AssociatedContractMember { .. }
        | TypeForm::AssociatedWitnessProjection { .. }
        | TypeForm::AssociatedReadViewMember { .. }
        | TypeForm::AssociatedReadViewProjection { .. } => {}
        TypeForm::Applied { arguments, .. } | TypeForm::ReadViewApplied { arguments, .. } => {
            for (index, argument) in arguments.iter().enumerate() {
                append_type(
                    standard,
                    &format!("{path}.argument.{index}"),
                    *argument,
                    records,
                    depth + 1,
                )?;
            }
        }
        TypeForm::StructuralRecord { fields }
        | TypeForm::OwnedProduct { fields }
        | TypeForm::OwnedChoice { cases: fields } => {
            let edge = if matches!(ty.form, TypeForm::OwnedChoice { .. }) {
                "type.case"
            } else {
                "type.field"
            };
            for (index, field) in fields.iter().enumerate() {
                let child = format!("{path}.{}", field.name);
                append_record(
                    records,
                    DiscoveryRecord::new(
                        edge,
                        [
                            ("parent", path.to_owned()),
                            ("index", index.to_string()),
                            ("name", field.name.as_str().to_owned()),
                            ("type-path", child.clone()),
                        ],
                    ),
                )?;
                append_type(standard, &child, field.ty, records, depth + 1)?;
            }
        }
        TypeForm::List { item }
        | TypeForm::Option { item }
        | TypeForm::Stream { item }
        | TypeForm::OwnedSequence { item } => {
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
        TypeForm::Function { parameters, result }
        | TypeForm::TaskFunction {
            parameters, result, ..
        } => {
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
        | TypeForm::F64
        | TypeForm::ByteBuffer
        | TypeForm::OwnedI64Cell
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

fn public_interface_owner(
    standard: &PackageInterfaceView<'_>,
    record: &PackageInterfaceRecord,
) -> bool {
    let layout_field = matches!(record, PackageInterfaceRecord::Field(_));
    let mut current = record;
    for _ in 0..=standard.interface_owners.len() {
        if matches!(
            current,
            PackageInterfaceRecord::RepresentationDeclaration(_)
        ) {
            return false;
        }
        if layout_field
            && matches!(current, PackageInterfaceRecord::Declaration(declaration)
                if matches!(declaration.payload, PackageInterfaceDeclarationPayload::OwnedRecord { .. }))
        {
            return false;
        }
        let Some(parent) = owner_parent(current) else {
            return true;
        };
        let Some(value) = standard.interface_owners.get(&parent) else {
            return false;
        };
        current = &value.record;
    }
    false
}

fn owner_name(record: &PackageInterfaceRecord) -> &str {
    match record {
        PackageInterfaceRecord::Declaration(record)
        | PackageInterfaceRecord::RepresentationDeclaration(record) => record.name.as_str(),
        PackageInterfaceRecord::TypeParameter(record) => record.name.as_str(),
        PackageInterfaceRecord::RequirementParameter(record) => record.name.as_str(),
        PackageInterfaceRecord::EffectParameter(record) => record.name.as_str(),
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
        PackageInterfaceRecord::Declaration(_)
        | PackageInterfaceRecord::RepresentationDeclaration(_) => None,
        PackageInterfaceRecord::TypeParameter(record) => {
            Some(OwnerKey::Declaration(record.declaration))
        }
        PackageInterfaceRecord::RequirementParameter(record) => {
            Some(OwnerKey::Declaration(record.declaration))
        }
        PackageInterfaceRecord::EffectParameter(record) => {
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
        TypeForm::ReadViewApplied { .. } => "read-view-application",
        TypeForm::AssociatedReadViewMember { .. } => "associated-read-view-member",
        TypeForm::AssociatedReadViewProjection { .. } => "associated-read-view-projection",
        TypeForm::AssociatedContractMember { .. } => "associated-contract-member",
        TypeForm::AssociatedWitnessProjection { .. } => "associated-witness-projection",
        TypeForm::Unit => "unit",
        TypeForm::Bool => "bool",
        TypeForm::I64 => "i64",
        TypeForm::F64 => "f64",
        TypeForm::ByteBuffer => "byte-buffer",
        TypeForm::OwnedI64Cell => "owned-i64-cell",
        TypeForm::OwnedProduct { .. } => "owned-product",
        TypeForm::OwnedChoice { .. } => "owned-choice",
        TypeForm::OwnedSequence { .. } => "owned-sequence",
        TypeForm::Bytes => "bytes",
        TypeForm::Text => "text",
        TypeForm::StaticText => "static-text",
        TypeForm::Secret => "secret",
        TypeForm::TypeParameter { .. } => "parameter",
        TypeForm::Named { .. } => "named",
        TypeForm::Applied { .. } => "application",
        TypeForm::CapabilityResource { .. } => "capability-resource",
        TypeForm::StructuralRecord { .. } => "structural-record",
        TypeForm::List { .. } => "list",
        TypeForm::Map { .. } => "map",
        TypeForm::Option { .. } => "option",
        TypeForm::Result { .. } => "result",
        TypeForm::Stream { .. } => "stream",
        TypeForm::Function { .. } => "function",
        TypeForm::TaskFunction { .. } => "task_function",
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
    standard: &PackageInterfaceView<'_>,
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
    standard: &PackageInterfaceView<'_>,
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

    use super::super::kernel::{
        AssociatedTypeBinding, AssociatedTypeMember, ImplementationParameter, Name, OwnedContract,
        OwnedImplementation, OwnedMethod, OwnedMethodImplementation, OwnedMethodParameter,
        OwnerHeader, PackageFunctionSignature, PackageInterfaceDeclaration, ParameterRecord,
        ParameterUse, ReadViewDeclaration, ReadViewField, TypeObject, TypeParameterConstraints,
        TypeParameterRecord,
    };
    use super::super::semantic_id::{
        AssociatedTypeMemberId, BindingId, DeclarationId, FieldId, ImplementationParameterId,
        MethodId, ParameterId, ReadScopeParameterId, TypeParameterId,
    };
    use std::collections::BTreeMap;

    struct InterfaceFixture {
        package: PackageId,
        owners: BTreeMap<OwnerKey, super::super::package_interface::PackageInterfaceOwner>,
        types: BTreeMap<TypeObjectDigest, TypeObject>,
    }

    impl InterfaceFixture {
        fn new() -> Self {
            Self {
                package: "pkg_20000000000000000000000000000001"
                    .parse()
                    .expect("package"),
                owners: BTreeMap::new(),
                types: BTreeMap::new(),
            }
        }

        fn reference(&self, ordinal: u64) -> DeclarationReference {
            DeclarationReference {
                package: self.package,
                declaration: DeclarationId::migrate(b"builtin-read-view-observation", ordinal),
            }
        }

        fn view(&self) -> PackageInterfaceView<'_> {
            PackageInterfaceView {
                package: self.package,
                package_revision: super::super::kernel::PackageRevisionDigest::of(
                    b"discovery-fixture",
                ),
                interface_owners: &self.owners,
                interface_types: &self.types,
            }
        }

        fn ty(&mut self, form: TypeForm) -> TypeObjectDigest {
            let object = TypeObject::new(form).expect("type shape");
            let (digest, _) =
                super::super::kernel::encode_type_object(&object).expect("exact type");
            self.types.insert(digest, object);
            digest
        }

        fn record(&mut self, record: PackageInterfaceRecord) {
            let owner = record.header().owner;
            assert!(
                self.owners
                    .insert(
                        owner,
                        super::super::package_interface::PackageInterfaceOwner {
                            contract_version: crate::platform::canonical_schema::SCHEMA_EPOCH,
                            record,
                        }
                    )
                    .is_none()
            );
        }

        fn declaration(
            &mut self,
            reference: DeclarationReference,
            name: &str,
            kind: OwnerKind,
            payload: PackageInterfaceDeclarationPayload,
        ) {
            self.record(PackageInterfaceRecord::Declaration(
                PackageInterfaceDeclaration {
                    header: OwnerHeader::new(OwnerKey::Declaration(reference.declaration), kind),
                    name: Name::new(name).expect("name"),
                    payload,
                },
            ));
        }

        fn type_parameter(
            &mut self,
            reference: DeclarationReference,
            ordinal: u64,
            constraints: TypeParameterConstraints,
        ) -> TypeParameterId {
            let id = TypeParameterId::migrate(b"builtin-read-view-observation", ordinal);
            self.record(PackageInterfaceRecord::TypeParameter(TypeParameterRecord {
                header: OwnerHeader::new(OwnerKey::TypeParameter(id), OwnerKind::TypeParameter),
                declaration: reference.declaration,
                name: Name::new("T").expect("name"),
                constraints,
            }));
            id
        }

        fn parameter(
            &mut self,
            reference: DeclarationReference,
            ordinal: u64,
            ty: TypeObjectDigest,
        ) -> ParameterId {
            let id = ParameterId::migrate(b"builtin-read-view-observation", ordinal);
            self.record(PackageInterfaceRecord::Parameter(ParameterRecord {
                header: OwnerHeader::new(OwnerKey::Parameter(id), OwnerKind::Parameter),
                parent: ParameterParent::Function(reference.declaration),
                name: Name::new(format!("source-{ordinal}")).expect("name"),
                ty,
                use_mode: ParameterUse::Borrow,
                resource_requirement: None,
            }));
            id
        }
    }

    fn scope_parameter(ordinal: u64, name: &str) -> ReadScopeParameter {
        ReadScopeParameter {
            id: ReadScopeParameterId::migrate(b"builtin-read-view-observation", ordinal),
            name: Name::new(name).expect("scope name"),
        }
    }

    fn parameter_scope(
        binder: ReadScopeBinder,
        parameter: &ReadScopeParameter,
    ) -> ReadScopeArgument {
        ReadScopeArgument::Parameter(ReadScopeReference {
            binder,
            parameter: parameter.id,
        })
    }

    fn select<'a>(records: &'a [DiscoveryRecord], operation: &str) -> Vec<&'a DiscoveryRecord> {
        records
            .iter()
            .filter(|record| record.operation == operation)
            .collect()
    }

    fn field<'a>(record: &'a DiscoveryRecord, name: &str) -> &'a str {
        record
            .fields
            .iter()
            .find_map(|(key, value)| (key == name).then_some(value.as_str()))
            .expect("required observation field")
    }

    #[test]
    fn interface_discovery_preserves_result_relations_and_unused_scope_inventories() {
        let mut fixture = InterfaceFixture::new();
        let function = fixture.reference(1);
        let view = fixture.reference(2);
        let scopes = [scope_parameter(1, "active"), scope_parameter(2, "unused")];
        let active = parameter_scope(ReadScopeBinder::Declaration(function), &scopes[0]);
        let unused = parameter_scope(ReadScopeBinder::Declaration(function), &scopes[1]);
        let cell = fixture.ty(TypeForm::OwnedI64Cell);
        let integer = fixture.ty(TypeForm::I64);
        let view_ty = fixture.ty(TypeForm::ReadViewApplied {
            declaration: view,
            arguments: vec![integer],
            read_scope: active,
        });
        let source = fixture.parameter(function, 1, cell);
        let other = fixture.parameter(function, 2, cell);
        fixture.declaration(
            function,
            "view-result",
            OwnerKind::PureFunction,
            PackageInterfaceDeclarationPayload::Function(PackageFunctionSignature {
                implementation_parameters: vec![],
                requirement_parameters: vec![],
                effect_parameters: vec![],
                type_parameters: vec![],
                parameters: vec![source, other],
                result: CallableResult {
                    ty: view_ty,
                    relation: ResultRelation::ViewFrom {
                        source,
                        scope: active,
                    },
                },
                read_scope_parameters: scopes.to_vec(),
                read_scope_sources: vec![
                    ReadScopeSource {
                        source,
                        scope: active,
                    },
                    ReadScopeSource {
                        source: other,
                        scope: unused,
                    },
                ],
                effect: FunctionEffect::Pure,
            }),
        );
        let owner = OwnerKey::Declaration(function.declaration);
        let records = inspect_interface_owner(&fixture.view(), OwnerKind::PureFunction, owner)
            .expect("complete function observation");
        assert_eq!(
            select(&records, "function.result"),
            vec![&DiscoveryRecord::new(
                "function.result",
                [
                    ("parent", owner.to_string()),
                    ("type-path", "result".to_owned()),
                    ("result-kind", "view-from".to_owned()),
                    ("result-source", source.to_string()),
                    (
                        "result-scope",
                        format!(
                            "parameter:declaration:{}/{}:{}",
                            function.package, function.declaration, scopes[0].id
                        )
                    ),
                ]
            )]
        );
        assert_eq!(select(&records, "read-scope-parameter").len(), 2);
        assert_eq!(select(&records, "read-scope-source").len(), 2);
        assert_eq!(
            field(select(&records, "read-scope-parameter")[1], "scope"),
            format!(
                "parameter:declaration:{}/{}:{}",
                function.package, function.declaration, scopes[1].id
            )
        );
        assert_eq!(
            field(select(&records, "read-scope-source")[1], "source"),
            other.to_string()
        );
        assert_eq!(
            field(select(&records, "read-scope-source")[1], "scope"),
            format!(
                "parameter:declaration:{}/{}:{}",
                function.package, function.declaration, scopes[1].id
            )
        );

        for (ty, relation, expected_kind, expected_source) in [
            (
                cell,
                ResultRelation::ReadFrom { source },
                "read-from",
                source.to_string(),
            ),
            (integer, ResultRelation::Value, "value", "-".to_owned()),
        ] {
            let PackageInterfaceRecord::Declaration(declaration) =
                &mut fixture.owners.get_mut(&owner).expect("function").record
            else {
                unreachable!()
            };
            let PackageInterfaceDeclarationPayload::Function(signature) = &mut declaration.payload
            else {
                unreachable!()
            };
            signature.result = CallableResult { ty, relation };
            let records = inspect_interface_owner(&fixture.view(), OwnerKind::PureFunction, owner)
                .expect("result variant");
            let result = select(&records, "function.result")[0];
            assert_eq!(field(result, "result-kind"), expected_kind);
            assert_eq!(field(result, "result-source"), expected_source);
            assert_eq!(field(result, "result-scope"), "-");
            assert!(
                select(&records, "type")
                    .iter()
                    .any(|record| field(record, "path") == "result"
                        && field(record, "digest") == ty.to_string())
            );
        }
    }

    #[test]
    fn interface_discovery_keeps_read_view_fields_phantom_actuals_and_private_evidence_distinct() {
        let mut fixture = InterfaceFixture::new();
        let view = fixture.reference(3);
        let scope = scope_parameter(3, "source");
        let scope_actual = parameter_scope(ReadScopeBinder::Declaration(view), &scope);
        let phantom = fixture.type_parameter(view, 3, TypeParameterConstraints::None);
        let integer = fixture.ty(TypeForm::I64);
        let cell = fixture.ty(TypeForm::OwnedI64Cell);
        let ordinary = FieldId::migrate(b"builtin-read-view-observation", 1);
        let borrowed = FieldId::migrate(b"builtin-read-view-observation", 2);
        fixture.declaration(
            view,
            "View",
            OwnerKind::ReadView,
            PackageInterfaceDeclarationPayload::ReadView(ReadViewDeclaration {
                type_parameters: vec![phantom],
                scope_parameter: scope.clone(),
                fields: vec![
                    ReadViewField {
                        id: ordinary,
                        name: Name::new("offset").expect("name"),
                        ty: integer,
                        kind: ReadViewFieldKind::Ordinary,
                    },
                    ReadViewField {
                        id: borrowed,
                        name: Name::new("child").expect("name"),
                        ty: cell,
                        kind: ReadViewFieldKind::ReadReference {
                            read_scope: scope_actual,
                        },
                    },
                ],
            }),
        );
        let owner = OwnerKey::Declaration(view.declaration);
        assert_eq!(
            parse_interface_owner_kind("read_view").expect("view public kind"),
            OwnerKind::ReadView
        );
        let page = query_interface_owners(
            &fixture.view(),
            &BuiltinOwnerSelector {
                kind: Some(OwnerKind::ReadView),
                ..BuiltinOwnerSelector::default()
            },
            1,
            64 * 1024,
            None,
        )
        .expect("view owner page");
        assert_eq!((page.matched, page.returned), (1, 1));
        let records = inspect_interface_owner(&fixture.view(), OwnerKind::ReadView, owner)
            .expect("view detail");
        assert_eq!(
            select(&records, "read-view.field"),
            vec![
                &DiscoveryRecord::new(
                    "read-view.field",
                    [
                        ("owner", owner.to_string()),
                        ("index", "0".to_owned()),
                        ("id", ordinary.to_string()),
                        ("name", "offset".to_owned()),
                        (
                            "reference",
                            format!("{}/{}/{ordinary}", view.package, view.declaration)
                        ),
                        ("type-path", format!("view-field.{ordinary}")),
                        ("kind", "ordinary".to_owned()),
                    ]
                ),
                &DiscoveryRecord::new(
                    "read-view.field",
                    [
                        ("owner", owner.to_string()),
                        ("index", "1".to_owned()),
                        ("id", borrowed.to_string()),
                        ("name", "child".to_owned()),
                        (
                            "reference",
                            format!("{}/{}/{borrowed}", view.package, view.declaration)
                        ),
                        ("type-path", format!("view-field.{borrowed}")),
                        ("kind", "read-reference".to_owned()),
                        (
                            "read-scope",
                            format!(
                                "parameter:declaration:{}/{}:{}",
                                view.package, view.declaration, scope.id
                            )
                        ),
                    ]
                ),
            ]
        );
        assert_eq!(
            select(&records, "type-parameter").len(),
            1,
            "unused formal stays visible"
        );
        let applied = fixture.ty(TypeForm::ReadViewApplied {
            declaration: view,
            arguments: vec![integer],
            read_scope: scope_actual,
        });
        let mut type_records = vec![];
        append_type(
            &fixture.view(),
            "application",
            applied,
            &mut type_records,
            0,
        )
        .expect("phantom actual");
        assert_eq!(select(&type_records, "type").len(), 2);
        assert_eq!(
            field(select(&type_records, "type")[1], "path"),
            "application.argument.0"
        );
        assert_eq!(
            field(select(&type_records, "type")[1], "digest"),
            integer.to_string()
        );

        fixture.types.remove(&cell);
        assert_eq!(
            inspect_interface_owner(&fixture.view(), OwnerKind::ReadView, owner)
                .expect_err("unused field root must still exist")
                .code,
            "builtin_interface_type"
        );
        let PackageInterfaceRecord::Declaration(declaration) =
            fixture.owners[&owner].record.clone()
        else {
            unreachable!()
        };
        fixture.owners.get_mut(&owner).expect("view").record =
            PackageInterfaceRecord::RepresentationDeclaration(declaration);
        assert_eq!(
            query_interface_owners(
                &fixture.view(),
                &BuiltinOwnerSelector::default(),
                10,
                64 * 1024,
                None
            )
            .expect("private evidence page")
            .matched,
            0
        );
        assert_eq!(
            inspect_interface_owner(&fixture.view(), OwnerKind::ReadView, owner)
                .expect_err("private view authority is hidden")
                .code,
            "builtin_owner_missing"
        );
    }

    #[test]
    fn interface_discovery_observes_unused_families_complete_methods_and_mapping_scopes() {
        let mut fixture = InterfaceFixture::new();
        let contract = fixture.reference(4);
        let implementation = fixture.reference(5);
        let view = fixture.reference(6);
        let target = fixture.reference(7);
        let member = AssociatedTypeMemberId::migrate(b"builtin-read-view-observation", 1);
        let family = AssociatedTypeMemberId::migrate(b"builtin-read-view-observation", 2);
        let method = MethodId::migrate(b"builtin-read-view-observation", 1);
        let family_scope = scope_parameter(4, "family");
        let binding_scope = scope_parameter(5, "binding");
        let method_scope = scope_parameter(6, "method");
        let unused_scope = scope_parameter(7, "unused-method");
        let scope = parameter_scope(
            ReadScopeBinder::ContractMethod { contract, method },
            &method_scope,
        );
        let self_parameter = fixture.type_parameter(contract, 4, TypeParameterConstraints::Owned);
        let self_ty = fixture.ty(TypeForm::TypeParameter {
            parameter: self_parameter,
        });
        let integer = fixture.ty(TypeForm::I64);
        let cell = fixture.ty(TypeForm::OwnedI64Cell);
        let result = fixture.ty(TypeForm::AssociatedReadViewMember {
            contract,
            member: family,
            read_scope: scope,
        });
        fixture.declaration(
            contract,
            "Read",
            OwnerKind::OwnedContract,
            PackageInterfaceDeclarationPayload::OwnedContract(OwnedContract {
                self_parameter,
                type_parameters: vec![],
                associated_types: vec![
                    AssociatedTypeMember {
                        id: member,
                        name: Name::new("UnusedOwned").expect("name"),
                        bound: AssociatedTypeBound::Owned,
                    },
                    AssociatedTypeMember {
                        id: family,
                        name: Name::new("UnusedView").expect("name"),
                        bound: AssociatedTypeBound::ReadViewFamily {
                            scope_parameter: family_scope.clone(),
                        },
                    },
                ],
                methods: vec![OwnedMethod {
                    id: method,
                    name: Name::new("view").expect("name"),
                    parameters: vec![OwnedMethodParameter {
                        ty: self_ty,
                        use_mode: ParameterUse::Borrow,
                    }],
                    result: CallableResult {
                        ty: result,
                        relation: ResultRelation::ViewFrom { source: 0, scope },
                    },
                    read_scope_parameters: vec![method_scope.clone(), unused_scope.clone()],
                    read_scope_sources: vec![ReadScopeSource { source: 0, scope }],
                    effect: FunctionEffect::Task {
                        effect_parameters: vec![],
                        requirements: vec![],
                    },
                }],
            }),
        );
        let contract_records = inspect_interface_owner(
            &fixture.view(),
            OwnerKind::OwnedContract,
            OwnerKey::Declaration(contract.declaration),
        )
        .expect("complete contract");
        let members = select(&contract_records, "owned.associated-type");
        assert_eq!(members.len(), 2);
        assert_eq!(
            (field(members[0], "kind"), field(members[0], "arity")),
            ("owned", "0")
        );
        assert_eq!(
            (field(members[1], "kind"), field(members[1], "arity")),
            ("read-view-family", "1")
        );
        assert_eq!(
            field(members[1], "scope-parameter"),
            family_scope.id.to_string()
        );
        assert_eq!(
            field(
                select(&contract_records, "read-scope-parameter")[0],
                "scope"
            ),
            format!(
                "parameter:associated-member:{}/{}:{family}:{}",
                contract.package, contract.declaration, family_scope.id
            )
        );
        assert_eq!(select(&contract_records, "read-scope-parameter").len(), 3);
        assert_eq!(
            field(select(&contract_records, "owned.method")[0], "effect"),
            "task",
            "empty effects remain Task"
        );
        assert_eq!(
            field(
                select(&contract_records, "owned.method-parameter")[0],
                "use"
            ),
            "borrow"
        );
        let result_record = select(&contract_records, "owned.method-result")[0];
        assert_eq!(field(result_record, "result-kind"), "view-from");
        assert_eq!(field(result_record, "result-source"), "0");
        assert_eq!(
            field(result_record, "result-scope"),
            format!(
                "parameter:contract-method:{}/{}:{method}:{}",
                contract.package, contract.declaration, method_scope.id
            )
        );

        let bound_ty = fixture.ty(TypeForm::ReadViewApplied {
            declaration: view,
            arguments: vec![integer],
            read_scope: parameter_scope(
                ReadScopeBinder::AssociatedBinding {
                    implementation,
                    member: family,
                },
                &binding_scope,
            ),
        });
        fixture.declaration(
            implementation,
            "Selected",
            OwnerKind::OwnedImplementation,
            PackageInterfaceDeclarationPayload::OwnedImplementation(OwnedImplementation {
                type_parameters: vec![],
                implementation_parameters: vec![ImplementationParameter {
                    id: ImplementationParameterId::migrate(b"builtin-read-view-observation", 1),
                    name: Name::new("unused-witness").expect("name"),
                    contract,
                    self_type: cell,
                    type_arguments: vec![],
                }],
                contract,
                self_type: cell,
                type_arguments: vec![],
                associated_types: vec![
                    AssociatedTypeBinding {
                        member,
                        value: AssociatedTypeBindingValue::Owned { ty: cell },
                    },
                    AssociatedTypeBinding {
                        member: family,
                        value: AssociatedTypeBindingValue::ReadViewFamily {
                            scope_parameter: binding_scope.clone(),
                            ty: bound_ty,
                        },
                    },
                ],
                methods: vec![OwnedMethodImplementation {
                    method,
                    function: target,
                    type_arguments: vec![],
                    implementations: vec![],
                    read_scope_arguments: vec![scope],
                }],
            }),
        );
        let owner = OwnerKey::Declaration(implementation.declaration);
        let records =
            inspect_interface_owner(&fixture.view(), OwnerKind::OwnedImplementation, owner)
                .expect("complete implementation");
        let bindings = select(&records, "owned.associated-binding");
        assert_eq!(bindings.len(), 2);
        assert_eq!(
            (field(bindings[0], "kind"), field(bindings[0], "arity")),
            ("owned", "0")
        );
        assert_eq!(
            (field(bindings[1], "kind"), field(bindings[1], "arity")),
            ("read-view-family", "1")
        );
        assert_eq!(
            field(bindings[1], "scope-parameter"),
            binding_scope.id.to_string()
        );
        assert_eq!(
            field(select(&records, "read-scope-parameter")[0], "scope"),
            format!(
                "parameter:associated-binding:{}/{}:{family}:{}",
                implementation.package, implementation.declaration, binding_scope.id
            )
        );
        assert_eq!(select(&records, "owned.prerequisite").len(), 1);
        assert_eq!(
            select(&records, "read-scope-argument"),
            vec![&DiscoveryRecord::new(
                "read-scope-argument",
                [
                    ("parent", format!("{owner}/{method}")),
                    ("slot", "method-mapping".to_owned()),
                    ("index", "0".to_owned()),
                    (
                        "scope",
                        format!(
                            "parameter:contract-method:{}/{}:{method}:{}",
                            contract.package, contract.declaration, method_scope.id
                        )
                    ),
                ]
            )]
        );
        fixture.types.remove(&bound_ty);
        assert_eq!(
            inspect_interface_owner(&fixture.view(), OwnerKind::OwnedImplementation, owner)
                .expect_err("unused family RHS is observed")
                .code,
            "builtin_interface_type"
        );
    }

    #[test]
    fn interface_discovery_distinguishes_exact_associated_and_view_projection_coordinates() {
        let mut fixture = InterfaceFixture::new();
        let contract = fixture.reference(8);
        let function = fixture.reference(9);
        let member = AssociatedTypeMemberId::migrate(b"builtin-read-view-observation", 3);
        let family = AssociatedTypeMemberId::migrate(b"builtin-read-view-observation", 4);
        let witness = ImplementationParameterId::migrate(b"builtin-read-view-observation", 2);
        let other_witness = ImplementationParameterId::migrate(b"builtin-read-view-observation", 3);
        let binding = BindingId::migrate(b"builtin-read-view-observation", 1);
        let lexical = ReadScopeArgument::Lexical {
            callable: function,
            binding,
        };
        let forms = [
            (
                TypeForm::AssociatedContractMember { contract, member },
                "associated-contract-member",
                false,
                false,
            ),
            (
                TypeForm::AssociatedWitnessProjection {
                    scope: function,
                    parameter: witness,
                    contract,
                    member,
                },
                "associated-witness-projection",
                true,
                false,
            ),
            (
                TypeForm::AssociatedReadViewMember {
                    contract,
                    member: family,
                    read_scope: lexical,
                },
                "associated-read-view-member",
                false,
                true,
            ),
            (
                TypeForm::AssociatedReadViewProjection {
                    scope: function,
                    parameter: witness,
                    contract,
                    member: family,
                    read_scope: lexical,
                },
                "associated-read-view-projection",
                true,
                true,
            ),
        ];
        let mut observations = vec![];
        for (form, name, has_witness, has_scope) in forms {
            let digest = fixture.ty(form);
            let mut records = vec![];
            append_type(&fixture.view(), "root", digest, &mut records, 0)
                .expect("exact symbolic type observation");
            let mut expected = vec![
                ("path".to_owned(), "root".to_owned()),
                ("digest".to_owned(), digest.to_string()),
                ("form".to_owned(), name.to_owned()),
            ];
            if has_witness {
                expected.extend([
                    (
                        "scope".to_owned(),
                        format!("{}/{}", function.package, function.declaration),
                    ),
                    ("parameter".to_owned(), witness.to_string()),
                ]);
            }
            expected.extend([
                (
                    "contract".to_owned(),
                    format!("{}/{}", contract.package, contract.declaration),
                ),
                (
                    "member".to_owned(),
                    if has_scope { family } else { member }.to_string(),
                ),
            ]);
            if has_scope {
                expected.push((
                    "read-scope".to_owned(),
                    format!(
                        "lexical:{}/{}:{binding}",
                        function.package, function.declaration
                    ),
                ));
            }
            assert_eq!(
                records,
                vec![DiscoveryRecord {
                    operation: "type".to_owned(),
                    fields: expected
                }]
            );
            observations.push(records);
        }
        let other = fixture.ty(TypeForm::AssociatedReadViewProjection {
            scope: function,
            parameter: other_witness,
            contract,
            member: family,
            read_scope: lexical,
        });
        let mut records = vec![];
        append_type(&fixture.view(), "root", other, &mut records, 0).expect("other exact witness");
        assert_eq!(field(&records[0], "parameter"), other_witness.to_string());
        assert_ne!(
            records, observations[3],
            "equal result forms do not erase witness identity"
        );
        assert!(!records[0].fields.iter().any(|(name, _)| matches!(
            name.as_str(),
            "invocation" | "address" | "lease" | "runtime"
        )));
    }

    #[test]
    fn interface_discovery_inline_scope_inventory_shares_the_complete_record_bound() {
        let fixture = InterfaceFixture::new();
        let reference = fixture.reference(10);
        let scopes = [scope_parameter(8, "first"), scope_parameter(9, "second")];
        let existing = DiscoveryRecord::new("owner", [("id", "prior-owner".to_owned())]);
        let mut exact = vec![existing.clone(); 9_998];
        append_scope_parameters(
            "parent",
            ReadScopeBinder::Declaration(reference),
            &scopes,
            &mut exact,
        )
        .expect("exact record capacity");
        assert_eq!(exact.len(), 10_000);
        let mut short = vec![existing; 9_999];
        let error = append_scope_parameters(
            "parent",
            ReadScopeBinder::Declaration(reference),
            &scopes,
            &mut short,
        )
        .expect_err("scope-only overflow is bounded");
        assert_eq!(
            (error.class, error.code.as_str()),
            (DiagnosticClass::Resource, "builtin_type_projection_limit")
        );
        assert_eq!(short.len(), 10_000, "refused record was not appended");
    }

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
            if !public_interface_owner(&standard.into(), &value.record) {
                assert_eq!(
                    inspect_builtin_owner(standard, value.kind(), *owner)
                        .expect_err("private layout is not a public owner")
                        .code,
                    "builtin_owner_missing"
                );
                continue;
            }
            let detail = inspect_builtin_owner(standard, value.kind(), *owner)
                .expect("exact interface owner detail");
            assert!(!detail.is_empty());
            assert!(detail.len() < 10_000);
            assert!(observed.insert(*owner));
        }
        assert_eq!(
            observed.len(),
            query_builtin_owners(
                standard,
                &BuiltinOwnerSelector::default(),
                10_000,
                4 * 1024 * 1024,
                None
            )
            .expect("all public owners")
            .matched
        );
    }

    #[test]
    fn private_layouts_and_their_fields_are_hidden_while_public_owned_records_are_descriptive() {
        let mut standard = BuiltinStandard::load().expect("built-in standard").clone();
        let (owner, mut declaration) = standard
            .interface_owners
            .iter()
            .find_map(|(owner, value)| match &value.record {
                PackageInterfaceRecord::Declaration(declaration)
                    if matches!(&declaration.payload, PackageInterfaceDeclarationPayload::Record { fields, .. } if !fields.is_empty()) =>
                {
                    Some((*owner, declaration.clone()))
                }
                _ => None,
            })
            .expect("public record with fields");
        let PackageInterfaceDeclarationPayload::Record {
            fields,
            type_parameters,
        } = declaration.payload
        else {
            unreachable!();
        };
        let field = OwnerKey::Field(fields[0]);
        declaration.header.kind = OwnerKind::OwnedRecord;
        declaration.payload = PackageInterfaceDeclarationPayload::OwnedRecord {
            fields,
            type_parameters,
        };
        standard.interface_owners.get_mut(&owner).unwrap().record =
            PackageInterfaceRecord::RepresentationDeclaration(declaration.clone());
        let selector = BuiltinOwnerSelector {
            parent: Some(owner),
            ..BuiltinOwnerSelector::default()
        };
        assert_eq!(
            query_builtin_owners(&standard, &selector, 100, 64 * 1024, None)
                .unwrap()
                .matched,
            0
        );
        for (kind, key) in [(OwnerKind::OwnedRecord, owner), (OwnerKind::Field, field)] {
            assert_eq!(
                inspect_builtin_owner(&standard, kind, key)
                    .unwrap_err()
                    .code,
                "builtin_owner_missing"
            );
        }
        standard.interface_owners.get_mut(&owner).unwrap().record =
            PackageInterfaceRecord::Declaration(declaration);
        let field_selector = BuiltinOwnerSelector {
            kind: Some(OwnerKind::Field),
            parent: Some(owner),
            ..BuiltinOwnerSelector::default()
        };
        assert_eq!(
            query_builtin_owners(&standard, &field_selector, 100, 64 * 1024, None)
                .unwrap()
                .matched,
            0
        );
        let named_field_selector = BuiltinOwnerSelector {
            name: Some(owner_name(&standard.interface_owners[&field].record).to_owned()),
            ..field_selector
        };
        assert_eq!(
            query_builtin_owners(&standard, &named_field_selector, 100, 64 * 1024, None)
                .unwrap()
                .matched,
            0
        );
        assert_eq!(
            inspect_builtin_owner(&standard, OwnerKind::Field, field)
                .unwrap_err()
                .code,
            "builtin_owner_missing"
        );
        assert_eq!(
            parse_interface_owner_kind("owned_record").unwrap(),
            OwnerKind::OwnedRecord
        );
        let detail = inspect_builtin_owner(&standard, OwnerKind::OwnedRecord, owner).unwrap();
        assert!(detail.iter().any(|record| {
            record.operation == "declaration"
                && record
                    .fields
                    .iter()
                    .any(|(name, value)| name == "kind" && value == "owned-record")
        }));
        assert!(detail.iter().any(|record| {
            record.operation == "owner"
                && record
                    .fields
                    .iter()
                    .any(|(name, value)| name == "id" && value == &field.to_string())
        }));
        assert_eq!(
            inspect_builtin_owner(&standard, OwnerKind::Record, owner)
                .unwrap_err()
                .code,
            "builtin_owner_missing"
        );
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
            &PackageInterfaceView::from(&stale),
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
            &PackageInterfaceView::from(&foreign),
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
