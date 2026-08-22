//! Canonical repository/package root for packed meaning-module shards.

use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::meaning::{GRAPH_CONTRACT_VERSION, MeaningModule};
use super::package::{PackageId, RunnerKind};
use super::packed;
use super::semantic_digest::{ArtifactDigest, ModuleObjectDigest, RootObjectDigest};
use super::semantic_id::{
    AnnotationId, BindingId, CaseId, DeclarationId, DocumentationId, ExpressionId, FieldId,
    ModuleId, OperationId, ParameterId, PortId, RepositoryId, RequirementId, RevisionId, TargetId,
};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MAXIMUM_ROOT_BYTES: usize = 16 * 1_048_576;
pub const MAXIMUM_ROOT_MODULES: usize = 100_000;
pub const MAXIMUM_ROOT_DEPENDENCIES: usize = 4_096;
pub const MAXIMUM_ROOT_TARGETS: usize = 65_536;
pub const MAXIMUM_TOMBSTONES: usize = 2_000_000;
const ROOT_MAGIC: [u8; 8] = *b"LKJROOT1";
const ROOT_DIGEST_DOMAIN: &str = "lkjscript.semantic-root-object.v1";

#[derive(Decode, Encode, Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleObjectRef {
    pub id: ModuleId,
    pub name: String,
    pub object: ModuleObjectDigest,
}

#[derive(Decode, Encode, Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyBinding {
    pub alias: String,
    pub package_id: PackageId,
    pub semantic_revision: RevisionId,
    pub artifact: ArtifactDigest,
}

#[derive(Decode, Encode, Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetBinding {
    pub id: TargetId,
    pub name: String,
    pub component_module: ModuleId,
    pub component_module_name: String,
    pub component: DeclarationId,
    pub component_name: String,
    pub port: PortId,
    pub port_name: String,
    pub runner: RunnerKind,
}

#[derive(Decode, Encode, Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum TombstoneIdentity {
    Module(ModuleId),
    Declaration(DeclarationId),
    Field(FieldId),
    Case(CaseId),
    Operation(OperationId),
    Parameter(ParameterId),
    Binding(BindingId),
    Expression(ExpressionId),
    Requirement(RequirementId),
    Port(PortId),
    Target(TargetId),
    Documentation(DocumentationId),
    Annotation(AnnotationId),
}

#[derive(Decode, Encode, Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Tombstone {
    pub identity: TombstoneIdentity,
    /// Exact revision in which the identity was last live. The deleting revision cannot be
    /// embedded here because revision identity commits to the root containing this record.
    pub last_live_revision: RevisionId,
    pub last_name: String,
}

#[derive(Decode, Encode, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GraphRoot {
    pub graph_contract_version: u16,
    pub repository_id: RepositoryId,
    pub package_id: PackageId,
    pub package_name: String,
    pub modules: Vec<ModuleObjectRef>,
    pub dependencies: Vec<DependencyBinding>,
    pub targets: Vec<TargetBinding>,
    pub tombstones: Vec<Tombstone>,
}

