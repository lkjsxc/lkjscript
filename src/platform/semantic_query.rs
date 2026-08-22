//! Revision-pinned, deterministic, bounded semantic owner and relation queries.

use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::graph::GraphRoot;
use super::meaning::{
    DeclarationKind, DocumentationOwner, GRAPH_CONTRACT_VERSION, MeaningModule, MemberIdentity,
    RelationRole, RelationSource, RelationTarget, SemanticRelation,
};
use super::package::PackageId;
use super::packed;
use super::repository::{DisposableIndexPart, RevisionSnapshot, SemanticRepository};
use super::semantic_digest::RootObjectDigest;
use super::semantic_id::{ModuleId, RevisionId};
use base64::Engine;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const QUERY_CONTRACT_VERSION: u16 = 1;
pub const DEFAULT_ITEM_LIMIT: usize = 50;
pub const DEFAULT_BYTE_LIMIT: usize = 64 * 1024;
pub const DEFAULT_WORK_LIMIT: usize = 100_000;
pub const MAXIMUM_ITEM_LIMIT: usize = 10_000;
pub const MAXIMUM_BYTE_LIMIT: usize = 4 * 1_048_576;
pub const MAXIMUM_WORK_LIMIT: usize = 10_000_000;
pub const MAXIMUM_QUERY_DEPTH: usize = 32;
pub const MAXIMUM_QUERY_FANOUT: usize = 10_000;
pub const QUERY_INDEX_CONTRACT_VERSION: u16 = 1;
pub const MAXIMUM_QUERY_INDEX_BYTES: usize = 128 * 1_048_576;
pub const MAXIMUM_QUERY_INDEX_OWNERS: usize = 2_000_000;
pub const MAXIMUM_QUERY_INDEX_RELATIONS: usize = 10_000_000;
const QUERY_INDEX_MAGIC: [u8; 8] = *b"LKJIDX01";
const QUERY_INDEX_DOMAIN: &str = "lkjscript.semantic-query-index.v1";
const LOCAL_INDEX_CONTRACT_VERSION: u16 = 1;
const LOCAL_INDEX_BUCKETS: usize = 256;
const MAXIMUM_LOCAL_INDEX_PART_BYTES: usize = 16 * 1_048_576;
const LOCAL_MANIFEST_MAGIC: [u8; 8] = *b"LKJIXM01";
const LOCAL_OWNER_MAGIC: [u8; 8] = *b"LKJIXO01";
const LOCAL_NAME_MAGIC: [u8; 8] = *b"LKJIXN01";
const LOCAL_MANIFEST_DOMAIN: &str = "lkjscript.semantic-local-index-manifest.v1";
const LOCAL_OWNER_DOMAIN: &str = "lkjscript.semantic-local-owner-index.v1";
const LOCAL_NAME_DOMAIN: &str = "lkjscript.semantic-local-name-index.v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QueryBudget {
    pub maximum_items: usize,
    pub maximum_bytes: usize,
    pub maximum_work: usize,
    pub maximum_depth: usize,
    pub maximum_fanout: usize,
}

impl Default for QueryBudget {
    fn default() -> Self {
        Self {
            maximum_items: DEFAULT_ITEM_LIMIT,
            maximum_bytes: DEFAULT_BYTE_LIMIT,
            maximum_work: DEFAULT_WORK_LIMIT,
            maximum_depth: 4,
            maximum_fanout: 1_000,
        }
    }
}

impl QueryBudget {
    pub fn validate(self) -> Result<Self, Diagnostic> {
        if self.maximum_items == 0 || self.maximum_items > MAXIMUM_ITEM_LIMIT {
            return Err(query_error(
                DiagnosticClass::Resource,
                "semantic_query_item_limit",
                format!("item limit must be 1 through {MAXIMUM_ITEM_LIMIT}"),
            ));
        }
        if self.maximum_bytes < 512 || self.maximum_bytes > MAXIMUM_BYTE_LIMIT {
            return Err(query_error(
                DiagnosticClass::Resource,
                "semantic_query_byte_limit",
                format!("byte limit must be 512 through {MAXIMUM_BYTE_LIMIT}"),
            ));
        }
        if self.maximum_work == 0 || self.maximum_work > MAXIMUM_WORK_LIMIT {
            return Err(query_error(
                DiagnosticClass::Resource,
                "semantic_query_work_limit",
                format!("work limit must be 1 through {MAXIMUM_WORK_LIMIT}"),
            ));
        }
        if self.maximum_depth > MAXIMUM_QUERY_DEPTH
            || self.maximum_fanout == 0
            || self.maximum_fanout > MAXIMUM_QUERY_FANOUT
        {
            return Err(query_error(
                DiagnosticClass::Resource,
                "semantic_query_traversal_limit",
                format!(
                    "depth must not exceed {MAXIMUM_QUERY_DEPTH} and fanout must be 1 through {MAXIMUM_QUERY_FANOUT}"
                ),
            ));
        }
        Ok(self)
    }
}

#[derive(
    Decode, Encode, Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(rename_all = "snake_case")]
pub enum OwnerKind {
    Repository,
    Package,
    Module,
    Record,
    Variant,
    Interface,
    External,
    PureFunction,
    TaskFunction,
    Constant,
    Component,
    Test,
    Field,
    Case,
    Operation,
    Parameter,
    Binding,
    Expression,
    Requirement,
    Port,
    Target,
    Documentation,
    Annotation,
}

impl OwnerKind {
    pub fn parse(value: &str) -> Result<Self, Diagnostic> {
        match value {
            "repository" => Ok(Self::Repository),
            "package" => Ok(Self::Package),
            "module" => Ok(Self::Module),
            "record" => Ok(Self::Record),
            "variant" => Ok(Self::Variant),
            "interface" => Ok(Self::Interface),
            "external" => Ok(Self::External),
            "pure_function" | "function" => Ok(Self::PureFunction),
            "task_function" | "task" => Ok(Self::TaskFunction),
            "constant" => Ok(Self::Constant),
            "component" => Ok(Self::Component),
            "test" => Ok(Self::Test),
            "field" => Ok(Self::Field),
            "case" => Ok(Self::Case),
            "operation" => Ok(Self::Operation),
            "parameter" => Ok(Self::Parameter),
            "binding" => Ok(Self::Binding),
            "expression" => Ok(Self::Expression),
            "requirement" => Ok(Self::Requirement),
            "port" => Ok(Self::Port),
            "target" => Ok(Self::Target),
            "documentation" => Ok(Self::Documentation),
            "annotation" => Ok(Self::Annotation),
            _ => Err(query_error(
                DiagnosticClass::Source,
                "semantic_query_owner_kind",
                format!("unknown semantic owner kind '{value}'"),
            )),
        }
    }
}

#[derive(Decode, Encode, Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerSummary {
    pub kind: OwnerKind,
    pub id: String,
    pub name: String,
    pub qualified_name: String,
    pub package_id: PackageId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_id: Option<ModuleId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerDetail {
    #[serde(flatten)]
    pub owner: OwnerSummary,
    pub semantic: JsonValue,
}

