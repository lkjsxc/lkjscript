//! Read-only inputs, not document layout or page-specific reference selection.
use super::{CapabilitiesSnapshot, ProjectTemplate, RegistrySection, Value, json};
use crate::platform::builtin_discovery::inspect_builtin_owner;
use crate::platform::builtin_standard::BuiltinStandard;
use crate::platform::kernel::{OwnerKey, PackageInterfaceRecord};

pub(super) fn section(
    snapshot: &CapabilitiesSnapshot,
    section: RegistrySection,
) -> Result<&str, String> {
    let value = snapshot
        .section(section)
        .ok_or_else(|| format!("missing capability section '{}'", section.name()))?;
    std::str::from_utf8(&value.bytes)
        .map_err(|_| format!("capability section '{}' is not UTF-8", section.name()))
}

pub(super) fn template(item: ProjectTemplate) -> Value {
    json!({
        "name": item.name(),
        "purpose": item.purpose(),
        "runner": item.runner(),
        "deployment": item.emits_deployment(),
        "artifact": item.recommended_artifact_output().unwrap_or("none"),
    })
}

pub(super) fn standard() -> Result<Value, String> {
    let standard = BuiltinStandard::load().map_err(|error| error.to_string())?;
    let mut records = String::new();
    for (owner, value) in &standard.interface_owners {
        if !matches!(value.record, PackageInterfaceRecord::Declaration(_)) {
            continue;
        }
        for record in inspect_builtin_owner(standard, value.record.header().kind, *owner)
            .map_err(|error| error.to_string())?
        {
            records.push_str(&record.render().map_err(|error| error.to_string())?);
        }
    }
    Ok(json!({"package": standard.package.to_string(),
        "revision": standard.package_revision.to_string(), "records": records}))
}

pub(super) fn owners() -> Result<Value, String> {
    let standard = BuiltinStandard::load().map_err(|error| error.to_string())?;
    Ok(Value::Array(
        standard
            .interface_owners
            .iter()
            .map(|(owner, value)| {
                json!({
                    "kind": value.record.header().kind.name(),
                    "name": package_owner_name(&value.record),
                    "parent": package_owner_parent(&value.record)
                        .map(|parent| format!("{}/{parent}", standard.package)).unwrap_or_default(),
                    "reference": format!("{}/{owner}", standard.package),
                })
            })
            .collect(),
    ))
}

pub(super) fn limits() -> Value {
    let limits = crate::platform::http_client::HttpClientLimits::default();
    json!([
        {"name": "request-headers", "maximum": limits.maximum_request_headers},
        {"name": "request-header-bytes", "maximum": limits.maximum_request_header_bytes},
        {"name": "response-headers", "maximum": limits.maximum_response_headers},
        {"name": "response-header-bytes", "maximum": limits.maximum_response_header_bytes},
        {"name": "response-body-bytes", "maximum": limits.maximum_response_body_bytes},
        {"name": "dns-results", "maximum": limits.maximum_dns_results},
        {"name": "concurrent-requests", "maximum": limits.maximum_concurrent_requests},
        {"name": "connection-milliseconds", "maximum": limits.connection_timeout_milliseconds},
        {"name": "total-milliseconds", "maximum": limits.total_timeout_milliseconds},
        {"name": "cleanup-milliseconds", "maximum": limits.cleanup_timeout_milliseconds},
    ])
}

fn package_owner_name(record: &PackageInterfaceRecord) -> &str {
    match record {
        PackageInterfaceRecord::Declaration(value) => value.name.as_str(),
        PackageInterfaceRecord::TypeParameter(value) => value.name.as_str(),
        PackageInterfaceRecord::RequirementParameter(value) => value.name.as_str(),
        PackageInterfaceRecord::EffectParameter(value) => value.name.as_str(),
        PackageInterfaceRecord::Field(value) => value.name.as_str(),
        PackageInterfaceRecord::Case(value) => value.name.as_str(),
        PackageInterfaceRecord::Operation(value) => value.name.as_str(),
        PackageInterfaceRecord::Parameter(value) => value.name.as_str(),
        PackageInterfaceRecord::Requirement(value) => value.name.as_str(),
        PackageInterfaceRecord::Port(value) => value.name.as_str(),
    }
}

fn package_owner_parent(record: &PackageInterfaceRecord) -> Option<OwnerKey> {
    match record {
        PackageInterfaceRecord::Declaration(_) => None,
        PackageInterfaceRecord::RequirementParameter(value) => {
            Some(OwnerKey::Declaration(value.declaration))
        }
        PackageInterfaceRecord::EffectParameter(value) => {
            Some(OwnerKey::Declaration(value.declaration))
        }
        PackageInterfaceRecord::TypeParameter(value) => {
            Some(OwnerKey::Declaration(value.declaration))
        }
        PackageInterfaceRecord::Field(value) => Some(OwnerKey::Declaration(value.declaration)),
        PackageInterfaceRecord::Case(value) => Some(OwnerKey::Declaration(value.declaration)),
        PackageInterfaceRecord::Operation(value) => Some(OwnerKey::Declaration(value.declaration)),
        PackageInterfaceRecord::Parameter(value) => Some(match value.parent {
            crate::platform::kernel::ParameterParent::Function(declaration) => {
                OwnerKey::Declaration(declaration)
            }
            crate::platform::kernel::ParameterParent::Operation(operation) => {
                OwnerKey::Operation(operation)
            }
        }),
        PackageInterfaceRecord::Requirement(value) => {
            Some(OwnerKey::Declaration(value.declaration))
        }
        PackageInterfaceRecord::Port(value) => Some(OwnerKey::Declaration(value.declaration)),
    }
}