impl GraphRoot {
    pub fn encode(&self) -> Result<Vec<u8>, Diagnostic> {
        self.validate_shape()?;
        packed::encode(ROOT_MAGIC, ROOT_DIGEST_DOMAIN, self, MAXIMUM_ROOT_BYTES)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, Diagnostic> {
        let value: Self =
            packed::decode(bytes, ROOT_MAGIC, ROOT_DIGEST_DOMAIN, MAXIMUM_ROOT_BYTES)?;
        value.validate_shape()?;
        Ok(value)
    }

    pub fn digest(&self) -> Result<RootObjectDigest, Diagnostic> {
        Ok(RootObjectDigest::of(&self.encode()?))
    }

    pub fn validate_shape(&self) -> Result<(), Diagnostic> {
        if self.graph_contract_version != GRAPH_CONTRACT_VERSION {
            return Err(graph_error(
                DiagnosticClass::Source,
                "graph_root_contract",
                format!(
                    "graph root contract {} is not current contract {GRAPH_CONTRACT_VERSION}",
                    self.graph_contract_version
                ),
            ));
        }
        validate_name(&self.package_name, "package name", true)?;
        if self.modules.is_empty() || self.modules.len() > MAXIMUM_ROOT_MODULES {
            return Err(graph_error(
                DiagnosticClass::Resource,
                "graph_root_module_count",
                format!("graph root must contain 1 through {MAXIMUM_ROOT_MODULES} modules"),
            ));
        }
        if self.dependencies.len() > MAXIMUM_ROOT_DEPENDENCIES
            || self.targets.len() > MAXIMUM_ROOT_TARGETS
            || self.tombstones.len() > MAXIMUM_TOMBSTONES
        {
            return Err(graph_error(
                DiagnosticClass::Resource,
                "graph_root_item_count",
                "graph root exceeds a canonical collection bound",
            ));
        }
        require_sorted_unique(&self.modules, "graph_root_module_order")?;
        require_sorted_unique(&self.dependencies, "graph_root_dependency_order")?;
        require_sorted_unique(&self.targets, "graph_root_target_order")?;
        require_sorted_unique(&self.tombstones, "graph_root_tombstone_order")?;

        let mut module_names = BTreeSet::new();
        let mut module_ids = BTreeSet::new();
        for module in &self.modules {
            validate_name(&module.name, "module name", true)?;
            if !module_names.insert(&module.name) || !module_ids.insert(module.id) {
                return Err(graph_error(
                    DiagnosticClass::Corrupt,
                    "graph_root_module_duplicate",
                    "module name or identity is duplicated",
                ));
            }
        }
        let mut aliases = BTreeSet::new();
        for dependency in &self.dependencies {
            validate_name(&dependency.alias, "dependency alias", false)?;
            if !aliases.insert(&dependency.alias) {
                return Err(graph_error(
                    DiagnosticClass::Corrupt,
                    "graph_root_dependency_duplicate",
                    "dependency alias is duplicated",
                ));
            }
        }
        let mut target_names = BTreeSet::new();
        let mut target_ids = BTreeSet::new();
        for target in &self.targets {
            validate_name(&target.name, "target name", false)?;
            validate_name(&target.component_module_name, "component module name", true)?;
            validate_declaration_name(&target.component_name, "component name")?;
            validate_name(&target.port_name, "port name", false)?;
            if !target_names.insert(&target.name) || !target_ids.insert(target.id) {
                return Err(graph_error(
                    DiagnosticClass::Corrupt,
                    "graph_root_target_duplicate",
                    "target name or identity is duplicated",
                ));
            }
        }
        let live = live_identity_domains(self);
        let mut deleted = BTreeSet::new();
        for tombstone in &self.tombstones {
            let domain = tombstone_domain_bytes(&tombstone.identity);
            if !deleted.insert(domain) {
                return Err(graph_error(
                    DiagnosticClass::Corrupt,
                    "graph_root_tombstone_duplicate",
                    "a deleted semantic identity has more than one tombstone",
                ));
            }
            if live.contains(&domain) {
                return Err(graph_error(
                    DiagnosticClass::Corrupt,
                    "graph_root_live_tombstone",
                    "a live identity is also retained as deleted",
                ));
            }
        }
        Ok(())
    }

    pub fn validate_modules(&self, modules: &[MeaningModule]) -> Result<(), Diagnostic> {
        if modules.len() != self.modules.len() {
            return Err(graph_error(
                DiagnosticClass::Corrupt,
                "graph_root_module_object_count",
                "loaded module count does not match the root",
            ));
        }
        let by_id = modules
            .iter()
            .map(|module| (module.module_id, module))
            .collect::<BTreeMap<_, _>>();
        if by_id.len() != modules.len() {
            return Err(graph_error(
                DiagnosticClass::Corrupt,
                "graph_module_identity_duplicate",
                "two module objects claim the same identity",
            ));
        }
        let mut global_identities = live_identity_domains(self);
        for reference in &self.modules {
            let module = by_id.get(&reference.id).ok_or_else(|| {
                graph_error(
                    DiagnosticClass::Corrupt,
                    "graph_module_identity_missing",
                    format!("root module '{}' has no matching object", reference.name),
                )
            })?;
            if module.module.name != reference.name || module.digest()? != reference.object {
                return Err(graph_error(
                    DiagnosticClass::Corrupt,
                    "graph_module_object_mismatch",
                    format!(
                        "module object '{}' does not match its root reference",
                        reference.name
                    ),
                ));
            }
            for declaration in &module.declarations {
                if !global_identities.insert(("declaration", declaration.id.bytes())) {
                    return Err(graph_error(
                        DiagnosticClass::Corrupt,
                        "graph_declaration_identity_duplicate",
                        "declaration identity is duplicated across modules",
                    ));
                }
                for member in &declaration.members {
                    let domain = match member {
                        super::meaning::MemberIdentity::Field { id, .. } => ("field", id.bytes()),
                        super::meaning::MemberIdentity::Case { id, .. } => ("case", id.bytes()),
                        super::meaning::MemberIdentity::Operation { id, .. } => {
                            ("operation", id.bytes())
                        }
                        super::meaning::MemberIdentity::Parameter { id, .. } => {
                            ("parameter", id.bytes())
                        }
                        super::meaning::MemberIdentity::TaskRequirement { id, .. }
                        | super::meaning::MemberIdentity::ComponentRequirement { id, .. } => {
                            ("requirement", id.bytes())
                        }
                        super::meaning::MemberIdentity::Port { id, .. } => ("port", id.bytes()),
                    };
                    if !global_identities.insert(domain) {
                        return Err(graph_error(
                            DiagnosticClass::Corrupt,
                            "graph_member_identity_duplicate",
                            "member identity is duplicated across modules",
                        ));
                    }
                }
                for binding in &declaration.bindings {
                    if !global_identities.insert(("binding", binding.id.bytes())) {
                        return Err(graph_error(
                            DiagnosticClass::Corrupt,
                            "graph_binding_identity_duplicate",
                            "binding identity is duplicated across modules",
                        ));
                    }
                }
                for expression in &declaration.expressions {
                    if !global_identities.insert(("expression", expression.id.bytes())) {
                        return Err(graph_error(
                            DiagnosticClass::Corrupt,
                            "graph_expression_identity_duplicate",
                            "expression identity is duplicated across modules",
                        ));
                    }
                }
            }
            for documentation in &module.documentation {
                if !global_identities.insert(("documentation", documentation.id.bytes())) {
                    return Err(graph_error(
                        DiagnosticClass::Corrupt,
                        "graph_documentation_identity_duplicate",
                        "documentation identity is duplicated across modules",
                    ));
                }
            }
            for annotation in &module.annotations {
                if !global_identities.insert(("annotation", annotation.id.bytes())) {
                    return Err(graph_error(
                        DiagnosticClass::Corrupt,
                        "graph_annotation_identity_duplicate",
                        "annotation identity is duplicated across modules",
                    ));
                }
            }
        }
        for target in &self.targets {
            let Some(module) = modules
                .iter()
                .find(|module| module.module_id == target.component_module)
            else {
                return Err(graph_error(
                    DiagnosticClass::Corrupt,
                    "graph_target_component_missing",
                    format!(
                        "target '{}' references a missing component module",
                        target.name
                    ),
                ));
            };
            if module.module.name != target.component_module_name {
                return Err(graph_error(
                    DiagnosticClass::Corrupt,
                    "graph_target_component_module_name",
                    format!(
                        "target '{}' has a stale component module locator",
                        target.name
                    ),
                ));
            }
            let Some((_, component)) = module.declaration(target.component) else {
                return Err(graph_error(
                    DiagnosticClass::Corrupt,
                    "graph_target_component_missing",
                    format!("target '{}' references a missing component", target.name),
                ));
            };
            if component.name() != target.component_name {
                return Err(graph_error(
                    DiagnosticClass::Corrupt,
                    "graph_target_component_name",
                    format!("target '{}' has a stale component locator", target.name),
                ));
            }
            let port_matches = module.declarations.iter().any(|declaration| {
                declaration.id == target.component
                    && declaration.members.iter().any(|member| {
                        matches!(
                            member,
                            super::meaning::MemberIdentity::Port { id, name }
                                if *id == target.port && *name == target.port_name
                        )
                    })
            });
            if !port_matches {
                return Err(graph_error(
                    DiagnosticClass::Corrupt,
                    "graph_target_port_missing",
                    format!(
                        "target '{}' references a missing component port",
                        target.name
                    ),
                ));
            }
        }
        let live = live_module_identity_domains(self, modules);
        for tombstone in &self.tombstones {
            if live.contains(&tombstone_domain_bytes(&tombstone.identity)) {
                return Err(graph_error(
                    DiagnosticClass::Corrupt,
                    "graph_live_tombstone",
                    "a live semantic identity is also retained as deleted",
                ));
            }
        }
        Ok(())
    }
}

fn live_module_identity_domains(
    root: &GraphRoot,
    modules: &[MeaningModule],
) -> BTreeSet<(&'static str, [u8; 16])> {
    let mut live = live_identity_domains(root);
    for module in modules {
        for declaration in &module.declarations {
            live.insert(("declaration", declaration.id.bytes()));
            for member in &declaration.members {
                match member {
                    super::meaning::MemberIdentity::Field { id, .. } => {
                        live.insert(("field", id.bytes()));
                    }
                    super::meaning::MemberIdentity::Case { id, .. } => {
                        live.insert(("case", id.bytes()));
                    }
                    super::meaning::MemberIdentity::Operation { id, .. } => {
                        live.insert(("operation", id.bytes()));
                    }
                    super::meaning::MemberIdentity::Parameter { id, .. } => {
                        live.insert(("parameter", id.bytes()));
                    }
                    super::meaning::MemberIdentity::TaskRequirement { id, .. }
                    | super::meaning::MemberIdentity::ComponentRequirement { id, .. } => {
                        live.insert(("requirement", id.bytes()));
                    }
                    super::meaning::MemberIdentity::Port { id, .. } => {
                        live.insert(("port", id.bytes()));
                    }
                }
            }
            for binding in &declaration.bindings {
                live.insert(("binding", binding.id.bytes()));
            }
            for expression in &declaration.expressions {
                live.insert(("expression", expression.id.bytes()));
            }
        }
        for documentation in &module.documentation {
            live.insert(("documentation", documentation.id.bytes()));
        }
        for annotation in &module.annotations {
            live.insert(("annotation", annotation.id.bytes()));
        }
    }
    live
}

fn require_sorted_unique<T: Ord>(values: &[T], code: &str) -> Result<(), Diagnostic> {
    if values.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(graph_error(
            DiagnosticClass::Corrupt,
            code,
            "canonical collection is not strictly ordered",
        ));
    }
    Ok(())
}

