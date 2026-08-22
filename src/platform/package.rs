//! Exact semantic package metadata plus the bounded predecessor-import descriptor model.

use super::diagnostic::{Diagnostic, DiagnosticClass};
use bincode::de::Decoder;
use bincode::enc::Encoder;
use bincode::error::{DecodeError, EncodeError};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
#[cfg(test)]
use std::collections::BTreeSet;

pub const PACKAGE_CONTRACT_VERSION: u16 = 1;
#[cfg(test)]
pub const MAXIMUM_PACKAGE_BYTES: usize = 1_048_576;
#[cfg(test)]
pub const MAXIMUM_MODULES: usize = 4_096;
#[cfg(test)]
pub const MAXIMUM_DEPENDENCIES: usize = 1_024;
#[cfg(test)]
pub const MAXIMUM_TARGETS: usize = 1_024;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct PackageId(String);

impl PackageId {
    pub fn parse(value: &str) -> Result<Self, Diagnostic> {
        validate_hex(value, 32, "package_id")?;
        if value.bytes().all(|byte| byte == b'0') {
            return Err(package_error(
                "package_id_zero",
                "package identity may not be all zeroes",
            ));
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn bytes(&self) -> [u8; 16] {
        let mut bytes = [0_u8; 16];
        for (index, pair) in self.0.as_bytes().chunks_exact(2).enumerate() {
            let high = decode_hex(pair[0]);
            let low = decode_hex(pair[1]);
            bytes[index] = (high << 4) | low;
        }
        bytes
    }
}

impl Encode for PackageId {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        17u8.encode(encoder)?;
        self.bytes().encode(encoder)
    }
}

impl<Context> Decode<Context> for PackageId {
    fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
        let tag = u8::decode(decoder)?;
        if tag != 17 {
            return Err(DecodeError::OtherString(format!(
                "foreign package identity domain tag {tag}"
            )));
        }
        let bytes = <[u8; 16]>::decode(decoder)?;
        let encoded = super::semantic_id::encode_hex(&bytes);
        Self::parse(&encoded).map_err(|error| DecodeError::OtherString(error.message))
    }
}

bincode::impl_borrow_decode!(PackageId);

impl<'de> Deserialize<'de> for PackageId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PackageDescriptor {
    pub contract_version: u16,
    pub package_id: PackageId,
    pub name: String,
    pub modules: Vec<ModuleLocator>,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
    #[serde(default)]
    pub targets: Vec<Target>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleLocator {
    pub name: String,
    pub path: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    pub alias: String,
    pub package_id: PackageId,
    pub revision_digest: String,
    pub artifact_digest: String,
    /// Workspace/deployment locator. This field is excluded from semantic dependency identity.
    pub artifact: String,
}

#[derive(
    Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(rename_all = "snake_case")]
pub enum RunnerKind {
    Command,
    Http,
    Interactive,
    Batch,
    Worker,
    Test,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub name: String,
    pub component: String,
    pub port: String,
    pub runner: RunnerKind,
}

#[cfg(test)]
pub(crate) fn decode_package(bytes: &[u8]) -> Result<PackageDescriptor, Diagnostic> {
    if bytes.len() > MAXIMUM_PACKAGE_BYTES {
        return Err(package_error(
            "package_too_large",
            format!(
                "package descriptor has {} bytes; the limit is {MAXIMUM_PACKAGE_BYTES}",
                bytes.len()
            ),
        ));
    }
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let descriptor = PackageDescriptor::deserialize(&mut deserializer).map_err(|error| {
        package_error(
            "package_json",
            format!("package descriptor is not strict JSON: {error}"),
        )
    })?;
    deserializer.end().map_err(|error| {
        package_error(
            "package_trailing_json",
            format!("package descriptor has trailing input: {error}"),
        )
    })?;
    validate_package(&descriptor)?;
    Ok(descriptor)
}

#[cfg(test)]
fn canonical_package_bytes(descriptor: &PackageDescriptor) -> Result<Vec<u8>, Diagnostic> {
    let mut bytes = serde_json::to_vec(descriptor).map_err(|error| {
        package_error(
            "package_encode",
            format!("package descriptor encoding failed: {error}"),
        )
    })?;
    bytes.push(b'\n');
    Ok(bytes)
}

