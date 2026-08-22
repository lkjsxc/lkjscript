//! Canonical repository/package root for packed meaning-module shards.

use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::meaning::{GRAPH_CONTRACT_VERSION, MeaningModule};
use super::package::{PackageId, RunnerKind};
use super::packed;
use super::persistent_map::{
    MapError, MapErrorClass, MapRoot, MapWork, MemoryPageStore, OverlayPageStore, PageStore,
    PersistentMap, RemoveOutcome,
};
use super::semantic_digest::{ArtifactDigest, ModuleObjectDigest, RootObjectDigest};
use super::semantic_id::{
    AnnotationId, BindingId, CaseId, DeclarationId, DocumentationId, ExpressionId, FieldId,
    ModuleId, OperationId, ParameterId, PortId, RepositoryId, RequirementId, RevisionId, TargetId,
    TypeParameterId,
};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MAXIMUM_ROOT_BYTES: usize = 16 * 1_048_576;
pub const ROOT_STORAGE_CONTRACT_VERSION: u16 = 2;
pub const ROOT_STORAGE_CONTRACT_IDENTITY: &str = "lkjscript-persistent-root-2";
pub const MAXIMUM_STORED_ROOT_BYTES: usize = 64 * 1024;
const ROOT_MAGIC: [u8; 8] = *b"LKJGRF04";
const ROOT_DIGEST_DOMAIN: &str = "lkjscript.logical-graph-root.v4";
const STORED_ROOT_MAGIC: [u8; 8] = *b"LKJROOT3";
const STORED_ROOT_DIGEST_DOMAIN: &str = "lkjscript.persistent-root-object.v2";
const ROOT_VALUE_LIMIT: usize = 64 * 1024;

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
    pub component: DeclarationId,
    pub port: PortId,
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
    TypeParameter(TypeParameterId),
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

/// Fixed-size canonical root manifest. Collection contents live in immutable Merkle radix pages;
/// this object is the value bound by an accepted revision record.
#[derive(Decode, Encode, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StoredGraphRoot {
    pub storage_contract_version: u16,
    pub graph_contract_version: u16,
    pub repository_id: RepositoryId,
    pub package_id: PackageId,
    pub package_name: String,
    pub modules: MapRoot,
    pub module_names: MapRoot,
    /// Exact dependency objects keyed by stable package identity.
    pub dependencies: MapRoot,
    /// Presentation aliases keyed to exact package identity.
    pub dependency_aliases: MapRoot,
    pub targets: MapRoot,
    pub tombstones: MapRoot,
}

#[derive(Clone, Debug)]
pub struct StoredGraphRootBuild {
    pub root: StoredGraphRoot,
    pub pages: MemoryPageStore,
    pub work: MapWork,
}

#[derive(Clone, Debug, Default)]
pub struct StoredGraphRootDelta {
    pub package_name: Option<String>,
    pub module_removals: Vec<ModuleObjectRef>,
    pub module_upserts: Vec<ModuleObjectRef>,
    pub dependency_removals: Vec<DependencyBinding>,
    pub dependency_upserts: Vec<DependencyBinding>,
    pub target_removals: Vec<TargetBinding>,
    pub target_upserts: Vec<TargetBinding>,
    pub tombstone_removals: Vec<Tombstone>,
    pub tombstone_upserts: Vec<Tombstone>,
}