#[derive(Decode, Encode, Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RelationView {
    pub source: String,
    pub target: String,
    pub role: RelationRole,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryStatus {
    Match,
    NoMatch,
    Ambiguous,
    Truncated,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QueryPage<T: Serialize> {
    pub contract_version: u16,
    pub revision: RevisionId,
    pub status: QueryStatus,
    pub query_digest: String,
    pub returned_items: usize,
    pub total_items: usize,
    pub work: usize,
    pub truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continuation: Option<String>,
    pub items: Vec<T>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContextItem {
    pub owner: OwnerSummary,
    pub reason: String,
    pub depth: usize,
}

#[derive(Decode, Encode, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct IndexedModuleFacts {
    id: ModuleId,
    declarations: u64,
    exports: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleQueryFacts {
    pub id: ModuleId,
    pub name: String,
    pub declarations: usize,
    pub exports: usize,
}

#[derive(Clone, Debug)]
struct OwnerRecord {
    summary: OwnerSummary,
    semantic: Option<JsonValue>,
}

#[derive(Decode, Encode, Clone, Debug, Eq, PartialEq)]
struct PackedQueryIndex {
    contract_version: u16,
    graph_contract_version: u16,
    revision: RevisionId,
    root: RootObjectDigest,
    package_id: PackageId,
    owners: Vec<OwnerSummary>,
    relations: Vec<RelationView>,
    modules: Vec<IndexedModuleFacts>,
}

#[derive(Decode, Encode, Clone, Debug, Eq, PartialEq)]
struct PackedLocalIndexManifest {
    contract_version: u16,
    graph_contract_version: u16,
    repository_id: super::semantic_id::RepositoryId,
    revision: RevisionId,
    root: RootObjectDigest,
    package_id: PackageId,
    owner_count: u64,
    owner_buckets: [u8; 32],
    name_buckets: [u8; 32],
}

#[derive(Decode, Encode, Clone, Debug, Eq, PartialEq)]
struct PackedLocalOwnerShard {
    contract_version: u16,
    revision: RevisionId,
    root: RootObjectDigest,
    bucket: u8,
    owners: Vec<OwnerSummary>,
}

#[derive(Decode, Encode, Clone, Debug, Eq, PartialEq)]
struct PackedNameEntry {
    value: String,
    owners: Vec<String>,
}

#[derive(Decode, Encode, Clone, Debug, Eq, PartialEq)]
struct PackedLocalNameShard {
    contract_version: u16,
    revision: RevisionId,
    root: RootObjectDigest,
    bucket: u8,
    names: Vec<PackedNameEntry>,
    qualified_names: Vec<PackedNameEntry>,
}

#[derive(Clone, Debug)]
pub struct SemanticQueryIndex {
    revision: RevisionId,
    package_id: PackageId,
    root: GraphRoot,
    repository: Option<SemanticRepository>,
    owners: BTreeMap<String, OwnerRecord>,
    by_name: BTreeMap<String, Vec<String>>,
    by_qualified_name: BTreeMap<String, Vec<String>>,
    relations: Vec<RelationView>,
    incoming: BTreeMap<String, Vec<RelationView>>,
    outgoing: BTreeMap<String, Vec<RelationView>>,
    modules: BTreeMap<ModuleId, (usize, usize)>,
    rebuilt: bool,
}

impl SemanticQueryIndex {
    pub fn current(repository: &SemanticRepository) -> Result<Self, Diagnostic> {
        Self::load_or_rebuild(repository, repository.current()?.head.revision)
    }

    pub fn revision(
        repository: &SemanticRepository,
        revision: RevisionId,
    ) -> Result<Self, Diagnostic> {
        Self::load_or_rebuild(repository, revision)
    }

    pub fn from_snapshot(snapshot: RevisionSnapshot) -> Result<Self, Diagnostic> {
        Self::from_parts(snapshot.record.revision, snapshot.root, snapshot.modules)
    }

    pub fn revision_id(&self) -> RevisionId {
        self.revision
    }

    pub fn rebuilt_index(&self) -> bool {
        self.rebuilt
    }

    pub fn owner_count(&self) -> usize {
        self.owners.len()
    }

    pub fn module_facts(&self) -> Result<Vec<ModuleQueryFacts>, Diagnostic> {
        self.modules
            .iter()
            .map(|(id, (declarations, exports))| {
                let owner = self
                    .owners
                    .get(&id.to_string())
                    .ok_or_else(index_binding_error)?;
                if owner.summary.kind != OwnerKind::Module {
                    return Err(index_binding_error());
                }
                Ok(ModuleQueryFacts {
                    id: *id,
                    name: owner.summary.name.clone(),
                    declarations: *declarations,
                    exports: *exports,
                })
            })
            .collect()
    }

    pub(crate) fn owner_states(
        &self,
    ) -> Result<BTreeMap<String, (OwnerSummary, JsonValue)>, Diagnostic> {
        self.owners
            .iter()
            .map(|(id, owner)| {
                Ok((
                    id.clone(),
                    (owner.summary.clone(), self.owner_semantic(owner)?.clone()),
                ))
            })
            .collect()
    }

    fn load_or_rebuild(
        repository: &SemanticRepository,
        revision: RevisionId,
    ) -> Result<Self, Diagnostic> {
        let record = repository.read_revision(revision)?;
        let root = repository.read_root(record.core.root)?;
        let cached = repository
            .read_index_object(revision, MAXIMUM_QUERY_INDEX_BYTES + 50)?
            .and_then(|bytes| PackedQueryIndex::decode(&bytes).ok())
            .filter(|index| {
                index.revision == revision
                    && index.root == record.core.root
                    && index.package_id == root.package_id
            });
        if let Some(cached) = cached {
            return Self::from_packed(repository.clone(), root, cached);
        }

        let snapshot = repository.reconstruct_revision(revision)?;
        let mut index = Self::from_snapshot(snapshot)?;
        let bytes = index.packed(record.core.root)?.encode()?;
        repository.write_index_object(revision, &bytes, MAXIMUM_QUERY_INDEX_BYTES + 50)?;
        index.repository = Some(repository.clone());
        index.rebuilt = true;
        Ok(index)
    }

    fn from_packed(
        repository: SemanticRepository,
        root: GraphRoot,
        packed: PackedQueryIndex,
    ) -> Result<Self, Diagnostic> {
        packed.validate()?;
        let owners = packed
            .owners
            .into_iter()
            .map(|summary| {
                (
                    summary.id.clone(),
                    OwnerRecord {
                        summary,
                        semantic: None,
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();
        let (by_name, by_qualified_name) = owner_name_indexes(&owners);
        let (incoming, outgoing) = adjacency(&packed.relations);
        let modules = packed
            .modules
            .iter()
            .map(|module| {
                Ok((
                    module.id,
                    (
                        usize::try_from(module.declarations).map_err(|_| index_binding_error())?,
                        usize::try_from(module.exports).map_err(|_| index_binding_error())?,
                    ),
                ))
            })
            .collect::<Result<BTreeMap<_, _>, Diagnostic>>()?;
        Ok(Self {
            revision: packed.revision,
            package_id: packed.package_id,
            root,
            repository: Some(repository),
            owners,
            by_name,
            by_qualified_name,
            relations: packed.relations,
            incoming,
            outgoing,
            modules,
            rebuilt: false,
        })
    }

    fn from_parts(
        revision: RevisionId,
        root: GraphRoot,
        modules: Vec<MeaningModule>,
    ) -> Result<Self, Diagnostic> {
        root.validate_modules(&modules)?;
        let module_facts = modules
            .iter()
            .map(|module| {
                (
                    module.module_id,
                    (module.declarations.len(), module.module.exports.len()),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let mut owners = BTreeMap::new();
        insert_owner(
            &mut owners,
            OwnerSummary {
                kind: OwnerKind::Repository,
                id: root.repository_id.to_string(),
                name: root.package_name.clone(),
                qualified_name: root.package_name.clone(),
                package_id: root.package_id.clone(),
                module_id: None,
                parent_id: None,
            },
            json!({
                "graph_contract_version": root.graph_contract_version,
                "revision": revision,
            }),
        )?;
        insert_owner(
            &mut owners,
            OwnerSummary {
                kind: OwnerKind::Package,
                id: format!("pkg_{}", root.package_id.as_str()),
                name: root.package_name.clone(),
                qualified_name: root.package_name.clone(),
                package_id: root.package_id.clone(),
                module_id: None,
                parent_id: Some(root.repository_id.to_string()),
            },
            json!({
                "dependencies": root.dependencies,
                "targets": root.targets,
            }),
        )?;

        let mut relations = Vec::new();
        for module in &modules {
            index_module(&root, module, &mut owners, &mut relations)?;
        }
        for target in &root.targets {
            let id = target.id.to_string();
            insert_owner(
                &mut owners,
                OwnerSummary {
                    kind: OwnerKind::Target,
                    id: id.clone(),
                    name: target.name.clone(),
                    qualified_name: format!("{}::target::{}", root.package_name, target.name),
                    package_id: root.package_id.clone(),
                    module_id: Some(target.component_module),
                    parent_id: Some(format!("pkg_{}", root.package_id.as_str())),
                },
                semantic_json(target)?,
            )?;
            relations.push(RelationView {
                source: id.clone(),
                target: target.component.to_string(),
                role: RelationRole::TargetComponent,
            });
            relations.push(RelationView {
                source: id,
                target: target.port.to_string(),
                role: RelationRole::TargetPort,
            });
        }
        relations.sort_by(|left, right| {
            (&left.source, &left.target, left.role).cmp(&(&right.source, &right.target, right.role))
        });
        relations.dedup();
        let (incoming, outgoing) = adjacency(&relations);
        let (by_name, by_qualified_name) = owner_name_indexes(&owners);
        Ok(Self {
            revision,
            package_id: root.package_id.clone(),
            root,
            repository: None,
            owners,
            by_name,
            by_qualified_name,
            relations,
            incoming,
            outgoing,
            modules: module_facts,
            rebuilt: false,
        })
    }

    pub fn exact_find_revision(
        repository: &SemanticRepository,
        revision: RevisionId,
        text: &str,
        continuation: Option<&str>,
        budget: QueryBudget,
    ) -> Result<QueryPage<OwnerSummary>, Diagnostic> {
        let budget = budget.validate()?;
        if text.is_empty() || text.len() > 4_096 {
            return Err(query_error(
                DiagnosticClass::Source,
                "semantic_query_text",
                "find text must contain 1 through 4096 bytes",
            ));
        }
        let manifest = Self::ensure_local_index(repository, revision)?;
        let mut ids = BTreeSet::new();
        let owner_bucket = local_bucket("owner", text);
        if let Some(shard) = Self::local_owner_shard(repository, &manifest, owner_bucket)?
            && shard.owners.iter().any(|owner| owner.id == text)
        {
            ids.insert(text.to_owned());
        }
        let name_bucket = local_bucket("name", text);
        if let Some(shard) = Self::local_name_shard(repository, &manifest, name_bucket)? {
            if let Ok(index) = shard
                .names
                .binary_search_by(|entry| entry.value.as_str().cmp(text))
            {
                ids.extend(shard.names[index].owners.iter().cloned());
            }
            if let Ok(index) = shard
                .qualified_names
                .binary_search_by(|entry| entry.value.as_str().cmp(text))
            {
                ids.extend(shard.qualified_names[index].owners.iter().cloned());
            }
        }
        let mut loaded = BTreeMap::<u8, PackedLocalOwnerShard>::new();
        let mut values = Vec::with_capacity(ids.len());
        let mut work = 0usize;
        for id in ids {
            charge_work(&mut work, budget.maximum_work)?;
            let bucket = local_bucket("owner", &id);
            if let std::collections::btree_map::Entry::Vacant(entry) = loaded.entry(bucket) {
                let shard = Self::local_owner_shard(repository, &manifest, bucket)?
                    .ok_or_else(index_binding_error)?;
                entry.insert(shard);
            }
            let owner = loaded[&bucket]
                .owners
                .binary_search_by(|owner| owner.id.cmp(&id))
                .ok()
                .and_then(|index| loaded[&bucket].owners.get(index))
                .ok_or_else(index_binding_error)?;
            values.push(owner.clone());
        }
        let ambiguous = values.len() > 1;
        page_values(
            revision,
            format!("find:true:{text}").as_bytes(),
            values,
            continuation,
            budget,
            ambiguous,
            work,
        )
    }

    pub fn show_revision(
        repository: &SemanticRepository,
        revision: RevisionId,
        id: &str,
        include_body: bool,
    ) -> Result<OwnerDetail, Diagnostic> {
        if id.is_empty() || id.len() > 4_096 {
            return Err(query_error(
                DiagnosticClass::Source,
                "semantic_owner_id",
                "owner identity must contain 1 through 4096 bytes",
            ));
        }
        let manifest = Self::ensure_local_index(repository, revision)?;
        let bucket = local_bucket("owner", id);
        let summary = Self::local_owner_shard(repository, &manifest, bucket)?
            .and_then(|shard| {
                shard
                    .owners
                    .binary_search_by(|owner| owner.id.as_str().cmp(id))
                    .ok()
                    .and_then(|index| shard.owners.get(index).cloned())
            })
            .ok_or_else(|| {
                query_error(
                    DiagnosticClass::Source,
                    "semantic_owner_missing",
                    format!("revision {revision} has no owner '{id}'"),
                )
            })?;
        if !include_body {
            return Ok(OwnerDetail {
                owner: summary,
                semantic: JsonValue::Object(Default::default()),
            });
        }
        let record = repository.read_revision(revision)?;
        let root = repository.read_root(record.core.root)?;
        let owner = OwnerRecord {
            summary: summary.clone(),
            semantic: None,
        };
        let owners = BTreeMap::from([(summary.id.clone(), owner)]);
        let (by_name, by_qualified_name) = owner_name_indexes(&owners);
        let index = Self {
            revision,
            package_id: root.package_id.clone(),
            root,
            repository: Some(repository.clone()),
            owners,
            by_name,
            by_qualified_name,
            relations: Vec::new(),
            incoming: BTreeMap::new(),
            outgoing: BTreeMap::new(),
            modules: BTreeMap::new(),
            rebuilt: false,
        };
        index.show(id, true)
    }

    fn ensure_local_index(
        repository: &SemanticRepository,
        revision: RevisionId,
    ) -> Result<PackedLocalIndexManifest, Diagnostic> {
        let record = repository.read_revision(revision)?;
        let root = repository.read_root(record.core.root)?;
        let cached = repository
            .read_index_part(revision, DisposableIndexPart::Manifest, 64 * 1024 + 50)?
            .and_then(|bytes| PackedLocalIndexManifest::decode(&bytes).ok())
            .filter(|manifest| {
                manifest.repository_id == record.core.repository_id
                    && manifest.revision == revision
                    && manifest.root == record.core.root
                    && manifest.package_id == root.package_id
            });
        match cached {
            Some(manifest) => Ok(manifest),
            None => Self::rebuild_local_index(repository, revision, record.core.root),
        }
    }

    fn rebuild_local_index(
        repository: &SemanticRepository,
        revision: RevisionId,
        root: RootObjectDigest,
    ) -> Result<PackedLocalIndexManifest, Diagnostic> {
        let index = Self::load_or_rebuild(repository, revision)?;
        index.write_local_index(repository, root)
    }

    fn write_local_index(
        &self,
        repository: &SemanticRepository,
        root: RootObjectDigest,
    ) -> Result<PackedLocalIndexManifest, Diagnostic> {
        let mut owner_shards = (0..LOCAL_INDEX_BUCKETS)
            .map(|_| Vec::<OwnerSummary>::new())
            .collect::<Vec<_>>();
        let mut name_shards = (0..LOCAL_INDEX_BUCKETS)
            .map(|_| BTreeMap::<String, Vec<String>>::new())
            .collect::<Vec<_>>();
        let mut qualified_name_shards = (0..LOCAL_INDEX_BUCKETS)
            .map(|_| BTreeMap::<String, Vec<String>>::new())
            .collect::<Vec<_>>();
        for owner in self.owners.values() {
            let summary = &owner.summary;
            owner_shards[local_bucket("owner", &summary.id) as usize].push(summary.clone());
            name_shards[local_bucket("name", &summary.name) as usize]
                .entry(summary.name.clone())
                .or_default()
                .push(summary.id.clone());
            qualified_name_shards[local_bucket("name", &summary.qualified_name) as usize]
                .entry(summary.qualified_name.clone())
                .or_default()
                .push(summary.id.clone());
        }
        let mut owner_buckets = [0u8; 32];
        let mut name_buckets = [0u8; 32];
        for bucket in 0..LOCAL_INDEX_BUCKETS {
            if !owner_shards[bucket].is_empty() {
                mark_bucket(&mut owner_buckets, bucket as u8);
                let shard = PackedLocalOwnerShard {
                    contract_version: LOCAL_INDEX_CONTRACT_VERSION,
                    revision: self.revision,
                    root,
                    bucket: bucket as u8,
                    owners: std::mem::take(&mut owner_shards[bucket]),
                };
                repository.write_index_part(
                    self.revision,
                    DisposableIndexPart::Owners(bucket as u8),
                    &shard.encode()?,
                    MAXIMUM_LOCAL_INDEX_PART_BYTES + 50,
                )?;
            }
            if !name_shards[bucket].is_empty() || !qualified_name_shards[bucket].is_empty() {
                mark_bucket(&mut name_buckets, bucket as u8);
                let shard = PackedLocalNameShard {
                    contract_version: LOCAL_INDEX_CONTRACT_VERSION,
                    revision: self.revision,
                    root,
                    bucket: bucket as u8,
                    names: name_entries(std::mem::take(&mut name_shards[bucket])),
                    qualified_names: name_entries(std::mem::take(
                        &mut qualified_name_shards[bucket],
                    )),
                };
                repository.write_index_part(
                    self.revision,
                    DisposableIndexPart::Names(bucket as u8),
                    &shard.encode()?,
                    MAXIMUM_LOCAL_INDEX_PART_BYTES + 50,
                )?;
            }
        }
        let manifest = PackedLocalIndexManifest {
            contract_version: LOCAL_INDEX_CONTRACT_VERSION,
            graph_contract_version: GRAPH_CONTRACT_VERSION,
            repository_id: self.root.repository_id,
            revision: self.revision,
            root,
            package_id: self.package_id.clone(),
            owner_count: u64::try_from(self.owners.len()).map_err(|_| index_binding_error())?,
            owner_buckets,
            name_buckets,
        };
        repository.write_index_part(
            self.revision,
            DisposableIndexPart::Manifest,
            &manifest.encode()?,
            64 * 1024 + 50,
        )?;
        Ok(manifest)
    }

    fn local_owner_shard(
        repository: &SemanticRepository,
        manifest: &PackedLocalIndexManifest,
        bucket: u8,
    ) -> Result<Option<PackedLocalOwnerShard>, Diagnostic> {
        if !bucket_marked(&manifest.owner_buckets, bucket) {
            return Ok(None);
        }
        let load = || -> Result<Option<PackedLocalOwnerShard>, Diagnostic> {
            Ok(repository
                .read_index_part(
                    manifest.revision,
                    DisposableIndexPart::Owners(bucket),
                    MAXIMUM_LOCAL_INDEX_PART_BYTES + 50,
                )?
                .and_then(|bytes| PackedLocalOwnerShard::decode(&bytes).ok())
                .filter(|shard| {
                    shard.revision == manifest.revision
                        && shard.root == manifest.root
                        && shard.bucket == bucket
                }))
        };
        if let Some(shard) = load()? {
            return Ok(Some(shard));
        }
        let rebuilt = Self::rebuild_local_index(repository, manifest.revision, manifest.root)?;
        if !bucket_marked(&rebuilt.owner_buckets, bucket) {
            return Err(index_binding_error());
        }
        load()?.map(Some).ok_or_else(index_binding_error)
    }

    fn local_name_shard(
        repository: &SemanticRepository,
        manifest: &PackedLocalIndexManifest,
        bucket: u8,
    ) -> Result<Option<PackedLocalNameShard>, Diagnostic> {
        if !bucket_marked(&manifest.name_buckets, bucket) {
            return Ok(None);
        }
        let load = || -> Result<Option<PackedLocalNameShard>, Diagnostic> {
            Ok(repository
                .read_index_part(
                    manifest.revision,
                    DisposableIndexPart::Names(bucket),
                    MAXIMUM_LOCAL_INDEX_PART_BYTES + 50,
                )?
                .and_then(|bytes| PackedLocalNameShard::decode(&bytes).ok())
                .filter(|shard| {
                    shard.revision == manifest.revision
                        && shard.root == manifest.root
                        && shard.bucket == bucket
                }))
        };
        if let Some(shard) = load()? {
            return Ok(Some(shard));
        }
        let rebuilt = Self::rebuild_local_index(repository, manifest.revision, manifest.root)?;
        if !bucket_marked(&rebuilt.name_buckets, bucket) {
            return Err(index_binding_error());
        }
        load()?.map(Some).ok_or_else(index_binding_error)
    }

    pub fn owners(
        &self,
        kind: Option<OwnerKind>,
        module: Option<ModuleId>,
        continuation: Option<&str>,
        budget: QueryBudget,
    ) -> Result<QueryPage<OwnerSummary>, Diagnostic> {
        let budget = budget.validate()?;
        let query = format!("owners:{kind:?}:{module:?}");
        let mut values = Vec::new();
        let mut work = 0usize;
        for owner in self.owners.values() {
            charge_work(&mut work, budget.maximum_work)?;
            if kind.is_none_or(|value| owner.summary.kind == value)
                && module.is_none_or(|value| owner.summary.module_id == Some(value))
            {
                values.push(owner.summary.clone());
            }
        }
        self.page_with_work(query.as_bytes(), values, continuation, budget, false, work)
    }

    pub fn find(
        &self,
        text: &str,
        exact: bool,
        continuation: Option<&str>,
        budget: QueryBudget,
    ) -> Result<QueryPage<OwnerSummary>, Diagnostic> {
        let budget = budget.validate()?;
        if text.is_empty() || text.len() > 4_096 {
            return Err(query_error(
                DiagnosticClass::Source,
                "semantic_query_text",
                "find text must contain 1 through 4096 bytes",
            ));
        }
        let query = format!("find:{exact}:{text}");
        let mut values = Vec::new();
        let mut work = 0usize;
        if exact {
            let ids = self
                .owners
                .get(text)
                .map(|owner| owner.summary.id.clone())
                .into_iter()
                .chain(self.by_name.get(text).into_iter().flatten().cloned())
                .chain(
                    self.by_qualified_name
                        .get(text)
                        .into_iter()
                        .flatten()
                        .cloned(),
                )
                .collect::<BTreeSet<_>>();
            for id in ids {
                charge_work(&mut work, budget.maximum_work)?;
                values.push(
                    self.owners
                        .get(&id)
                        .ok_or_else(index_binding_error)?
                        .summary
                        .clone(),
                );
            }
        } else {
            for owner in self.owners.values() {
                charge_work(&mut work, budget.maximum_work)?;
                if owner.summary.name.contains(text)
                    || owner.summary.qualified_name.contains(text)
                    || owner.summary.id.contains(text)
                {
                    values.push(owner.summary.clone());
                }
            }
        }
        let ambiguous = exact && values.len() > 1;
        self.page_with_work(
            query.as_bytes(),
            values,
            continuation,
            budget,
            ambiguous,
            work,
        )
    }

    pub fn show(&self, id: &str, include_body: bool) -> Result<OwnerDetail, Diagnostic> {
        let owner = self.owners.get(id).ok_or_else(|| {
            query_error(
                DiagnosticClass::Source,
                "semantic_owner_missing",
                format!("revision {} has no owner '{id}'", self.revision),
            )
        })?;
        Ok(OwnerDetail {
            owner: owner.summary.clone(),
            semantic: if include_body {
                self.owner_semantic(owner)?
            } else {
                JsonValue::Object(Default::default())
            },
        })
    }

    fn packed(&self, root: RootObjectDigest) -> Result<PackedQueryIndex, Diagnostic> {
        let value = PackedQueryIndex {
            contract_version: QUERY_INDEX_CONTRACT_VERSION,
            graph_contract_version: GRAPH_CONTRACT_VERSION,
            revision: self.revision,
            root,
            package_id: self.package_id.clone(),
            owners: self
                .owners
                .values()
                .map(|owner| owner.summary.clone())
                .collect(),
            relations: self.relations.clone(),
            modules: self
                .modules
                .iter()
                .map(|(id, (declarations, exports))| {
                    Ok(IndexedModuleFacts {
                        id: *id,
                        declarations: u64::try_from(*declarations)
                            .map_err(|_| index_binding_error())?,
                        exports: u64::try_from(*exports).map_err(|_| index_binding_error())?,
                    })
                })
                .collect::<Result<Vec<_>, Diagnostic>>()?,
        };
        value.validate()?;
        Ok(value)
    }

    fn owner_semantic(&self, owner: &OwnerRecord) -> Result<JsonValue, Diagnostic> {
        if let Some(value) = &owner.semantic {
            return Ok(value.clone());
        }
        match owner.summary.kind {
            OwnerKind::Repository => {
                return Ok(json!({
                    "graph_contract_version": self.root.graph_contract_version,
                    "revision": self.revision,
                }));
            }
            OwnerKind::Package => {
                return Ok(json!({
                    "dependencies": self.root.dependencies,
                    "targets": self.root.targets,
                }));
            }
            OwnerKind::Target => {
                let target = self
                    .root
                    .targets
                    .iter()
                    .find(|target| target.id.to_string() == owner.summary.id)
                    .ok_or_else(index_binding_error)?;
                return semantic_json(target);
            }
            _ => {}
        }

        let repository = self.repository.as_ref().ok_or_else(|| {
            query_error(
                DiagnosticClass::Infrastructure,
                "semantic_index_repository",
                "lazy semantic projection has no repository binding",
            )
        })?;
        let module_id = owner.summary.module_id.ok_or_else(index_binding_error)?;
        let module = repository.module_by_id(self.revision, module_id)?;
        if owner.summary.kind == OwnerKind::Module {
            if module.module_id.to_string() != owner.summary.id {
                return Err(index_binding_error());
            }
            return Ok(json!({
                "imports": module.module.imports,
                "exports": module.module.exports,
                "documentation": module.documentation,
                "annotations": module.annotations,
            }));
        }
        for (identity, declaration) in module.declarations.iter().zip(&module.module.declarations) {
            if identity.id.to_string() == owner.summary.id {
                return semantic_json(declaration);
            }
            if let Some(member) = identity
                .members
                .iter()
                .find(|member| member_parts(member).1 == owner.summary.id)
            {
                return semantic_json(member);
            }
            if let Some(binding) = identity
                .bindings
                .iter()
                .find(|binding| binding.id.to_string() == owner.summary.id)
            {
                return semantic_json(binding);
            }
            if let Some(expression) = identity
                .expressions
                .iter()
                .find(|expression| expression.id.to_string() == owner.summary.id)
            {
                return semantic_json(expression);
            }
        }
        if let Some(documentation) = module
            .documentation
            .iter()
            .find(|value| value.id.to_string() == owner.summary.id)
        {
            return semantic_json(documentation);
        }
        if let Some(annotation) = module
            .annotations
            .iter()
            .find(|value| value.id.to_string() == owner.summary.id)
        {
            return semantic_json(annotation);
        }
        Err(index_binding_error())
    }

    pub fn relations(
        &self,
        id: &str,
        incoming: bool,
        outgoing: bool,
        roles: &BTreeSet<RelationRole>,
        continuation: Option<&str>,
        budget: QueryBudget,
    ) -> Result<QueryPage<RelationView>, Diagnostic> {
        let budget = budget.validate()?;
        if !self.owners.contains_key(id) {
            return Err(query_error(
                DiagnosticClass::Source,
                "semantic_owner_missing",
                format!("revision {} has no owner '{id}'", self.revision),
            ));
        }
        if !incoming && !outgoing {
            return Err(query_error(
                DiagnosticClass::Source,
                "semantic_query_direction",
                "relation query must select incoming, outgoing, or both",
            ));
        }
        let query = format!("relations:{id}:{incoming}:{outgoing}:{roles:?}");
        let mut values = Vec::new();
        let mut work = 0usize;
        if incoming {
            for edge in self.incoming.get(id).into_iter().flatten() {
                charge_work(&mut work, budget.maximum_work)?;
                if roles.is_empty() || roles.contains(&edge.role) {
                    values.push(edge.clone());
                }
            }
        }
        if outgoing {
            for edge in self.outgoing.get(id).into_iter().flatten() {
                charge_work(&mut work, budget.maximum_work)?;
                if roles.is_empty() || roles.contains(&edge.role) {
                    values.push(edge.clone());
                }
            }
        }
        values.sort_by(|left, right| {
            (&left.source, &left.target, left.role).cmp(&(&right.source, &right.target, right.role))
        });
        values.dedup();
        self.page_with_work(query.as_bytes(), values, continuation, budget, false, work)
    }

    pub fn context(
        &self,
        seeds: &[String],
        continuation: Option<&str>,
        budget: QueryBudget,
    ) -> Result<QueryPage<ContextItem>, Diagnostic> {
        let budget = budget.validate()?;
        if seeds.is_empty() || seeds.len() > budget.maximum_fanout {
            return Err(query_error(
                DiagnosticClass::Resource,
                "semantic_context_seed_limit",
                "context requires a bounded non-empty seed set",
            ));
        }
        let mut queue = VecDeque::new();
        let mut visited = BTreeMap::<String, (usize, String)>::new();
        for seed in seeds {
            if !self.owners.contains_key(seed) {
                return Err(query_error(
                    DiagnosticClass::Source,
                    "semantic_owner_missing",
                    format!("revision {} has no context seed '{seed}'", self.revision),
                ));
            }
            if visited
                .insert(seed.clone(), (0, "explicit_seed".to_owned()))
                .is_none()
            {
                queue.push_back(seed.clone());
            }
        }
        let mut work = 0usize;
        while let Some(owner) = queue.pop_front() {
            let depth = visited.get(&owner).map_or(0, |entry| entry.0);
            if depth >= budget.maximum_depth {
                continue;
            }
            let mut adjacent = Vec::new();
            adjacent.extend(self.outgoing.get(&owner).into_iter().flatten().map(|edge| {
                (
                    edge.target.clone(),
                    format!("outgoing_{:?}", edge.role).to_lowercase(),
                    edge.clone(),
                )
            }));
            adjacent.extend(self.incoming.get(&owner).into_iter().flatten().map(|edge| {
                (
                    edge.source.clone(),
                    format!("incoming_{:?}", edge.role).to_lowercase(),
                    edge.clone(),
                )
            }));
            adjacent.sort_by(|left, right| {
                (
                    &left.0,
                    &left.1,
                    &left.2.source,
                    &left.2.target,
                    left.2.role,
                )
                    .cmp(&(
                        &right.0,
                        &right.1,
                        &right.2.source,
                        &right.2.target,
                        right.2.role,
                    ))
            });
            adjacent.dedup();
            for (fanout, (next, reason, _)) in adjacent.into_iter().enumerate() {
                if fanout >= budget.maximum_fanout {
                    break;
                }
                charge_work(&mut work, budget.maximum_work)?;
                if self.owners.contains_key(&next) && !visited.contains_key(&next) {
                    visited.insert(next.clone(), (depth + 1, reason));
                    queue.push_back(next);
                }
            }
        }
        let mut values = visited
            .into_iter()
            .filter_map(|(id, (depth, reason))| {
                self.owners.get(&id).map(|owner| ContextItem {
                    owner: owner.summary.clone(),
                    reason,
                    depth,
                })
            })
            .collect::<Vec<_>>();
        values.sort_by(|left, right| {
            (left.depth, &left.owner.qualified_name, &left.owner.id).cmp(&(
                right.depth,
                &right.owner.qualified_name,
                &right.owner.id,
            ))
        });
        let query = format!("context:{}:{}", budget.maximum_depth, seeds.join(","));
        self.page_with_work(query.as_bytes(), values, continuation, budget, false, work)
    }

    pub fn impact(
        &self,
        seeds: &[String],
        continuation: Option<&str>,
        mut budget: QueryBudget,
    ) -> Result<QueryPage<ContextItem>, Diagnostic> {
        budget.maximum_depth = budget.maximum_depth.max(1);
        self.context(seeds, continuation, budget)
    }

    pub fn package_id(&self) -> &PackageId {
        &self.package_id
    }

    fn page_with_work<T: Clone + Serialize>(
        &self,
        query: &[u8],
        values: Vec<T>,
        continuation: Option<&str>,
        budget: QueryBudget,
        ambiguous: bool,
        work: usize,
    ) -> Result<QueryPage<T>, Diagnostic> {
        page_values(
            self.revision,
            query,
            values,
            continuation,
            budget,
            ambiguous,
            work,
        )
    }
}

impl PackedQueryIndex {
    fn encode(&self) -> Result<Vec<u8>, Diagnostic> {
        self.validate()?;
        packed::encode(
            QUERY_INDEX_MAGIC,
            QUERY_INDEX_DOMAIN,
            self,
            MAXIMUM_QUERY_INDEX_BYTES,
        )
    }

    fn decode(bytes: &[u8]) -> Result<Self, Diagnostic> {
        let value: Self = packed::decode(
            bytes,
            QUERY_INDEX_MAGIC,
            QUERY_INDEX_DOMAIN,
            MAXIMUM_QUERY_INDEX_BYTES,
        )?;
        value.validate()?;
        Ok(value)
    }

    fn validate(&self) -> Result<(), Diagnostic> {
        if self.contract_version != QUERY_INDEX_CONTRACT_VERSION
            || self.graph_contract_version != GRAPH_CONTRACT_VERSION
        {
            return Err(query_error(
                DiagnosticClass::Source,
                "semantic_index_contract",
                "query index uses an unknown index or graph contract",
            ));
        }
        if self.owners.is_empty()
            || self.owners.len() > MAXIMUM_QUERY_INDEX_OWNERS
            || self.relations.len() > MAXIMUM_QUERY_INDEX_RELATIONS
            || self.modules.len() > super::graph::MAXIMUM_ROOT_MODULES
        {
            return Err(query_error(
                DiagnosticClass::Resource,
                "semantic_index_item_limit",
                "query index owner or relation count exceeds its hard bound",
            ));
        }
        if self.owners.windows(2).any(|pair| pair[0].id >= pair[1].id)
            || self.relations.windows(2).any(|pair| pair[0] >= pair[1])
            || self.modules.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(query_error(
                DiagnosticClass::Corrupt,
                "semantic_index_order",
                "query index owners, relations, or modules are not uniquely and canonically ordered",
            ));
        }
        if self
            .owners
            .iter()
            .any(|owner| owner.package_id != self.package_id)
        {
            return Err(query_error(
                DiagnosticClass::Corrupt,
                "semantic_index_package",
                "query index owner belongs to a foreign package",
            ));
        }
        Ok(())
    }
}

impl PackedLocalIndexManifest {
    fn encode(&self) -> Result<Vec<u8>, Diagnostic> {
        self.validate()?;
        packed::encode(LOCAL_MANIFEST_MAGIC, LOCAL_MANIFEST_DOMAIN, self, 64 * 1024)
    }

    fn decode(bytes: &[u8]) -> Result<Self, Diagnostic> {
        let value: Self = packed::decode(
            bytes,
            LOCAL_MANIFEST_MAGIC,
            LOCAL_MANIFEST_DOMAIN,
            64 * 1024,
        )?;
        value.validate()?;
        Ok(value)
    }

    fn validate(&self) -> Result<(), Diagnostic> {
        if self.contract_version != LOCAL_INDEX_CONTRACT_VERSION
            || self.graph_contract_version != GRAPH_CONTRACT_VERSION
            || self.owner_count == 0
            || self.owner_count > MAXIMUM_QUERY_INDEX_OWNERS as u64
        {
            return Err(local_index_error("local index manifest is malformed"));
        }
        Ok(())
    }
}

impl PackedLocalOwnerShard {
    fn encode(&self) -> Result<Vec<u8>, Diagnostic> {
        self.validate()?;
        packed::encode(
            LOCAL_OWNER_MAGIC,
            LOCAL_OWNER_DOMAIN,
            self,
            MAXIMUM_LOCAL_INDEX_PART_BYTES,
        )
    }

    fn decode(bytes: &[u8]) -> Result<Self, Diagnostic> {
        let value: Self = packed::decode(
            bytes,
            LOCAL_OWNER_MAGIC,
            LOCAL_OWNER_DOMAIN,
            MAXIMUM_LOCAL_INDEX_PART_BYTES,
        )?;
        value.validate()?;
        Ok(value)
    }

    fn validate(&self) -> Result<(), Diagnostic> {
        if self.contract_version != LOCAL_INDEX_CONTRACT_VERSION
            || self.owners.len() > MAXIMUM_QUERY_INDEX_OWNERS
            || self.owners.windows(2).any(|pair| pair[0].id >= pair[1].id)
            || self
                .owners
                .iter()
                .any(|owner| local_bucket("owner", &owner.id) != self.bucket)
        {
            return Err(local_index_error("local owner index shard is malformed"));
        }
        Ok(())
    }
}

impl PackedLocalNameShard {
    fn encode(&self) -> Result<Vec<u8>, Diagnostic> {
        self.validate()?;
        packed::encode(
            LOCAL_NAME_MAGIC,
            LOCAL_NAME_DOMAIN,
            self,
            MAXIMUM_LOCAL_INDEX_PART_BYTES,
        )
    }

    fn decode(bytes: &[u8]) -> Result<Self, Diagnostic> {
        let value: Self = packed::decode(
            bytes,
            LOCAL_NAME_MAGIC,
            LOCAL_NAME_DOMAIN,
            MAXIMUM_LOCAL_INDEX_PART_BYTES,
        )?;
        value.validate()?;
        Ok(value)
    }

    fn validate(&self) -> Result<(), Diagnostic> {
        if self.contract_version != LOCAL_INDEX_CONTRACT_VERSION
            || !valid_name_entries(&self.names, self.bucket)
            || !valid_name_entries(&self.qualified_names, self.bucket)
        {
            return Err(local_index_error("local name index shard is malformed"));
        }
        Ok(())
    }
}

fn valid_name_entries(entries: &[PackedNameEntry], bucket: u8) -> bool {
    entries.len() <= MAXIMUM_QUERY_INDEX_OWNERS
        && !entries
            .windows(2)
            .any(|pair| pair[0].value >= pair[1].value)
        && entries.iter().all(|entry| {
            !entry.value.is_empty()
                && entry.value.len() <= 4_096
                && local_bucket("name", &entry.value) == bucket
                && !entry.owners.is_empty()
                && !entry.owners.windows(2).any(|pair| pair[0] >= pair[1])
        })
}

fn local_index_error(message: &str) -> Diagnostic {
    query_error(DiagnosticClass::Corrupt, "semantic_local_index", message)
}

fn local_bucket(domain: &str, value: &str) -> u8 {
    let mut hasher = blake3::Hasher::new_derive_key("lkjscript.semantic-local-index-bucket.v1");
    hasher.update(&(domain.len() as u64).to_be_bytes());
    hasher.update(domain.as_bytes());
    hasher.update(&(value.len() as u64).to_be_bytes());
    hasher.update(value.as_bytes());
    hasher.finalize().as_bytes()[0]
}

fn mark_bucket(bitmap: &mut [u8; 32], bucket: u8) {
    bitmap[(bucket / 8) as usize] |= 1 << (bucket % 8);
}

fn bucket_marked(bitmap: &[u8; 32], bucket: u8) -> bool {
    bitmap[(bucket / 8) as usize] & (1 << (bucket % 8)) != 0
}

fn name_entries(values: BTreeMap<String, Vec<String>>) -> Vec<PackedNameEntry> {
    values
        .into_iter()
        .map(|(value, mut owners)| {
            owners.sort();
            owners.dedup();
            PackedNameEntry { value, owners }
        })
        .collect()
}

fn page_values<T: Clone + Serialize>(
    revision: RevisionId,
    query: &[u8],
    values: Vec<T>,
    continuation: Option<&str>,
    budget: QueryBudget,
    ambiguous: bool,
    work: usize,
) -> Result<QueryPage<T>, Diagnostic> {
    let digest = query_digest(query);
    let start = continuation
        .map(|handle| decode_continuation(handle, revision, digest))
        .transpose()?
        .unwrap_or(0);
    if start > values.len() {
        return Err(query_error(
            DiagnosticClass::Source,
            "semantic_continuation_cursor",
            "continuation cursor is beyond the exact result",
        ));
    }
    let end = start.saturating_add(budget.maximum_items).min(values.len());
    let mut items = values[start..end].to_vec();
    while !items.is_empty()
        && serde_json::to_vec(&items)
            .map_err(query_json)?
            .len()
            .saturating_add(512)
            > budget.maximum_bytes
    {
        items.pop();
    }
    if items.is_empty() && start < values.len() {
        return Err(query_error(
            DiagnosticClass::Resource,
            "semantic_query_byte_exhausted",
            "byte budget cannot contain the next complete semantic item",
        ));
    }
    let next = start + items.len();
    let truncated = next < values.len();
    let status = if truncated {
        QueryStatus::Truncated
    } else if values.is_empty() {
        QueryStatus::NoMatch
    } else if ambiguous {
        QueryStatus::Ambiguous
    } else {
        QueryStatus::Match
    };
    Ok(QueryPage {
        contract_version: QUERY_CONTRACT_VERSION,
        revision,
        status,
        query_digest: hex_digest(digest),
        returned_items: items.len(),
        total_items: values.len(),
        work,
        truncated,
        continuation: truncated.then(|| encode_continuation(revision, digest, next)),
        items,
    })
}

fn adjacency(
    relations: &[RelationView],
) -> (
    BTreeMap<String, Vec<RelationView>>,
    BTreeMap<String, Vec<RelationView>>,
) {
    let mut incoming = BTreeMap::<String, Vec<RelationView>>::new();
    let mut outgoing = BTreeMap::<String, Vec<RelationView>>::new();
    for relation in relations {
        incoming
            .entry(relation.target.clone())
            .or_default()
            .push(relation.clone());
        outgoing
            .entry(relation.source.clone())
            .or_default()
            .push(relation.clone());
    }
    (incoming, outgoing)
}

fn owner_name_indexes(
    owners: &BTreeMap<String, OwnerRecord>,
) -> (BTreeMap<String, Vec<String>>, BTreeMap<String, Vec<String>>) {
    let mut by_name = BTreeMap::<String, Vec<String>>::new();
    let mut by_qualified_name = BTreeMap::<String, Vec<String>>::new();
    for owner in owners.values() {
        by_name
            .entry(owner.summary.name.clone())
            .or_default()
            .push(owner.summary.id.clone());
        by_qualified_name
            .entry(owner.summary.qualified_name.clone())
            .or_default()
            .push(owner.summary.id.clone());
    }
    (by_name, by_qualified_name)
}

fn index_binding_error() -> Diagnostic {
    query_error(
        DiagnosticClass::Corrupt,
        "semantic_index_binding",
        "disposable query index does not bind an exact canonical semantic owner",
    )
}

fn index_module(
    root: &GraphRoot,
    module: &MeaningModule,
    owners: &mut BTreeMap<String, OwnerRecord>,
    relations: &mut Vec<RelationView>,
) -> Result<(), Diagnostic> {
    let module_id = module.module_id.to_string();
    insert_owner(
        owners,
        OwnerSummary {
            kind: OwnerKind::Module,
            id: module_id.clone(),
            name: module.module.name.clone(),
            qualified_name: format!("{}::{}", root.package_name, module.module.name),
            package_id: root.package_id.clone(),
            module_id: Some(module.module_id),
            parent_id: Some(format!("pkg_{}", root.package_id.as_str())),
        },
        json!({
            "imports": module.module.imports,
            "exports": module.module.exports,
            "documentation": module.documentation,
            "annotations": module.annotations,
        }),
    )?;
    for (identity, declaration) in module.declarations.iter().zip(&module.module.declarations) {
        let declaration_id = identity.id.to_string();
        insert_owner(
            owners,
            OwnerSummary {
                kind: declaration_owner_kind(identity.kind),
                id: declaration_id.clone(),
                name: identity.name.clone(),
                qualified_name: format!(
                    "{}::{}::{}",
                    root.package_name, module.module.name, identity.name
                ),
                package_id: root.package_id.clone(),
                module_id: Some(module.module_id),
                parent_id: Some(module_id.clone()),
            },
            semantic_json(declaration)?,
        )?;
        for member in &identity.members {
            let (kind, id, name) = member_parts(member);
            insert_owner(
                owners,
                OwnerSummary {
                    kind,
                    id,
                    name: name.clone(),
                    qualified_name: format!(
                        "{}::{}::{}::{}",
                        root.package_name, module.module.name, identity.name, name
                    ),
                    package_id: root.package_id.clone(),
                    module_id: Some(module.module_id),
                    parent_id: Some(declaration_id.clone()),
                },
                semantic_json(member)?,
            )?;
        }
        for binding in &identity.bindings {
            insert_owner(
                owners,
                OwnerSummary {
                    kind: OwnerKind::Binding,
                    id: binding.id.to_string(),
                    name: binding.name.clone(),
                    qualified_name: format!(
                        "{}::{}::{}::binding::{}",
                        root.package_name, module.module.name, identity.name, binding.name
                    ),
                    package_id: root.package_id.clone(),
                    module_id: Some(module.module_id),
                    parent_id: Some(declaration_id.clone()),
                },
                semantic_json(binding)?,
            )?;
        }
        for expression in &identity.expressions {
            let path = expression
                .path
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(".");
            insert_owner(
                owners,
                OwnerSummary {
                    kind: OwnerKind::Expression,
                    id: expression.id.to_string(),
                    name: path.clone(),
                    qualified_name: format!(
                        "{}::{}::{}::expression::{}",
                        root.package_name, module.module.name, identity.name, path
                    ),
                    package_id: root.package_id.clone(),
                    module_id: Some(module.module_id),
                    parent_id: Some(declaration_id.clone()),
                },
                semantic_json(expression)?,
            )?;
        }
    }
    for documentation in &module.documentation {
        insert_owner(
            owners,
            OwnerSummary {
                kind: OwnerKind::Documentation,
                id: documentation.id.to_string(),
                name: "documentation".to_owned(),
                qualified_name: format!(
                    "{}::{}::documentation::{}",
                    root.package_name, module.module.name, documentation.id
                ),
                package_id: root.package_id.clone(),
                module_id: Some(module.module_id),
                parent_id: Some(documentation_owner_id(&documentation.owner)),
            },
            semantic_json(documentation)?,
        )?;
    }
    for annotation in &module.annotations {
        insert_owner(
            owners,
            OwnerSummary {
                kind: OwnerKind::Annotation,
                id: annotation.id.to_string(),
                name: annotation.key.clone(),
                qualified_name: format!(
                    "{}::{}::annotation::{}",
                    root.package_name, module.module.name, annotation.key
                ),
                package_id: root.package_id.clone(),
                module_id: Some(module.module_id),
                parent_id: Some(documentation_owner_id(&annotation.owner)),
            },
            semantic_json(annotation)?,
        )?;
    }
    relations.extend(module.relations.iter().map(relation_view));
    Ok(())
}

fn insert_owner(
    owners: &mut BTreeMap<String, OwnerRecord>,
    summary: OwnerSummary,
    semantic: JsonValue,
) -> Result<(), Diagnostic> {
    if owners
        .insert(
            summary.id.clone(),
            OwnerRecord {
                summary,
                semantic: Some(semantic),
            },
        )
        .is_some()
    {
        return Err(query_error(
            DiagnosticClass::Corrupt,
            "semantic_index_identity_duplicate",
            "two semantic owners have the same domain-qualified identity",
        ));
    }
    Ok(())
}

fn declaration_owner_kind(kind: DeclarationKind) -> OwnerKind {
    match kind {
        DeclarationKind::Record => OwnerKind::Record,
        DeclarationKind::Variant => OwnerKind::Variant,
        DeclarationKind::Interface => OwnerKind::Interface,
        DeclarationKind::External => OwnerKind::External,
        DeclarationKind::PureFunction => OwnerKind::PureFunction,
        DeclarationKind::TaskFunction => OwnerKind::TaskFunction,
        DeclarationKind::Constant => OwnerKind::Constant,
        DeclarationKind::Component => OwnerKind::Component,
        DeclarationKind::Test => OwnerKind::Test,
    }
}

fn member_parts(member: &MemberIdentity) -> (OwnerKind, String, String) {
    match member {
        MemberIdentity::Field { id, name } => (OwnerKind::Field, id.to_string(), name.clone()),
        MemberIdentity::Case { id, name } => (OwnerKind::Case, id.to_string(), name.clone()),
        MemberIdentity::Operation { id, name } => {
            (OwnerKind::Operation, id.to_string(), name.clone())
        }
        MemberIdentity::Parameter { id, name } => {
            (OwnerKind::Parameter, id.to_string(), name.clone())
        }
        MemberIdentity::TaskRequirement { id, name }
        | MemberIdentity::ComponentRequirement { id, name } => {
            (OwnerKind::Requirement, id.to_string(), name.clone())
        }
        MemberIdentity::Port { id, name } => (OwnerKind::Port, id.to_string(), name.clone()),
    }
}

fn relation_view(relation: &SemanticRelation) -> RelationView {
    RelationView {
        source: relation_source_id(&relation.source),
        target: relation_target_id(&relation.target),
        role: relation.role,
    }
}

fn relation_source_id(source: &RelationSource) -> String {
    match source {
        RelationSource::Module(id) => id.to_string(),
        RelationSource::Declaration(id) => id.to_string(),
        RelationSource::Field(id) => id.to_string(),
        RelationSource::Case(id) => id.to_string(),
        RelationSource::Operation(id) => id.to_string(),
        RelationSource::Parameter(id) => id.to_string(),
        RelationSource::Binding(id) => id.to_string(),
        RelationSource::Requirement(id) => id.to_string(),
        RelationSource::Port(id) => id.to_string(),
        RelationSource::Expression(id) => id.to_string(),
        RelationSource::Target(id) => id.to_string(),
    }
}

fn relation_target_id(target: &RelationTarget) -> String {
    match target {
        RelationTarget::Module(reference) => reference.module.to_string(),
        RelationTarget::Declaration(reference) => reference.declaration.to_string(),
        RelationTarget::Field { field, .. } => field.to_string(),
        RelationTarget::Case { case, .. } => case.to_string(),
        RelationTarget::Operation { operation, .. } => operation.to_string(),
        RelationTarget::Parameter { parameter, .. } => parameter.to_string(),
        RelationTarget::Binding { binding, .. } => binding.to_string(),
        RelationTarget::Requirement { requirement, .. } => requirement.to_string(),
        RelationTarget::Port { port, .. } => port.to_string(),
    }
}

fn documentation_owner_id(owner: &DocumentationOwner) -> String {
    match owner {
        DocumentationOwner::Module(id) => id.to_string(),
        DocumentationOwner::Declaration(id) => id.to_string(),
        DocumentationOwner::Field(id) => id.to_string(),
        DocumentationOwner::Case(id) => id.to_string(),
        DocumentationOwner::Operation(id) => id.to_string(),
        DocumentationOwner::Port(id) => id.to_string(),
    }
}

fn semantic_json(value: &impl Serialize) -> Result<JsonValue, Diagnostic> {
    let mut value = serde_json::to_value(value).map_err(query_json)?;
    remove_spans(&mut value);
    Ok(value)
}

fn remove_spans(value: &mut JsonValue) {
    match value {
        JsonValue::Object(values) => {
            values.remove("span");
            for value in values.values_mut() {
                remove_spans(value);
            }
        }
        JsonValue::Array(values) => {
            values.retain(|value| !is_source_span(value));
            for value in values {
                remove_spans(value);
            }
        }
        _ => {}
    }
}

fn is_source_span(value: &JsonValue) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    object.len() == 4
        && object.contains_key("byte_start")
        && object.contains_key("byte_end")
        && object.contains_key("line")
        && object.contains_key("column")
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ContinuationCore {
    contract_version: u16,
    revision: RevisionId,
    query_digest: String,
    cursor: usize,
}

fn encode_continuation(revision: RevisionId, query_digest: [u8; 32], cursor: usize) -> String {
    let core = ContinuationCore {
        contract_version: QUERY_CONTRACT_VERSION,
        revision,
        query_digest: hex_digest(query_digest),
        cursor,
    };
    let payload = serde_json::to_vec(&core).unwrap_or_default();
    let checksum = continuation_checksum(&payload);
    let mut bytes = payload;
    bytes.extend_from_slice(&checksum[..16]);
    format!(
        "cont_{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    )
}

fn decode_continuation(
    handle: &str,
    revision: RevisionId,
    query_digest: [u8; 32],
) -> Result<usize, Diagnostic> {
    let encoded = handle
        .strip_prefix("cont_")
        .ok_or_else(stale_continuation)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|_| stale_continuation())?;
    if bytes.len() < 17 || bytes.len() > 4_096 {
        return Err(stale_continuation());
    }
    let split = bytes.len() - 16;
    let (payload, checksum) = bytes.split_at(split);
    if continuation_checksum(payload)[..16] != *checksum {
        return Err(stale_continuation());
    }
    let mut deserializer = serde_json::Deserializer::from_slice(payload);
    let core =
        ContinuationCore::deserialize(&mut deserializer).map_err(|_| stale_continuation())?;
    deserializer.end().map_err(|_| stale_continuation())?;
    if core.contract_version != QUERY_CONTRACT_VERSION
        || core.revision != revision
        || core.query_digest != hex_digest(query_digest)
    {
        return Err(stale_continuation());
    }
    Ok(core.cursor)
}

fn continuation_checksum(payload: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key("lkjscript.semantic-continuation.v1");
    hasher.update(&(payload.len() as u64).to_be_bytes());
    hasher.update(payload);
    *hasher.finalize().as_bytes()
}

fn query_digest(query: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key("lkjscript.semantic-query.v1");
    hasher.update(&(query.len() as u64).to_be_bytes());
    hasher.update(query);
    *hasher.finalize().as_bytes()
}

fn hex_digest(bytes: [u8; 32]) -> String {
    super::semantic_id::encode_hex(&bytes)
}

fn work_exhausted() -> Diagnostic {
    query_error(
        DiagnosticClass::Resource,
        "semantic_query_work_exhausted",
        "semantic query exhausted its declared work budget",
    )
}

fn charge_work(work: &mut usize, maximum: usize) -> Result<(), Diagnostic> {
    *work = work.checked_add(1).ok_or_else(work_exhausted)?;
    if *work > maximum {
        return Err(work_exhausted());
    }
    Ok(())
}

fn stale_continuation() -> Diagnostic {
    query_error(
        DiagnosticClass::Source,
        "semantic_continuation_stale",
        "continuation is malformed, tampered, stale, or belongs to a different query",
    )
}

fn query_json(error: serde_json::Error) -> Diagnostic {
    query_error(
        DiagnosticClass::Infrastructure,
        "semantic_query_projection",
        format!("semantic query projection failed: {error}"),
    )
}

fn query_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{
        GRAPH_CONTRACT_VERSION, InitialPublication, MigrationIdentityAllocator, ModuleObjectRef,
        PackageId, RepositoryId, SemanticDiffDigest, SourceLimits, TransactionDigest, parse_module,
        parse_source,
    };

    #[test]
    fn continuation_rejects_revision_query_and_byte_tampering() {
        let revision = RevisionId::from_digest([7; 32]);
        let digest = query_digest(b"owners");
        let handle = encode_continuation(revision, digest, 19);
        assert_eq!(
            decode_continuation(&handle, revision, digest).expect("continuation"),
            19
        );
        assert!(decode_continuation(&handle, RevisionId::from_digest([8; 32]), digest).is_err());
        assert!(decode_continuation(&handle, revision, query_digest(b"other")).is_err());
        let mut tampered = handle.into_bytes();
        let last = tampered.len() - 1;
        tampered[last] = if tampered[last] == b'A' { b'B' } else { b'A' };
        assert!(
            decode_continuation(
                std::str::from_utf8(&tampered).expect("utf8"),
                revision,
                digest
            )
            .is_err()
        );
    }

    #[test]
    fn disposable_index_rebuilds_and_cached_show_loads_one_module() {
        let temporary = tempfile::TempDir::new().expect("temporary project");
        let document = parse_source(
            "fixture.lkj",
            b"(module sample (record Item (name Text)))\n",
            SourceLimits::default(),
        )
        .expect("source oracle");
        let module = parse_module(&document).expect("module oracle");
        let mut allocator = MigrationIdentityAllocator::new(b"query-index".to_vec());
        let meaning = MeaningModule::import(module, &mut allocator).expect("meaning");
        let declaration = meaning.declarations[0].id;
        let root = GraphRoot {
            graph_contract_version: GRAPH_CONTRACT_VERSION,
            repository_id: RepositoryId::migrate(b"query-index", 1),
            package_id: PackageId::parse("10000000000000000000000000000001").expect("package"),
            package_name: "fixture".to_owned(),
            modules: vec![ModuleObjectRef {
                id: meaning.module_id,
                name: meaning.module.name.clone(),
                object: meaning.digest().expect("module digest"),
            }],
            dependencies: Vec::new(),
            targets: Vec::new(),
            tombstones: Vec::new(),
        };
        let (repository, _) = SemanticRepository::initialize(
            temporary.path(),
            InitialPublication {
                root,
                modules: vec![meaning],
                transaction: TransactionDigest::of(b"query-index-import"),
                semantic_diff: SemanticDiffDigest::of(b"query-index-initial"),
                intent: None,
                validation_profile: None,
                dependency_artifacts: Vec::new(),
            },
        )
        .expect("initialize");

        let first = SemanticQueryIndex::current(&repository).expect("build index");
        assert!(first.rebuilt_index());
        let second = SemanticQueryIndex::current(&repository).expect("load index");
        assert!(!second.rebuilt_index());
        assert_eq!(second.module_facts().expect("module facts").len(), 1);
        assert_eq!(
            second
                .show(&declaration.to_string(), true)
                .expect("lazy show")
                .owner
                .id,
            declaration.to_string()
        );

        let exact = SemanticQueryIndex::exact_find_revision(
            &repository,
            repository.current().expect("current").head.revision,
            "Item",
            None,
            QueryBudget::default(),
        )
        .expect("sharded exact find");
        assert_eq!(exact.work, 1);
        assert_eq!(exact.total_items, 1);
        assert_eq!(exact.items[0].id, declaration.to_string());
        assert_eq!(
            SemanticQueryIndex::show_revision(
                &repository,
                repository.current().expect("current").head.revision,
                &declaration.to_string(),
                true,
            )
            .expect("sharded show")
            .owner
            .id,
            declaration.to_string()
        );

        let revision = repository.current().expect("current").head.revision;
        let encoded = super::super::semantic_id::encode_hex(&revision.bytes());
        let path = repository
            .store_path()
            .join("indexes")
            .join(&encoded[..2])
            .join(format!("{encoded}.lkji"));
        std::fs::write(&path, b"corrupt derived bytes").expect("corrupt index");
        let rebuilt = SemanticQueryIndex::current(&repository).expect("rebuild corrupt index");
        assert!(rebuilt.rebuilt_index());
        assert!(
            !SemanticQueryIndex::current(&repository)
                .expect("load repaired index")
                .rebuilt_index()
        );

        let local_owner = repository
            .store_path()
            .join("indexes")
            .join(&encoded[..2])
            .join(&encoded)
            .join("owners")
            .join(format!(
                "{:02x}.lkix",
                local_bucket("owner", &declaration.to_string())
            ));
        std::fs::write(&local_owner, b"corrupt derived owner shard").expect("corrupt owner shard");
        assert_eq!(
            SemanticQueryIndex::show_revision(
                &repository,
                revision,
                &declaration.to_string(),
                true,
            )
            .expect("rebuild corrupt owner shard")
            .owner
            .id,
            declaration.to_string()
        );
    }
}