pub fn semantic_dependency_bytes(descriptor: &PackageDescriptor) -> Result<Vec<u8>, Diagnostic> {
    #[derive(Serialize)]
    struct SemanticPackage<'a> {
        contract_version: u16,
        package_id: &'a PackageId,
        name: &'a str,
        modules: Vec<&'a str>,
        dependencies: Vec<SemanticDependency<'a>>,
        targets: Vec<&'a Target>,
    }
    #[derive(Serialize)]
    struct SemanticDependency<'a> {
        alias: &'a str,
        package_id: &'a PackageId,
        revision_digest: &'a str,
        artifact_digest: &'a str,
    }
    let mut modules: Vec<_> = descriptor
        .modules
        .iter()
        .map(|module| module.name.as_str())
        .collect();
    modules.sort_unstable();
    let mut dependencies: Vec<_> = descriptor
        .dependencies
        .iter()
        .map(|dependency| SemanticDependency {
            alias: &dependency.alias,
            package_id: &dependency.package_id,
            revision_digest: &dependency.revision_digest,
            artifact_digest: &dependency.artifact_digest,
        })
        .collect();
    dependencies.sort_by(|left, right| left.alias.cmp(right.alias));
    let mut targets: Vec<_> = descriptor.targets.iter().collect();
    targets.sort_by(|left, right| left.name.cmp(&right.name));
    let semantic = SemanticPackage {
        contract_version: descriptor.contract_version,
        package_id: &descriptor.package_id,
        name: &descriptor.name,
        modules,
        dependencies,
        targets,
    };
    serde_json::to_vec(&semantic).map_err(|error| {
        package_error(
            "package_semantic_encode",
            format!("semantic package encoding failed: {error}"),
        )
    })
}