fn live_identity_domains(root: &GraphRoot) -> BTreeSet<(&'static str, [u8; 16])> {
    let mut live = BTreeSet::new();
    for module in &root.modules {
        live.insert(("module", module.id.bytes()));
    }
    for target in &root.targets {
        live.insert(("target", target.id.bytes()));
    }
    live
}

fn tombstone_domain_bytes(identity: &TombstoneIdentity) -> (&'static str, [u8; 16]) {
    match identity {
        TombstoneIdentity::Module(id) => ("module", id.bytes()),
        TombstoneIdentity::Declaration(id) => ("declaration", id.bytes()),
        TombstoneIdentity::Field(id) => ("field", id.bytes()),
        TombstoneIdentity::Case(id) => ("case", id.bytes()),
        TombstoneIdentity::Operation(id) => ("operation", id.bytes()),
        TombstoneIdentity::Parameter(id) => ("parameter", id.bytes()),
        TombstoneIdentity::Binding(id) => ("binding", id.bytes()),
        TombstoneIdentity::Expression(id) => ("expression", id.bytes()),
        TombstoneIdentity::Requirement(id) => ("requirement", id.bytes()),
        TombstoneIdentity::Port(id) => ("port", id.bytes()),
        TombstoneIdentity::Target(id) => ("target", id.bytes()),
        TombstoneIdentity::Documentation(id) => ("documentation", id.bytes()),
        TombstoneIdentity::Annotation(id) => ("annotation", id.bytes()),
    }
}