#[derive(Clone, Debug)]
pub struct StoredGraphRootUpdate {
    pub root: StoredGraphRoot,
    pub pages: MemoryPageStore,
    pub work: MapWork,
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
        StoredGraphRoot::build(self)?.root.digest()
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
        if self.modules.is_empty() {
            return Err(graph_error(
                DiagnosticClass::Semantic,
                "graph_root_module_count",
                "graph root must contain at least one module",
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
        let mut dependency_packages = BTreeSet::new();
        for dependency in &self.dependencies {
            validate_name(&dependency.alias, "dependency alias", false)?;
            if !aliases.insert(&dependency.alias)
                || !dependency_packages.insert(&dependency.package_id)
            {
                return Err(graph_error(
                    DiagnosticClass::Corrupt,
                    "graph_root_dependency_duplicate",
                    "dependency alias or exact package identity is duplicated",
                ));
            }
        }
        let mut target_names = BTreeSet::new();
        let mut target_ids = BTreeSet::new();
        for target in &self.targets {
            validate_name(&target.name, "target name", false)?;
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
                        super::meaning::MemberIdentity::TypeParameter { id, .. } => {
                            ("type_parameter", id.bytes())
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
            let Some((_, component)) = module.declaration(target.component) else {
                return Err(graph_error(
                    DiagnosticClass::Corrupt,
                    "graph_target_component_missing",
                    format!("target '{}' references a missing component", target.name),
                ));
            };
            if !matches!(component, super::language::Declaration::Component(_)) {
                return Err(graph_error(
                    DiagnosticClass::Corrupt,
                    "graph_target_component_kind",
                    format!("target '{}' does not reference a component", target.name),
                ));
            }
            let port_matches = module.declarations.iter().any(|declaration| {
                declaration.id == target.component
                    && declaration.members.iter().any(|member| {
                        matches!(
                            member,
                            super::meaning::MemberIdentity::Port { id, .. }
                                if *id == target.port
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

impl StoredGraphRootDelta {
    pub fn between(before: &GraphRoot, after: &GraphRoot) -> Result<Self, Diagnostic> {
        before.validate_shape()?;
        after.validate_shape()?;
        if before.repository_id != after.repository_id || before.package_id != after.package_id {
            return Err(graph_error(
                DiagnosticClass::Source,
                "graph_root_delta_identity",
                "a persistent-root delta cannot change repository or package identity",
            ));
        }
        let (module_removals, module_upserts) =
            collection_delta(&before.modules, &after.modules, |value| value.id);
        let (dependency_removals, dependency_upserts) =
            collection_delta(&before.dependencies, &after.dependencies, |value| {
                value.alias.clone()
            });
        let (target_removals, target_upserts) =
            collection_delta(&before.targets, &after.targets, |value| value.id);
        let (tombstone_removals, tombstone_upserts) =
            collection_delta(&before.tombstones, &after.tombstones, |value| {
                value.identity.clone()
            });
        Ok(Self {
            package_name: (before.package_name != after.package_name)
                .then(|| after.package_name.clone()),
            module_removals,
            module_upserts,
            dependency_removals,
            dependency_upserts,
            target_removals,
            target_upserts,
            tombstone_removals,
            tombstone_upserts,
        })
    }
}

impl StoredGraphRoot {
    pub fn build(graph: &GraphRoot) -> Result<StoredGraphRootBuild, Diagnostic> {
        graph.validate_shape()?;
        let mut staging = MemoryPageStore::default();
        let mut work = MapWork::default();
        let modules = PersistentMap::from_sorted(
            &mut staging,
            graph
                .modules
                .iter()
                .map(|reference| Ok((reference.id.bytes().to_vec(), encode_root_value(reference)?)))
                .collect::<Result<Vec<_>, Diagnostic>>()?,
            &mut work,
        )
        .map_err(map_diagnostic)?;
        let mut module_name_entries = graph
            .modules
            .iter()
            .map(|reference| {
                Ok((
                    reference.name.as_bytes().to_vec(),
                    encode_root_value(&reference.id)?,
                ))
            })
            .collect::<Result<Vec<_>, Diagnostic>>()?;
        module_name_entries.sort_by(|left, right| left.0.cmp(&right.0));
        let module_names = PersistentMap::from_sorted(&mut staging, module_name_entries, &mut work)
            .map_err(map_diagnostic)?;
        let mut dependency_entries = graph
            .dependencies
            .iter()
            .map(|binding| {
                Ok((
                    binding.package_id.bytes().to_vec(),
                    encode_root_value(binding)?,
                ))
            })
            .collect::<Result<Vec<_>, Diagnostic>>()?;
        dependency_entries.sort_by(|left, right| left.0.cmp(&right.0));
        let dependencies = PersistentMap::from_sorted(&mut staging, dependency_entries, &mut work)
            .map_err(map_diagnostic)?;
        let mut dependency_alias_entries = graph
            .dependencies
            .iter()
            .map(|binding| {
                Ok((
                    binding.alias.as_bytes().to_vec(),
                    encode_root_value(&binding.package_id)?,
                ))
            })
            .collect::<Result<Vec<_>, Diagnostic>>()?;
        dependency_alias_entries.sort_by(|left, right| left.0.cmp(&right.0));
        let dependency_aliases =
            PersistentMap::from_sorted(&mut staging, dependency_alias_entries, &mut work)
                .map_err(map_diagnostic)?;
        let targets = PersistentMap::from_sorted(
            &mut staging,
            graph
                .targets
                .iter()
                .map(|binding| Ok((binding.id.bytes().to_vec(), encode_root_value(binding)?)))
                .collect::<Result<Vec<_>, Diagnostic>>()?,
            &mut work,
        )
        .map_err(map_diagnostic)?;
        let tombstones = PersistentMap::from_sorted(
            &mut staging,
            graph
                .tombstones
                .iter()
                .map(|tombstone| {
                    Ok((
                        tombstone_key(&tombstone.identity),
                        encode_root_value(tombstone)?,
                    ))
                })
                .collect::<Result<Vec<_>, Diagnostic>>()?,
            &mut work,
        )
        .map_err(map_diagnostic)?;

        let root = Self {
            storage_contract_version: ROOT_STORAGE_CONTRACT_VERSION,
            graph_contract_version: graph.graph_contract_version,
            repository_id: graph.repository_id,
            package_id: graph.package_id.clone(),
            package_name: graph.package_name.clone(),
            modules: modules.root(),
            module_names: module_names.root(),
            dependencies: dependencies.root(),
            dependency_aliases: dependency_aliases.root(),
            targets: targets.root(),
            tombstones: tombstones.root(),
        };
        root.validate_shape()?;

        // Path-copy construction may leave unreachable staging pages. Only the exact reachable
        // closure is returned for publication, backup, or artifact assembly.
        let mut pages = MemoryPageStore::default();
        for map in [
            modules,
            module_names,
            dependencies,
            dependency_aliases,
            targets,
            tombstones,
        ] {
            map.copy_reachable(&staging, &mut pages, &mut work)
                .map_err(map_diagnostic)?;
        }
        Ok(StoredGraphRootBuild { root, pages, work })
    }

    /// Applies an exact logical delta by path-copying only affected persistent-map pages. The
    /// returned page set contains the final generated path pages, including exact physical reuse,
    /// but excludes unchanged base-only subtrees and intermediate mutation roots.
    pub fn apply_delta<S: PageStore + ?Sized>(
        &self,
        store: &S,
        delta: &StoredGraphRootDelta,
    ) -> Result<StoredGraphRootUpdate, Diagnostic> {
        self.validate_shape()?;
        let mut overlay = OverlayPageStore::new(store);
        let mut work = MapWork::default();
        let mut modules = PersistentMap::from_root(self.modules);
        let mut module_names = PersistentMap::from_root(self.module_names);
        let mut dependencies = PersistentMap::from_root(self.dependencies);
        let mut dependency_aliases = PersistentMap::from_root(self.dependency_aliases);
        let mut targets = PersistentMap::from_root(self.targets);
        let mut tombstones = PersistentMap::from_root(self.tombstones);

        for reference in &delta.module_removals {
            modules = remove_exact(
                modules,
                &mut overlay,
                &reference.id.bytes(),
                reference,
                &mut work,
                "graph_root_delta_module_missing",
            )?;
            module_names = remove_exact(
                module_names,
                &mut overlay,
                reference.name.as_bytes(),
                &reference.id,
                &mut work,
                "graph_root_delta_module_name_missing",
            )?;
        }
        for reference in &delta.module_upserts {
            if let Some(existing) = module_names
                .lookup(&overlay, reference.name.as_bytes(), &mut work)
                .map_err(map_diagnostic)?
            {
                let existing: ModuleId = decode_root_value(&existing)?;
                if existing != reference.id {
                    return Err(graph_error(
                        DiagnosticClass::Semantic,
                        "graph_root_delta_module_name_duplicate",
                        "module name is already bound to a different stable identity",
                    ));
                }
            }
            modules = modules
                .insert(
                    &mut overlay,
                    &reference.id.bytes(),
                    &encode_root_value(reference)?,
                    &mut work,
                )
                .map_err(map_diagnostic)?
                .0;
            module_names = module_names
                .insert(
                    &mut overlay,
                    reference.name.as_bytes(),
                    &encode_root_value(&reference.id)?,
                    &mut work,
                )
                .map_err(map_diagnostic)?
                .0;
        }
        for binding in &delta.dependency_removals {
            dependencies = remove_exact(
                dependencies,
                &mut overlay,
                &binding.package_id.bytes(),
                binding,
                &mut work,
                "graph_root_delta_dependency_missing",
            )?;
            dependency_aliases = remove_exact(
                dependency_aliases,
                &mut overlay,
                binding.alias.as_bytes(),
                &binding.package_id,
                &mut work,
                "graph_root_delta_dependency_alias_missing",
            )?;
        }
        for binding in &delta.dependency_upserts {
            if let Some(existing) = dependency_aliases
                .lookup(&overlay, binding.alias.as_bytes(), &mut work)
                .map_err(map_diagnostic)?
            {
                let existing: PackageId = decode_root_value(&existing)?;
                if existing != binding.package_id {
                    return Err(graph_error(
                        DiagnosticClass::Semantic,
                        "graph_root_delta_dependency_alias_duplicate",
                        "dependency alias is already bound to a different package identity",
                    ));
                }
            }
            dependencies = dependencies
                .insert(
                    &mut overlay,
                    &binding.package_id.bytes(),
                    &encode_root_value(binding)?,
                    &mut work,
                )
                .map_err(map_diagnostic)?
                .0;
            dependency_aliases = dependency_aliases
                .insert(
                    &mut overlay,
                    binding.alias.as_bytes(),
                    &encode_root_value(&binding.package_id)?,
                    &mut work,
                )
                .map_err(map_diagnostic)?
                .0;
        }
        for binding in &delta.target_removals {
            targets = remove_exact(
                targets,
                &mut overlay,
                &binding.id.bytes(),
                binding,
                &mut work,
                "graph_root_delta_target_missing",
            )?;
        }
        for binding in &delta.target_upserts {
            targets = targets
                .insert(
                    &mut overlay,
                    &binding.id.bytes(),
                    &encode_root_value(binding)?,
                    &mut work,
                )
                .map_err(map_diagnostic)?
                .0;
        }
        for tombstone in &delta.tombstone_removals {
            tombstones = remove_exact(
                tombstones,
                &mut overlay,
                &tombstone_key(&tombstone.identity),
                tombstone,
                &mut work,
                "graph_root_delta_tombstone_missing",
            )?;
        }
        for tombstone in &delta.tombstone_upserts {
            tombstones = tombstones
                .insert(
                    &mut overlay,
                    &tombstone_key(&tombstone.identity),
                    &encode_root_value(tombstone)?,
                    &mut work,
                )
                .map_err(map_diagnostic)?
                .0;
        }

        let root = Self {
            storage_contract_version: self.storage_contract_version,
            graph_contract_version: self.graph_contract_version,
            repository_id: self.repository_id,
            package_id: self.package_id.clone(),
            package_name: delta
                .package_name
                .clone()
                .unwrap_or_else(|| self.package_name.clone()),
            modules: modules.root(),
            module_names: module_names.root(),
            dependencies: dependencies.root(),
            dependency_aliases: dependency_aliases.root(),
            targets: targets.root(),
            tombstones: tombstones.root(),
        };
        root.validate_shape()?;

        let staged = overlay.into_pages();
        let mut reachable = MemoryPageStore::default();
        for (map, base) in [
            (modules, self.modules),
            (module_names, self.module_names),
            (dependencies, self.dependencies),
            (dependency_aliases, self.dependency_aliases),
            (targets, self.targets),
            (tombstones, self.tombstones),
        ] {
            if map.root() != base {
                map.copy_staged_reachable(&staged, &mut reachable, &mut work)
                    .map_err(map_diagnostic)?;
            }
        }
        Ok(StoredGraphRootUpdate {
            root,
            pages: reachable,
            work,
        })
    }

    pub fn encode(&self) -> Result<Vec<u8>, Diagnostic> {
        self.validate_shape()?;
        packed::encode(
            STORED_ROOT_MAGIC,
            STORED_ROOT_DIGEST_DOMAIN,
            self,
            MAXIMUM_STORED_ROOT_BYTES,
        )
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, Diagnostic> {
        let root: Self = packed::decode(
            bytes,
            STORED_ROOT_MAGIC,
            STORED_ROOT_DIGEST_DOMAIN,
            MAXIMUM_STORED_ROOT_BYTES,
        )?;
        root.validate_shape()?;
        Ok(root)
    }

    pub fn digest(&self) -> Result<RootObjectDigest, Diagnostic> {
        Ok(RootObjectDigest::of(&self.encode()?))
    }

    pub fn validate_shape(&self) -> Result<(), Diagnostic> {
        if self.storage_contract_version != ROOT_STORAGE_CONTRACT_VERSION {
            return Err(graph_error(
                DiagnosticClass::Source,
                "graph_root_storage_contract",
                format!(
                    "persistent root storage contract {} is not current contract {ROOT_STORAGE_CONTRACT_VERSION}",
                    self.storage_contract_version
                ),
            ));
        }
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
        if self.modules.entries() == 0 || self.modules.entries() != self.module_names.entries() {
            return Err(graph_error(
                DiagnosticClass::Corrupt,
                "graph_root_module_summary",
                "persistent module and name maps must contain the same nonzero item count",
            ));
        }
        if self.dependencies.entries() != self.dependency_aliases.entries() {
            return Err(graph_error(
                DiagnosticClass::Corrupt,
                "graph_root_dependency_summary",
                "persistent dependency identity and alias maps must contain the same item count",
            ));
        }
        Ok(())
    }

    pub fn reconstruct<S: PageStore + ?Sized>(
        &self,
        store: &S,
        work: &mut MapWork,
    ) -> Result<GraphRoot, Diagnostic> {
        self.validate_shape()?;
        let mut modules = Vec::new();
        PersistentMap::from_root(self.modules)
            .for_each(store, work, |key, value| {
                let reference: ModuleObjectRef =
                    decode_root_value(value).map_err(diagnostic_map)?;
                if key != reference.id.bytes() {
                    return Err(storage_map_error(
                        "graph_root_module_key",
                        "module map key does not match its typed stable identity",
                    ));
                }
                modules.push(reference);
                Ok(())
            })
            .map_err(map_diagnostic)?;

        let mut names = BTreeMap::<String, ModuleId>::new();
        PersistentMap::from_root(self.module_names)
            .for_each(store, work, |key, value| {
                let name = std::str::from_utf8(key).map_err(|_| {
                    storage_map_error(
                        "graph_root_module_name_utf8",
                        "module name map contains non-UTF-8 key bytes",
                    )
                })?;
                let id = module_id_from_bytes(value)?;
                if names.insert(name.to_owned(), id).is_some() {
                    return Err(storage_map_error(
                        "graph_root_module_name_duplicate",
                        "module name map contains a duplicate name",
                    ));
                }
                Ok(())
            })
            .map_err(map_diagnostic)?;
        if modules.len() != names.len()
            || modules
                .iter()
                .any(|reference| names.get(&reference.name) != Some(&reference.id))
        {
            return Err(graph_error(
                DiagnosticClass::Corrupt,
                "graph_root_module_name_binding",
                "module identity and name maps do not describe the same namespace",
            ));
        }

        let dependencies = decode_map_values::<DependencyBinding, _>(
            PersistentMap::from_root(self.dependencies),
            store,
            work,
            |value| value.package_id.bytes().to_vec(),
            "graph_root_dependency_key",
        )?;
        let mut dependency_aliases = BTreeMap::<String, PackageId>::new();
        PersistentMap::from_root(self.dependency_aliases)
            .for_each(store, work, |key, value| {
                let alias = std::str::from_utf8(key).map_err(|_| {
                    storage_map_error(
                        "graph_root_dependency_alias_utf8",
                        "dependency alias map contains non-UTF-8 key bytes",
                    )
                })?;
                let package: PackageId = decode_root_value(value).map_err(diagnostic_map)?;
                if dependency_aliases
                    .insert(alias.to_owned(), package)
                    .is_some()
                {
                    return Err(storage_map_error(
                        "graph_root_dependency_alias_duplicate",
                        "dependency alias map contains a duplicate alias",
                    ));
                }
                Ok(())
            })
            .map_err(map_diagnostic)?;
        if dependencies.len() != dependency_aliases.len()
            || dependencies
                .iter()
                .any(|binding| dependency_aliases.get(&binding.alias) != Some(&binding.package_id))
        {
            return Err(graph_error(
                DiagnosticClass::Corrupt,
                "graph_root_dependency_alias_binding",
                "dependency identity and alias maps do not describe the same bindings",
            ));
        }
        let targets = decode_map_values::<TargetBinding, _>(
            PersistentMap::from_root(self.targets),
            store,
            work,
            |value| value.id.bytes().to_vec(),
            "graph_root_target_key",
        )?;
        let tombstones = decode_map_values::<Tombstone, _>(
            PersistentMap::from_root(self.tombstones),
            store,
            work,
            |value| tombstone_key(&value.identity),
            "graph_root_tombstone_key",
        )?;
        let graph = GraphRoot {
            graph_contract_version: self.graph_contract_version,
            repository_id: self.repository_id,
            package_id: self.package_id.clone(),
            package_name: self.package_name.clone(),
            modules,
            dependencies,
            targets,
            tombstones,
        };
        graph.validate_shape()?;
        Ok(graph)
    }

    pub fn module_by_id<S: PageStore + ?Sized>(
        &self,
        store: &S,
        id: ModuleId,
        work: &mut MapWork,
    ) -> Result<Option<ModuleObjectRef>, Diagnostic> {
        PersistentMap::from_root(self.modules)
            .lookup(store, &id.bytes(), work)
            .map_err(map_diagnostic)?
            .map(|bytes| decode_root_value(&bytes))
            .transpose()
    }

    /// Visits exact module bindings without reconstructing the other logical root maps.
    pub fn for_each_module_reference<S, F>(
        &self,
        store: &S,
        work: &mut MapWork,
        mut visitor: F,
    ) -> Result<(), Diagnostic>
    where
        S: PageStore + ?Sized,
        F: FnMut(&ModuleObjectRef) -> Result<(), Diagnostic>,
    {
        PersistentMap::from_root(self.modules)
            .for_each(store, work, |key, value| {
                let reference: ModuleObjectRef =
                    decode_root_value(value).map_err(diagnostic_map)?;
                if key != reference.id.bytes() {
                    return Err(storage_map_error(
                        "graph_root_module_key",
                        "module map key does not match its typed stable identity",
                    ));
                }
                visitor(&reference).map_err(diagnostic_map)
            })
            .map_err(map_diagnostic)
    }

    pub fn module_by_name<S: PageStore + ?Sized>(
        &self,
        store: &S,
        name: &str,
        work: &mut MapWork,
    ) -> Result<Option<ModuleObjectRef>, Diagnostic> {
        let Some(bytes) = PersistentMap::from_root(self.module_names)
            .lookup(store, name.as_bytes(), work)
            .map_err(map_diagnostic)?
        else {
            return Ok(None);
        };
        let id = module_id_from_bytes(&bytes).map_err(map_diagnostic)?;
        self.module_by_id(store, id, work)
    }

    pub fn dependency_bindings<S: PageStore + ?Sized>(
        &self,
        store: &S,
        work: &mut MapWork,
    ) -> Result<Vec<DependencyBinding>, Diagnostic> {
        decode_map_values::<DependencyBinding, _>(
            PersistentMap::from_root(self.dependencies),
            store,
            work,
            |value| value.package_id.bytes().to_vec(),
            "graph_root_dependency_key",
        )
    }

    /// Visits exact dependency bindings without reconstructing unrelated root maps.
    pub fn for_each_dependency_binding<S, F>(
        &self,
        store: &S,
        work: &mut MapWork,
        mut visitor: F,
    ) -> Result<(), Diagnostic>
    where
        S: PageStore + ?Sized,
        F: FnMut(&DependencyBinding) -> Result<(), Diagnostic>,
    {
        PersistentMap::from_root(self.dependencies)
            .for_each(store, work, |key, value| {
                let binding: DependencyBinding =
                    decode_root_value(value).map_err(diagnostic_map)?;
                if key != binding.package_id.bytes() {
                    return Err(storage_map_error(
                        "graph_root_dependency_key",
                        "dependency map key does not match its typed package identity",
                    ));
                }
                visitor(&binding).map_err(diagnostic_map)
            })
            .map_err(map_diagnostic)
    }

    pub fn dependency_by_alias<S: PageStore + ?Sized>(
        &self,
        store: &S,
        alias: &str,
        work: &mut MapWork,
    ) -> Result<Option<DependencyBinding>, Diagnostic> {
        let Some(bytes) = PersistentMap::from_root(self.dependency_aliases)
            .lookup(store, alias.as_bytes(), work)
            .map_err(map_diagnostic)?
        else {
            return Ok(None);
        };
        let package: PackageId = decode_root_value(&bytes)?;
        self.dependency_by_package(store, &package, work)
    }

    pub fn dependency_by_package<S: PageStore + ?Sized>(
        &self,
        store: &S,
        package: &PackageId,
        work: &mut MapWork,
    ) -> Result<Option<DependencyBinding>, Diagnostic> {
        PersistentMap::from_root(self.dependencies)
            .lookup(store, &package.bytes(), work)
            .map_err(map_diagnostic)?
            .map(|bytes| decode_root_value(&bytes))
            .transpose()
    }

    pub fn tombstone_by_identity<S: PageStore + ?Sized>(
        &self,
        store: &S,
        identity: &TombstoneIdentity,
        work: &mut MapWork,
    ) -> Result<Option<Tombstone>, Diagnostic> {
        PersistentMap::from_root(self.tombstones)
            .lookup(store, &tombstone_key(identity), work)
            .map_err(map_diagnostic)?
            .map(|bytes| decode_root_value(&bytes))
            .transpose()
    }
}

fn collection_delta<T, K>(before: &[T], after: &[T], key: impl Fn(&T) -> K) -> (Vec<T>, Vec<T>)
where
    T: Clone + Eq,
    K: Clone + Ord,
{
    let before = before
        .iter()
        .map(|value| (key(value), value.clone()))
        .collect::<BTreeMap<_, _>>();
    let after = after
        .iter()
        .map(|value| (key(value), value.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut removals = Vec::new();
    for (identity, value) in &before {
        if after.get(identity) != Some(value) {
            removals.push(value.clone());
        }
    }
    let mut upserts = Vec::new();
    for (identity, value) in &after {
        if before.get(identity) != Some(value) {
            upserts.push(value.clone());
        }
    }
    (removals, upserts)
}

fn remove_exact<T, S>(
    map: PersistentMap,
    store: &mut S,
    key: &[u8],
    expected: &T,
    work: &mut MapWork,
    code: &'static str,
) -> Result<PersistentMap, Diagnostic>
where
    T: Decode<()> + Eq,
    S: PageStore + ?Sized,
{
    let (next, outcome) = map.remove(store, key, work).map_err(map_diagnostic)?;
    let RemoveOutcome::Removed { previous } = outcome else {
        return Err(graph_error(
            DiagnosticClass::Corrupt,
            code,
            "persistent-root delta expected an exact base entry that is absent",
        ));
    };
    let previous: T = decode_root_value(&previous)?;
    if &previous != expected {
        return Err(graph_error(
            DiagnosticClass::Corrupt,
            code,
            "persistent-root delta base entry differs from its exact expected value",
        ));
    }
    Ok(next)
}

fn decode_map_values<T, S>(
    map: PersistentMap,
    store: &S,
    work: &mut MapWork,
    key: impl Fn(&T) -> Vec<u8>,
    code: &'static str,
) -> Result<Vec<T>, Diagnostic>
where
    T: Decode<()> + Ord,
    S: PageStore + ?Sized,
{
    let mut values = Vec::new();
    map.for_each(store, work, |encoded_key, encoded_value| {
        let value: T = decode_root_value(encoded_value).map_err(diagnostic_map)?;
        if encoded_key != key(&value) {
            return Err(storage_map_error(
                code,
                "persistent root key disagrees with its typed value",
            ));
        }
        values.push(value);
        Ok(())
    })
    .map_err(map_diagnostic)?;
    values.sort();
    Ok(values)
}

fn encode_root_value<T: Encode>(value: &T) -> Result<Vec<u8>, Diagnostic> {
    let configuration = bincode::config::standard()
        .with_little_endian()
        .with_variable_int_encoding()
        .with_limit::<ROOT_VALUE_LIMIT>();
    bincode::encode_to_vec(value, configuration).map_err(|error| {
        graph_error(
            DiagnosticClass::Infrastructure,
            "graph_root_value_encode",
            format!("persistent root value could not be encoded: {error}"),
        )
    })
}

fn decode_root_value<T: Decode<()>>(bytes: &[u8]) -> Result<T, Diagnostic> {
    let configuration = bincode::config::standard()
        .with_little_endian()
        .with_variable_int_encoding()
        .with_limit::<ROOT_VALUE_LIMIT>();
    let (value, consumed): (T, usize) =
        bincode::decode_from_slice(bytes, configuration).map_err(|error| {
            graph_error(
                DiagnosticClass::Corrupt,
                "graph_root_value_decode",
                format!("persistent root value is malformed: {error}"),
            )
        })?;
    if consumed != bytes.len() {
        return Err(graph_error(
            DiagnosticClass::Corrupt,
            "graph_root_value_trailing",
            "persistent root value has trailing bytes",
        ));
    }
    Ok(value)
}

fn tombstone_key(identity: &TombstoneIdentity) -> Vec<u8> {
    let (tag, bytes) = match identity {
        TombstoneIdentity::Module(id) => (1, id.bytes()),
        TombstoneIdentity::Declaration(id) => (2, id.bytes()),
        TombstoneIdentity::Field(id) => (3, id.bytes()),
        TombstoneIdentity::Case(id) => (4, id.bytes()),
        TombstoneIdentity::Operation(id) => (5, id.bytes()),
        TombstoneIdentity::Parameter(id) => (6, id.bytes()),
        TombstoneIdentity::Binding(id) => (7, id.bytes()),
        TombstoneIdentity::Expression(id) => (8, id.bytes()),
        TombstoneIdentity::Requirement(id) => (9, id.bytes()),
        TombstoneIdentity::Port(id) => (10, id.bytes()),
        TombstoneIdentity::Target(id) => (11, id.bytes()),
        TombstoneIdentity::Documentation(id) => (12, id.bytes()),
        TombstoneIdentity::Annotation(id) => (13, id.bytes()),
        TombstoneIdentity::TypeParameter(id) => (14, id.bytes()),
    };
    let mut key = Vec::with_capacity(17);
    key.push(tag);
    key.extend_from_slice(&bytes);
    key
}

fn module_id_from_bytes(bytes: &[u8]) -> Result<ModuleId, MapError> {
    decode_root_value(bytes).map_err(diagnostic_map)
}

fn storage_map_error(code: &'static str, message: impl Into<String>) -> MapError {
    MapError {
        class: MapErrorClass::Corrupt,
        code,
        message: message.into(),
    }
}

fn diagnostic_map(error: Diagnostic) -> MapError {
    MapError {
        class: match error.class {
            DiagnosticClass::Source | DiagnosticClass::Semantic => MapErrorClass::Input,
            DiagnosticClass::Resource => MapErrorClass::Resource,
            DiagnosticClass::Corrupt => MapErrorClass::Corrupt,
            DiagnosticClass::Capability
            | DiagnosticClass::Cancelled
            | DiagnosticClass::Infrastructure => MapErrorClass::Store,
        },
        code: "graph_root_value",
        message: error.message,
    }
}

fn map_diagnostic(error: MapError) -> Diagnostic {
    graph_error(
        match error.class {
            MapErrorClass::Input => DiagnosticClass::Source,
            MapErrorClass::Resource => DiagnosticClass::Resource,
            MapErrorClass::Corrupt => DiagnosticClass::Corrupt,
            MapErrorClass::Store => DiagnosticClass::Infrastructure,
        },
        error.code,
        error.message,
    )
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
                    super::meaning::MemberIdentity::TypeParameter { id, .. } => {
                        live.insert(("type_parameter", id.bytes()));
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
        TombstoneIdentity::TypeParameter(id) => ("type_parameter", id.bytes()),
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

fn graph_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::persistent_map::{PageDigest, PageWrite};
    use crate::platform::{MigrationIdentityAllocator, SourceLimits, parse_module, parse_source};
    use std::cell::Cell;

    struct CountingPageStore {
        inner: MemoryPageStore,
        reads: Cell<u64>,
        bytes_read: Cell<u64>,
    }

    impl CountingPageStore {
        const fn new(inner: MemoryPageStore) -> Self {
            Self {
                inner,
                reads: Cell::new(0),
                bytes_read: Cell::new(0),
            }
        }
    }

    impl PageStore for CountingPageStore {
        fn read_page(&self, digest: PageDigest) -> Result<Option<Vec<u8>>, MapError> {
            self.reads.set(self.reads.get() + 1);
            let bytes = self.inner.read_page(digest)?;
            if let Some(bytes) = &bytes {
                self.bytes_read.set(
                    self.bytes_read.get()
                        + u64::try_from(bytes.len()).expect("test page length fits u64"),
                );
            }
            Ok(bytes)
        }

        fn write_page(&mut self, digest: PageDigest, bytes: &[u8]) -> Result<PageWrite, MapError> {
            self.inner.write_page(digest, bytes)
        }
    }

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

        let stored = StoredGraphRoot::build(&root).expect("persistent root");
        let reconstructed = stored
            .root
            .reconstruct(&stored.pages, &mut MapWork::default())
            .expect("persistent root reconstruction");
        assert_eq!(reconstructed, root);
        assert_eq!(
            stored.root.digest().expect("stored digest"),
            root.digest().expect("root digest")
        );

        let mut changed = root.clone();
        changed.modules[0].object = ModuleObjectDigest::of(b"changed module object");
        let delta = StoredGraphRootDelta::between(&root, &changed).expect("root delta");
        let update = stored
            .root
            .apply_delta(&stored.pages, &delta)
            .expect("local persistent-root update");
        let rebuilt = StoredGraphRoot::build(&changed).expect("full root oracle");
        assert_eq!(update.root, rebuilt.root);
        assert_eq!(update.pages.object_count(), 1);

        let mut combined = stored.pages.clone();
        let new_pages = update
            .pages
            .objects()
            .map(|(digest, bytes)| (digest, bytes.to_vec()))
            .collect::<Vec<_>>();
        for (digest, bytes) in new_pages {
            combined
                .write_page(digest, &bytes)
                .expect("new root page must publish");
        }
        assert_eq!(
            update
                .root
                .reconstruct(&combined, &mut MapWork::default())
                .expect("updated root reconstruction"),
            changed
        );
    }

    #[test]
    fn local_delta_page_reads_do_not_scan_a_large_unchanged_root() {
        let mut observations = Vec::new();
        for module_count in [10_000_u64, 100_000_u64] {
            let mut modules = (0..module_count)
                .map(|ordinal| ModuleObjectRef {
                    id: ModuleId::migrate(b"large-root-module", ordinal),
                    name: format!("module{ordinal:06}"),
                    object: ModuleObjectDigest::of(&ordinal.to_be_bytes()),
                })
                .collect::<Vec<_>>();
            modules.sort();
            let root = GraphRoot {
                graph_contract_version: GRAPH_CONTRACT_VERSION,
                repository_id: RepositoryId::migrate(b"large-root-repository", 1),
                package_id: PackageId::parse("10000000000000000000000000000001").expect("package"),
                package_name: "large-root".to_owned(),
                modules,
                dependencies: Vec::new(),
                targets: Vec::new(),
                tombstones: Vec::new(),
            };
            let stored = StoredGraphRoot::build(&root).expect("large persistent root");
            let base_pages = stored.pages.object_count();
            let base_bytes = stored.pages.stored_bytes();
            assert!(
                base_pages > 64,
                "fixture must contain a broad page background"
            );
            let counted_store = CountingPageStore::new(stored.pages);

            let mut changed = root.clone();
            let changed_index = usize::try_from(module_count / 2).expect("module index fits usize");
            changed.modules[changed_index].object =
                ModuleObjectDigest::of(b"one local module update");
            let delta = StoredGraphRootDelta::between(&root, &changed).expect("one-module delta");
            let update = stored
                .root
                .apply_delta(&counted_store, &delta)
                .expect("bounded local persistent-root update");
            let rebuilt = StoredGraphRoot::build(&changed).expect("full persistent-root oracle");
            assert_eq!(update.root, rebuilt.root);
            let physical_reads = counted_store.reads.get();
            let physical_bytes_read = counted_store.bytes_read.get();
            let retained_pages = update.pages.object_count();
            assert!(
                physical_reads < 64 && physical_reads < (base_pages as u64 / 4),
                "{module_count}-module update physically read {physical_reads} of {base_pages} base pages"
            );
            assert!(
                physical_bytes_read
                    < u64::try_from(base_bytes / 4).expect("base byte count fits u64"),
                "{module_count}-module update physically read {physical_bytes_read} of {base_bytes} base bytes"
            );
            assert!(
                retained_pages < 32,
                "{module_count}-module update retained {retained_pages} pages"
            );

            let mut combined = counted_store.inner;
            for (digest, bytes) in update.pages.objects() {
                combined
                    .write_page(digest, bytes)
                    .expect("publish one-module root pages");
            }
            assert_eq!(
                update
                    .root
                    .reconstruct(&combined, &mut MapWork::default())
                    .expect("updated large root reconstruction"),
                changed
            );
            observations.push((physical_reads, physical_bytes_read, retained_pages));
        }

        let (reads_10k, bytes_10k, pages_10k) = observations[0];
        let (reads_100k, bytes_100k, pages_100k) = observations[1];
        assert!(
            reads_100k <= reads_10k + 8,
            "physical page reads grew from {reads_10k} at 10k modules to {reads_100k} at 100k"
        );
        assert!(
            bytes_100k <= bytes_10k.saturating_mul(2),
            "physical bytes grew from {bytes_10k} at 10k modules to {bytes_100k} at 100k"
        );
        assert!(
            pages_100k <= pages_10k + 8,
            "retained pages grew from {pages_10k} at 10k modules to {pages_100k} at 100k"
        );
    }
}