#[cfg(test)]
fn validate_package(descriptor: &PackageDescriptor) -> Result<(), Diagnostic> {
    if descriptor.contract_version != PACKAGE_CONTRACT_VERSION {
        return Err(package_error(
            "package_contract",
            format!(
                "package contract {} is not current contract {PACKAGE_CONTRACT_VERSION}",
                descriptor.contract_version
            ),
        ));
    }
    validate_name(&descriptor.name, "package name", true)?;
    if descriptor.modules.is_empty() {
        return Err(package_error(
            "package_without_modules",
            "package must declare at least one module",
        ));
    }
    if descriptor.modules.len() > MAXIMUM_MODULES {
        return Err(package_error(
            "package_module_count",
            format!("package declares more than {MAXIMUM_MODULES} modules"),
        ));
    }
    if descriptor.dependencies.len() > MAXIMUM_DEPENDENCIES {
        return Err(package_error(
            "package_dependency_count",
            format!("package declares more than {MAXIMUM_DEPENDENCIES} dependencies"),
        ));
    }
    if descriptor.targets.len() > MAXIMUM_TARGETS {
        return Err(package_error(
            "package_target_count",
            format!("package declares more than {MAXIMUM_TARGETS} targets"),
        ));
    }

    let mut module_names = BTreeSet::new();
    let mut module_paths = BTreeSet::new();
    for module in &descriptor.modules {
        validate_name(&module.name, "module name", true)?;
        validate_relative_path(&module.path, "module path", true)?;
        if !module.path.ends_with(".lkj") {
            return Err(package_error(
                "package_module_extension",
                format!("module path '{}' must end in '.lkj'", module.path),
            ));
        }
        if !module_names.insert(&module.name) {
            return Err(package_error(
                "package_module_name_duplicate",
                format!("duplicate module name '{}'", module.name),
            ));
        }
        if !module_paths.insert(&module.path) {
            return Err(package_error(
                "package_module_path_duplicate",
                format!("duplicate module path '{}'", module.path),
            ));
        }
    }

    let mut dependency_aliases = BTreeSet::new();
    for dependency in &descriptor.dependencies {
        validate_name(&dependency.alias, "dependency alias", false)?;
        validate_hex(
            &dependency.revision_digest,
            64,
            "dependency revision digest",
        )?;
        validate_hex(
            &dependency.artifact_digest,
            64,
            "dependency artifact digest",
        )?;
        validate_relative_path(&dependency.artifact, "dependency artifact locator", false)?;
        if !dependency_aliases.insert(&dependency.alias) {
            return Err(package_error(
                "package_dependency_duplicate",
                format!("duplicate dependency alias '{}'", dependency.alias),
            ));
        }
    }

    let mut targets = BTreeSet::new();
    for target in &descriptor.targets {
        validate_name(&target.name, "target name", false)?;
        validate_qualified_owner(&target.component, "target component")?;
        validate_name(&target.port, "target port", false)?;
        if !targets.insert(&target.name) {
            return Err(package_error(
                "package_target_duplicate",
                format!("duplicate target name '{}'", target.name),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
fn validate_name(value: &str, label: &str, qualified: bool) -> Result<(), Diagnostic> {
    if value.is_empty() || value.len() > 128 {
        return Err(package_error(
            "package_name",
            format!("{label} must contain 1 through 128 bytes"),
        ));
    }
    let segments: Vec<_> = value.split('.').collect();
    if !qualified && segments.len() != 1 {
        return Err(package_error(
            "package_name",
            format!("{label} may not contain '.'"),
        ));
    }
    for segment in segments {
        let mut bytes = segment.bytes();
        let Some(first) = bytes.next() else {
            return Err(package_error(
                "package_name",
                format!("{label} contains an empty segment"),
            ));
        };
        if !first.is_ascii_lowercase() {
            return Err(package_error(
                "package_name",
                format!("{label} segments must start with a lowercase ASCII letter"),
            ));
        }
        if !bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-') {
            return Err(package_error(
                "package_name",
                format!("{label} contains a character outside [a-z0-9-]"),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
fn validate_qualified_owner(value: &str, label: &str) -> Result<(), Diagnostic> {
    if value.len() > 257 {
        return Err(package_error(
            "package_owner",
            format!("{label} exceeds 257 bytes"),
        ));
    }
    let Some((module, declaration)) = value.rsplit_once('.') else {
        return Err(package_error(
            "package_owner",
            format!("{label} must be 'module.Declaration'"),
        ));
    };
    validate_name(module, label, true)?;
    let first = declaration.as_bytes().first().copied();
    if first.is_none_or(|byte| !byte.is_ascii_uppercase())
        || !declaration
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err(package_error(
            "package_owner",
            format!("{label} declaration must start uppercase and use [A-Za-z0-9_-]"),
        ));
    }
    Ok(())
}

#[cfg(test)]
fn validate_relative_path(
    value: &str,
    label: &str,
    require_source_root: bool,
) -> Result<(), Diagnostic> {
    if value.is_empty() || value.len() > 512 {
        return Err(package_error(
            "package_path",
            format!("{label} must contain 1 through 512 bytes"),
        ));
    }
    if value.starts_with('/') || value.contains('\\') || value.contains('\0') {
        return Err(package_error(
            "package_path",
            format!("{label} is not a canonical relative slash path"),
        ));
    }
    let components: Vec<_> = value.split('/').collect();
    if components
        .iter()
        .any(|component| component.is_empty() || matches!(*component, "." | ".."))
    {
        return Err(package_error(
            "package_path",
            format!("{label} contains an empty, '.' or '..' component"),
        ));
    }
    if require_source_root && components.first() != Some(&"src") {
        return Err(package_error(
            "package_path",
            format!("{label} must be beneath the explicit 'src/' root"),
        ));
    }
    Ok(())
}

fn validate_hex(value: &str, length: usize, label: &str) -> Result<(), Diagnostic> {
    if value.len() != length
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(package_error(
            "package_hex",
            format!("{label} must be exactly {length} lowercase hexadecimal characters"),
        ));
    }
    Ok(())
}

fn decode_hex(value: u8) -> u8 {
    match value {
        b'0'..=b'9' => value - b'0',
        b'a'..=b'f' => value - b'a' + 10,
        _ => 0,
    }
}

fn package_error(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Source, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PACKAGE: &[u8] = br#"{
  "contract_version": 1,
  "package_id": "1234567890abcdef1234567890abcdef",
  "name": "resource-service",
  "modules": [
    {"name": "domain", "path": "src/domain.lkj"},
    {"name": "service.http", "path": "src/service/http.lkj"}
  ],
  "dependencies": [{
    "alias": "standard",
    "package_id": "abcdef1234567890abcdef1234567890",
    "revision_digest": "1111111111111111111111111111111111111111111111111111111111111111",
    "artifact_digest": "2222222222222222222222222222222222222222222222222222222222222222",
    "artifact": "dependencies/standard.lkpackage"
  }],
  "targets": [{"name": "serve", "component": "service.http.Web", "port": "service", "runner": "http"}]
}"#;

    #[test]
    fn exact_dependency_and_locator_are_distinct() {
        let descriptor = decode_package(PACKAGE).expect("valid package");
        let first = semantic_dependency_bytes(&descriptor).expect("semantic encoding");
        let mut relocated = descriptor.clone();
        relocated.dependencies[0].artifact = "vendor/standard.lkpackage".to_owned();
        relocated.modules[0].path = "src/moved/domain.lkj".to_owned();
        let second = semantic_dependency_bytes(&relocated).expect("relocated encoding");
        assert_eq!(first, second);
        assert_ne!(
            canonical_package_bytes(&descriptor).expect("canonical descriptor"),
            canonical_package_bytes(&relocated).expect("canonical relocated descriptor")
        );
    }

    #[test]
    fn mutable_coordinates_and_path_escape_are_not_representable() {
        let mut value: serde_json::Value = serde_json::from_slice(PACKAGE).expect("fixture json");
        value["dependencies"][0]["revision"] = serde_json::Value::String("latest".to_owned());
        let bytes = serde_json::to_vec(&value).expect("json");
        let error = decode_package(&bytes).expect_err("unknown mutable field rejects");
        assert_eq!(error.code, "package_json");

        value["dependencies"][0]
            .as_object_mut()
            .expect("dependency object")
            .remove("revision");
        value["modules"][0]["path"] = serde_json::Value::String("src/../escape.lkj".to_owned());
        let bytes = serde_json::to_vec(&value).expect("json");
        let error = decode_package(&bytes).expect_err("traversal rejects");
        assert_eq!(error.code, "package_path");
    }

    #[test]
    fn current_contract_rejects_a_predecessor_exactly() {
        let mut value: serde_json::Value = serde_json::from_slice(PACKAGE).expect("fixture json");
        value["contract_version"] = serde_json::Value::from(0);
        let bytes = serde_json::to_vec(&value).expect("json");
        let error = decode_package(&bytes).expect_err("predecessor rejects");
        assert_eq!(error.code, "package_contract");
    }
}