fn validate_name(value: &str, label: &str, qualified: bool) -> Result<(), Diagnostic> {
    if value.is_empty() || value.len() > 128 {
        return Err(graph_error(
            DiagnosticClass::Semantic,
            "graph_name",
            format!("{label} must contain 1 through 128 bytes"),
        ));
    }
    let segments = value.split('.').collect::<Vec<_>>();
    if !qualified && segments.len() != 1 {
        return Err(graph_error(
            DiagnosticClass::Semantic,
            "graph_name",
            format!("{label} may not contain '.'"),
        ));
    }
    for segment in segments {
        let mut bytes = segment.bytes();
        if bytes.next().is_none_or(|byte| !byte.is_ascii_lowercase())
            || !bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            return Err(graph_error(
                DiagnosticClass::Semantic,
                "graph_name",
                format!("{label} is not a canonical lowercase semantic name"),
            ));
        }
    }
    Ok(())
}

fn validate_declaration_name(value: &str, label: &str) -> Result<(), Diagnostic> {
    if value.is_empty()
        || value.len() > 128
        || value
            .bytes()
            .next()
            .is_none_or(|byte| !byte.is_ascii_uppercase())
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err(graph_error(
            DiagnosticClass::Semantic,
            "graph_declaration_name",
            format!("{label} is not a canonical declaration name"),
        ));
    }
    Ok(())
}

fn graph_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{MigrationIdentityAllocator, SourceLimits, parse_module, parse_source};

    #[test]
    fn root_and_modules_round_trip_with_exact_shard_binding() {
        let document = parse_source(
            "fixture.lkj",
            b"(module sample (record Item (name Text)))\n",
            SourceLimits::default(),
        )
        .expect("source");
        let module = parse_module(&document).expect("module");
        let mut allocator = MigrationIdentityAllocator::new(b"fixture".to_vec());
        let meaning = MeaningModule::import(module, &mut allocator).expect("meaning");
        let root = GraphRoot {
            graph_contract_version: GRAPH_CONTRACT_VERSION,
            repository_id: RepositoryId::migrate(b"fixture", 1),
            package_id: PackageId::parse("10000000000000000000000000000001").expect("package"),
            package_name: "fixture".to_owned(),
            modules: vec![ModuleObjectRef {
                id: meaning.module_id,
                name: meaning.module.name.clone(),
                object: meaning.digest().expect("digest"),
            }],
            dependencies: Vec::new(),
            targets: Vec::new(),
            tombstones: Vec::new(),
        };
        let bytes = root.encode().expect("encode");
        assert_eq!(GraphRoot::decode(&bytes).expect("decode"), root);
        root.validate_modules(&[meaning]).expect("modules");
    }
}
