//! Persistent, revision-bound semantic facts for incremental validation.
//!
//! Module summaries are content-addressed facts. This module owns the derived physical index that
//! binds those summaries, graph-owned tests, and flat reverse-dependency edges in three persistent
//! Merkle maps. Accepted revisions authenticate the map roots through a constant-size semantic
//! certificate; the pages and manifest remain disposable and rebuildable from canonical meaning.

use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::meaning::{DeclarationKind, GRAPH_CONTRACT_VERSION};
use super::package::PackageId;
use super::packed;
use super::persistent_map::{
    MapEdit, MapError, MapErrorClass, MapRoot, MapWork, MemoryPageStore, OverlayPageStore,
    PageStore, PersistentMap,
};
use super::semantic_digest::{RootObjectDigest, SemanticCertificateDigest};
use super::semantic_id::{DeclarationId, ModuleId, RepositoryId, RevisionId};
use super::semantic_summary::{
    DependencyTarget, ModuleSemanticSummary, SemanticSummaryDigest, SummaryDependencyKind,
    SummaryOwner, validator_contract_digest,
};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const SEMANTIC_FACT_CONTRACT_VERSION: u16 = 3;
pub const SEMANTIC_FACT_CONTRACT_IDENTITY: &str = "lkjscript-semantic-facts-3";
pub const MAXIMUM_SEMANTIC_FACT_MANIFEST_BYTES: usize = 64 * 1024;

const MANIFEST_MAGIC: [u8; 8] = *b"LKJSFI03";
const MANIFEST_DOMAIN: &str = "lkjscript.semantic-fact-manifest.v3";
const CERTIFICATE_DOMAIN: &str = "lkjscript.semantic-certificate.v3";
const SUMMARY_KEY_TAG: u8 = 1;
const TEST_KEY_TAG: u8 = 2;
const REVERSE_KEY_TAG: u8 = 3;
const SUMMARY_BINDING_BYTES: usize = 64;

#[derive(Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticFactRoots {
    pub summaries: MapRoot,
    pub tests: MapRoot,
    pub reverse: MapRoot,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticFactManifest {
    pub contract_version: u16,
    pub graph_contract_version: u16,
    pub validator_contract: SemanticSummaryDigest,
    pub repository_id: RepositoryId,
    pub package_id: PackageId,
    pub revision: RevisionId,
    pub canonical_root: RootObjectDigest,
    pub roots: SemanticFactRoots,
    pub certificate: SemanticCertificateDigest,
}

#[derive(Clone, Debug)]
pub struct SemanticFactUpdate {
    pub manifest: SemanticFactManifest,
    pub pages: MemoryPageStore,
    pub work: MapWork,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModuleSummaryBinding {
    pub input: SemanticSummaryDigest,
    pub summary: SemanticSummaryDigest,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleInvalidationClass {
    Unchanged,
    PrivateImplementation,
    PublicSignature,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InvalidationFrontier {
    pub class: ModuleInvalidationClass,
    pub changed_declarations: Vec<DeclarationId>,
    pub validate_modules: Vec<ModuleId>,
    pub retest_modules: Vec<ModuleId>,
    pub traversed_edges: u64,
    pub pages_read: u64,
    pub bytes_read: u64,
}

const PUBLIC_PROPAGATION_KINDS: [SummaryDependencyKind; 7] = [
    SummaryDependencyKind::Namespace,
    SummaryDependencyKind::Type,
    SummaryDependencyKind::Value,
    SummaryDependencyKind::Call,
    SummaryDependencyKind::Effect,
    SummaryDependencyKind::Capability,
    SummaryDependencyKind::Deployment,
];

const EXECUTION_PROPAGATION_KINDS: [SummaryDependencyKind; 3] = [
    SummaryDependencyKind::Value,
    SummaryDependencyKind::Call,
    SummaryDependencyKind::Effect,
];

#[derive(Encode)]
struct CertificateCore<'a> {
    fact_contract: u16,
    summary_contract: u16,
    validator_contract: SemanticSummaryDigest,
    package: &'a PackageId,
    roots: SemanticFactRoots,
}

impl SemanticFactManifest {
    pub fn new(
        repository_id: RepositoryId,
        package_id: PackageId,
        revision: RevisionId,
        canonical_root: RootObjectDigest,
        roots: SemanticFactRoots,
    ) -> Result<Self, Diagnostic> {
        let validator_contract = validator_contract_digest();
        let certificate = semantic_certificate(&package_id, validator_contract, roots)?;
        let manifest = Self {
            contract_version: SEMANTIC_FACT_CONTRACT_VERSION,
            graph_contract_version: GRAPH_CONTRACT_VERSION,
            validator_contract,
            repository_id,
            package_id,
            revision,
            canonical_root,
            roots,
            certificate,
        };
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn encode(&self) -> Result<Vec<u8>, Diagnostic> {
        self.validate()?;
        packed::encode(
            MANIFEST_MAGIC,
            MANIFEST_DOMAIN,
            self,
            MAXIMUM_SEMANTIC_FACT_MANIFEST_BYTES,
        )
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, Diagnostic> {
        let manifest: Self = packed::decode(
            bytes,
            MANIFEST_MAGIC,
            MANIFEST_DOMAIN,
            MAXIMUM_SEMANTIC_FACT_MANIFEST_BYTES,
        )?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<(), Diagnostic> {
        if self.contract_version != SEMANTIC_FACT_CONTRACT_VERSION
            || self.graph_contract_version != GRAPH_CONTRACT_VERSION
        {
            return Err(fact_error(
                DiagnosticClass::Source,
                "semantic_fact_contract",
                "semantic fact manifest belongs to a predecessor fact or graph contract",
            ));
        }
        if self.validator_contract != validator_contract_digest() {
            return Err(fact_error(
                DiagnosticClass::Source,
                "semantic_fact_validator_contract",
                "semantic fact manifest belongs to a foreign validator contract",
            ));
        }
        if self.roots.summaries.entries() == 0 {
            return Err(fact_error(
                DiagnosticClass::Corrupt,
                "semantic_fact_empty_summaries",
                "semantic fact manifest must bind at least one module summary",
            ));
        }
        let expected = semantic_certificate(&self.package_id, self.validator_contract, self.roots)?;
        if self.certificate != expected {
            return Err(fact_error(
                DiagnosticClass::Corrupt,
                "semantic_fact_certificate",
                "semantic fact manifest certificate does not bind its exact Merkle roots",
            ));
        }
        Ok(())
    }

    pub fn rebind_revision(&self, revision: RevisionId) -> Result<Self, Diagnostic> {
        let mut rebound = self.clone();
        rebound.revision = revision;
        rebound.validate()?;
        Ok(rebound)
    }

    pub fn summary_binding<S: PageStore + ?Sized>(
        &self,
        store: &S,
        module: ModuleId,
        work: &mut MapWork,
    ) -> Result<Option<ModuleSummaryBinding>, Diagnostic> {
        PersistentMap::from_root(self.roots.summaries)
            .lookup(store, &summary_key(module), work)
            .map_err(map_diagnostic)?
            .map(|bytes| decode_summary_binding(&bytes))
            .transpose()
    }

    pub fn contains_test<S: PageStore + ?Sized>(
        &self,
        store: &S,
        owner: &SummaryOwner,
        work: &mut MapWork,
    ) -> Result<bool, Diagnostic> {
        Ok(PersistentMap::from_root(self.roots.tests)
            .lookup(store, &test_key(owner), work)
            .map_err(map_diagnostic)?
            .is_some())
    }

    pub fn for_each_dependent<S, F>(
        &self,
        store: &S,
        target: &DependencyTarget,
        kind: SummaryDependencyKind,
        work: &mut MapWork,
        mut visitor: F,
    ) -> Result<u64, Diagnostic>
    where
        S: PageStore + ?Sized,
        F: FnMut(SummaryOwner) -> Result<(), MapError>,
    {
        let prefix = reverse_prefix(target, kind);
        PersistentMap::from_root(self.roots.reverse)
            .for_each_prefix(store, &prefix, work, |key, value| {
                if !value.is_empty() || !key.starts_with(&prefix) {
                    return Err(map_fact_error(
                        MapErrorClass::Corrupt,
                        "semantic_fact_reverse_value",
                        "reverse semantic fact has a foreign key or nonempty set value",
                    ));
                }
                let dependent = decode_owner(&key[prefix.len()..])?;
                visitor(dependent)
            })
            .map_err(map_diagnostic)
    }

    /// Exhaustively verifies all derived pages and typed fact bindings. This is the derived-cache
    /// oracle; ordinary local publication uses exact path updates instead.
    pub fn verify<S: PageStore + ?Sized>(&self, store: &S) -> Result<MapWork, Diagnostic> {
        self.validate()?;
        let mut work = MapWork::default();
        let mut modules = BTreeSet::new();
        PersistentMap::from_root(self.roots.summaries)
            .for_each(store, &mut work, |key, value| {
                let module = decode_summary_key(key)?;
                decode_summary_binding(value).map_err(diagnostic_map)?;
                if !modules.insert(module) {
                    return Err(map_fact_error(
                        MapErrorClass::Corrupt,
                        "semantic_fact_summary_duplicate",
                        "semantic fact summary map contains a duplicate module",
                    ));
                }
                Ok(())
            })
            .map_err(map_diagnostic)?;
        PersistentMap::from_root(self.roots.tests)
            .for_each(store, &mut work, |key, value| {
                if !value.is_empty() {
                    return Err(map_fact_error(
                        MapErrorClass::Corrupt,
                        "semantic_fact_test_value",
                        "test semantic fact set has a nonempty value",
                    ));
                }
                let owner = decode_test_key(key)?;
                if owner.declaration.is_none() || !modules.contains(&owner.module) {
                    return Err(map_fact_error(
                        MapErrorClass::Corrupt,
                        "semantic_fact_test_owner",
                        "test semantic fact names a foreign or module-level owner",
                    ));
                }
                Ok(())
            })
            .map_err(map_diagnostic)?;
        PersistentMap::from_root(self.roots.reverse)
            .for_each(store, &mut work, |key, value| {
                if !value.is_empty() {
                    return Err(map_fact_error(
                        MapErrorClass::Corrupt,
                        "semantic_fact_reverse_value",
                        "reverse semantic fact set has a nonempty value",
                    ));
                }
                let (_target, _kind, dependent) = decode_reverse_key(key)?;
                if !modules.contains(&dependent.module) {
                    return Err(map_fact_error(
                        MapErrorClass::Corrupt,
                        "semantic_fact_reverse_owner",
                        "reverse semantic fact names a dependent outside the package",
                    ));
                }
                Ok(())
            })
            .map_err(map_diagnostic)?;
        Ok(work)
    }
}

pub fn build_semantic_facts(
    repository_id: RepositoryId,
    package_id: &PackageId,
    revision: RevisionId,
    canonical_root: RootObjectDigest,
    summaries: &[ModuleSemanticSummary],
) -> Result<SemanticFactUpdate, Diagnostic> {
    if summaries.is_empty() {
        return Err(fact_error(
            DiagnosticClass::Source,
            "semantic_fact_empty_summaries",
            "semantic fact generation requires at least one module summary",
        ));
    }
    validate_summary_generation(package_id, summaries)?;
    let mut ordered = summaries.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|summary| summary.module);
    if ordered
        .windows(2)
        .any(|pair| pair[0].module == pair[1].module)
    {
        return Err(fact_error(
            DiagnosticClass::Source,
            "semantic_fact_duplicate_module",
            "one semantic fact generation contains duplicate module summaries",
        ));
    }
    let summary_entries = ordered
        .iter()
        .map(|summary| {
            (
                summary_key(summary.module),
                encode_summary_binding(ModuleSummaryBinding {
                    input: summary.input,
                    summary: summary.digest,
                }),
            )
        })
        .collect::<Vec<_>>();
    let test_entries = ordered
        .iter()
        .flat_map(|summary| {
            summary
                .declarations
                .iter()
                .filter(|declaration| declaration.kind == DeclarationKind::Test)
                .map(|declaration| {
                    (
                        test_key(&SummaryOwner::declaration(
                            summary.module,
                            declaration.declaration,
                        )),
                        Vec::new(),
                    )
                })
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let reverse_entries = ordered
        .iter()
        .flat_map(|summary| {
            summary.dependencies.iter().map(|dependency| {
                (
                    reverse_key(&dependency.target, dependency.kind, &dependency.source),
                    Vec::new(),
                )
            })
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut pages = MemoryPageStore::default();
    let mut work = MapWork::default();
    let summaries = PersistentMap::from_sorted(&mut pages, summary_entries, &mut work)
        .map_err(map_diagnostic)?;
    let tests =
        PersistentMap::from_sorted(&mut pages, test_entries, &mut work).map_err(map_diagnostic)?;
    let reverse = PersistentMap::from_sorted(&mut pages, reverse_entries, &mut work)
        .map_err(map_diagnostic)?;
    let roots = SemanticFactRoots {
        summaries: summaries.root(),
        tests: tests.root(),
        reverse: reverse.root(),
    };
    let manifest = SemanticFactManifest::new(
        repository_id,
        package_id.clone(),
        revision,
        canonical_root,
        roots,
    )?;
    manifest.verify(&pages)?;
    Ok(SemanticFactUpdate {
        manifest,
        pages,
        work,
    })
}

pub fn update_semantic_facts<S: PageStore + ?Sized>(
    base: &SemanticFactManifest,
    store: &S,
    revision: RevisionId,
    canonical_root: RootObjectDigest,
    before: &[ModuleSemanticSummary],
    after: &[ModuleSemanticSummary],
) -> Result<SemanticFactUpdate, Diagnostic> {
    base.validate()?;
    validate_summary_generation(&base.package_id, before)?;
    validate_summary_generation(&base.package_id, after)?;
    let before_modules = before
        .iter()
        .map(|summary| summary.module)
        .collect::<BTreeSet<_>>();
    let after_modules = after
        .iter()
        .map(|summary| summary.module)
        .collect::<BTreeSet<_>>();
    if before_modules.len() != before.len() || after_modules.len() != after.len() {
        return Err(fact_error(
            DiagnosticClass::Source,
            "semantic_fact_delta_modules",
            "semantic fact delta contains duplicate or ambiguous module summaries",
        ));
    }

    let summary_edits = map_fact_edits(
        before.iter().map(|summary| {
            (
                summary_key(summary.module),
                encode_summary_binding(ModuleSummaryBinding {
                    input: summary.input,
                    summary: summary.digest,
                }),
            )
        }),
        after.iter().map(|summary| {
            (
                summary_key(summary.module),
                encode_summary_binding(ModuleSummaryBinding {
                    input: summary.input,
                    summary: summary.digest,
                }),
            )
        }),
    )?;
    let test_edits = map_fact_edits(test_facts(before), test_facts(after))?;
    let reverse_edits = map_fact_edits(reverse_facts(before), reverse_facts(after))?;

    let mut overlay = OverlayPageStore::new(store);
    let mut work = MapWork::default();
    let summaries = PersistentMap::from_root(base.roots.summaries)
        .apply_sorted_edits(&mut overlay, &summary_edits, &mut work)
        .map_err(map_diagnostic)?
        .0;
    let tests = PersistentMap::from_root(base.roots.tests)
        .apply_sorted_edits(&mut overlay, &test_edits, &mut work)
        .map_err(map_diagnostic)?
        .0;
    let reverse = PersistentMap::from_root(base.roots.reverse)
        .apply_sorted_edits(&mut overlay, &reverse_edits, &mut work)
        .map_err(map_diagnostic)?
        .0;
    let roots = SemanticFactRoots {
        summaries: summaries.root(),
        tests: tests.root(),
        reverse: reverse.root(),
    };
    let manifest = SemanticFactManifest::new(
        base.repository_id,
        base.package_id.clone(),
        revision,
        canonical_root,
        roots,
    )?;
    let staged = overlay.into_pages();
    let mut pages = MemoryPageStore::default();
    for (map, previous) in [
        (summaries, base.roots.summaries),
        (tests, base.roots.tests),
        (reverse, base.roots.reverse),
    ] {
        if map.root() != previous {
            map.copy_staged_reachable(&staged, &mut pages, &mut work)
                .map_err(map_diagnostic)?;
        }
    }
    Ok(SemanticFactUpdate {
        manifest,
        pages,
        work,
    })
}

pub fn build_semantic_certificate(
    summaries: &[ModuleSemanticSummary],
) -> Result<SemanticCertificateDigest, Diagnostic> {
    let package = summaries.first().ok_or_else(|| {
        fact_error(
            DiagnosticClass::Source,
            "semantic_fact_empty_summaries",
            "semantic certificate requires at least one module summary",
        )
    })?;
    let update = build_semantic_facts(
        RepositoryId::migrate(b"semantic-certificate-oracle", 1),
        &package.package,
        RevisionId::from_digest([0; 32]),
        RootObjectDigest::from_bytes([0; 32]),
        summaries,
    )?;
    Ok(update.manifest.certificate)
}

pub fn classify_module_change(
    before: &ModuleSemanticSummary,
    after: &ModuleSemanticSummary,
) -> Result<ModuleInvalidationClass, Diagnostic> {
    before.validate()?;
    after.validate()?;
    require_same_summary_owner(before, after)?;
    if before.public_signature != after.public_signature {
        Ok(ModuleInvalidationClass::PublicSignature)
    } else if before.input != after.input
        || before.implementation != after.implementation
        || before.dependency_digest != after.dependency_digest
        || before.declarations != after.declarations
    {
        Ok(ModuleInvalidationClass::PrivateImplementation)
    } else {
        Ok(ModuleInvalidationClass::Unchanged)
    }
}

/// Computes a bounded conservative invalidation closure from persistent reverse facts.
///
/// The manifest must bind the exact accepted base and the supplied `before` summary. Callers may
/// then validate the returned modules and run the selected tests, while the exhaustive validator
/// remains the independent publication oracle for broad or unsupported change classes.
pub fn invalidation_frontier<S: PageStore + ?Sized>(
    expected_base: RevisionId,
    before: &ModuleSemanticSummary,
    after: &ModuleSemanticSummary,
    facts: &SemanticFactManifest,
    store: &S,
    maximum_edges: u64,
) -> Result<InvalidationFrontier, Diagnostic> {
    let class = classify_module_change(before, after)?;
    facts.validate()?;
    if maximum_edges == 0 {
        return Err(fact_error(
            DiagnosticClass::Resource,
            "semantic_fact_invalidation_budget",
            "semantic invalidation requires a positive edge budget",
        ));
    }
    if facts.package_id != before.package
        || facts.revision != expected_base
        || facts.validator_contract != before.validator_contract
    {
        return Err(fact_error(
            DiagnosticClass::Source,
            "semantic_fact_stale",
            "semantic invalidation requires the exact base revision fact generation",
        ));
    }

    let mut work = MapWork::default();
    let binding = facts
        .summary_binding(store, before.module, &mut work)?
        .ok_or_else(|| {
            fact_error(
                DiagnosticClass::Corrupt,
                "semantic_fact_summary_missing",
                "semantic fact generation omits the changed module summary",
            )
        })?;
    if binding.input != before.input || binding.summary != before.digest {
        return Err(fact_error(
            DiagnosticClass::Corrupt,
            "semantic_fact_summary_stale",
            "semantic fact generation does not bind the supplied base module summary",
        ));
    }

    let changed_declarations = changed_declarations(before, after);
    if class == ModuleInvalidationClass::Unchanged {
        return Ok(InvalidationFrontier {
            class,
            changed_declarations,
            validate_modules: Vec::new(),
            retest_modules: Vec::new(),
            traversed_edges: 0,
            pages_read: work.pages_read,
            bytes_read: work.bytes_read,
        });
    }

    let mut validate_modules = BTreeSet::from([before.module]);
    let mut retest_modules = before
        .declarations
        .iter()
        .chain(&after.declarations)
        .filter(|declaration| declaration.kind == DeclarationKind::Test)
        .map(|_| before.module)
        .collect::<BTreeSet<_>>();
    let initial_owners = if changed_declarations.is_empty() {
        vec![SummaryOwner::module(before.module)]
    } else {
        changed_declarations
            .iter()
            .copied()
            .map(|declaration| SummaryOwner::declaration(before.module, declaration))
            .collect()
    };
    let mut queue = initial_owners
        .iter()
        .map(|owner| owner.target(&before.package))
        .collect::<VecDeque<_>>();
    let mut seen = queue.iter().cloned().collect::<BTreeSet<_>>();
    let mut traversed_edges = 0_u64;

    while let Some(target) = queue.pop_front() {
        let propagation: &[SummaryDependencyKind] = match class {
            ModuleInvalidationClass::PublicSignature => &PUBLIC_PROPAGATION_KINDS,
            ModuleInvalidationClass::PrivateImplementation => &EXECUTION_PROPAGATION_KINDS,
            ModuleInvalidationClass::Unchanged => &[],
        };
        for &kind in propagation {
            let mut dependents = Vec::new();
            facts.for_each_dependent(store, &target, kind, &mut work, |dependent| {
                traversed_edges = traversed_edges.checked_add(1).ok_or_else(|| {
                    map_fact_error(
                        MapErrorClass::Resource,
                        "semantic_fact_invalidation_budget",
                        "semantic invalidation edge count overflowed",
                    )
                })?;
                if traversed_edges > maximum_edges {
                    return Err(map_fact_error(
                        MapErrorClass::Resource,
                        "semantic_fact_invalidation_budget",
                        "semantic invalidation exhausted its declared edge budget",
                    ));
                }
                dependents.push(dependent);
                Ok(())
            })?;
            for dependent in dependents {
                if class == ModuleInvalidationClass::PublicSignature {
                    validate_modules.insert(dependent.module);
                }
                if facts.contains_test(store, &dependent, &mut work)? {
                    retest_modules.insert(dependent.module);
                }
                let dependent_target = dependent.target(&before.package);
                if seen.insert(dependent_target.clone()) {
                    queue.push_back(dependent_target);
                }
            }
        }

        let mut test_dependents = Vec::new();
        facts.for_each_dependent(
            store,
            &target,
            SummaryDependencyKind::Test,
            &mut work,
            |dependent| {
                traversed_edges = traversed_edges.checked_add(1).ok_or_else(|| {
                    map_fact_error(
                        MapErrorClass::Resource,
                        "semantic_fact_invalidation_budget",
                        "semantic invalidation edge count overflowed",
                    )
                })?;
                if traversed_edges > maximum_edges {
                    return Err(map_fact_error(
                        MapErrorClass::Resource,
                        "semantic_fact_invalidation_budget",
                        "semantic invalidation exhausted its declared edge budget",
                    ));
                }
                test_dependents.push(dependent);
                Ok(())
            },
        )?;
        for dependent in test_dependents {
            retest_modules.insert(dependent.module);
            if class == ModuleInvalidationClass::PublicSignature {
                validate_modules.insert(dependent.module);
            }
        }
    }

    Ok(InvalidationFrontier {
        class,
        changed_declarations,
        validate_modules: validate_modules.into_iter().collect(),
        retest_modules: retest_modules.into_iter().collect(),
        traversed_edges,
        pages_read: work.pages_read,
        bytes_read: work.bytes_read,
    })
}

fn require_same_summary_owner(
    before: &ModuleSemanticSummary,
    after: &ModuleSemanticSummary,
) -> Result<(), Diagnostic> {
    if before.package != after.package || before.module != after.module {
        return Err(fact_error(
            DiagnosticClass::Source,
            "semantic_fact_foreign_owner",
            "module summaries belong to different stable package or module identities",
        ));
    }
    if before.validator_contract != after.validator_contract {
        return Err(fact_error(
            DiagnosticClass::Source,
            "semantic_fact_validator_change",
            "validator contract changes require complete summary and fact invalidation",
        ));
    }
    Ok(())
}

fn changed_declarations(
    before: &ModuleSemanticSummary,
    after: &ModuleSemanticSummary,
) -> Vec<DeclarationId> {
    let before = before
        .declarations
        .iter()
        .map(|summary| (summary.declaration, summary))
        .collect::<BTreeMap<_, _>>();
    let after = after
        .declarations
        .iter()
        .map(|summary| (summary.declaration, summary))
        .collect::<BTreeMap<_, _>>();
    before
        .keys()
        .chain(after.keys())
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|declaration| before.get(declaration) != after.get(declaration))
        .collect()
}

fn validate_summary_generation(
    package: &PackageId,
    summaries: &[ModuleSemanticSummary],
) -> Result<(), Diagnostic> {
    for summary in summaries {
        summary.validate()?;
        if &summary.package != package || summary.validator_contract != validator_contract_digest()
        {
            return Err(fact_error(
                DiagnosticClass::Source,
                "semantic_fact_summary_generation",
                "semantic fact summaries must bind one exact package and validator contract",
            ));
        }
    }
    Ok(())
}

fn semantic_certificate(
    package: &PackageId,
    validator_contract: SemanticSummaryDigest,
    roots: SemanticFactRoots,
) -> Result<SemanticCertificateDigest, Diagnostic> {
    let core = CertificateCore {
        fact_contract: SEMANTIC_FACT_CONTRACT_VERSION,
        summary_contract: super::semantic_summary::SEMANTIC_SUMMARY_CONTRACT_VERSION,
        validator_contract,
        package,
        roots,
    };
    let bytes = bincode::encode_to_vec(core, packed_configuration()).map_err(|error| {
        fact_error(
            DiagnosticClass::Infrastructure,
            "semantic_fact_certificate_encode",
            format!("semantic certificate inputs could not be encoded: {error}"),
        )
    })?;
    let mut hasher = blake3::Hasher::new_derive_key(CERTIFICATE_DOMAIN);
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(&bytes);
    Ok(SemanticCertificateDigest::from_bytes(
        *hasher.finalize().as_bytes(),
    ))
}

fn summary_key(module: ModuleId) -> Vec<u8> {
    [SUMMARY_KEY_TAG]
        .into_iter()
        .chain(module.bytes())
        .collect()
}

fn decode_summary_key(key: &[u8]) -> Result<ModuleId, MapError> {
    if key.len() != 17 || key[0] != SUMMARY_KEY_TAG {
        return Err(map_fact_error(
            MapErrorClass::Corrupt,
            "semantic_fact_summary_key",
            "semantic summary fact has a foreign key encoding",
        ));
    }
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&key[1..]);
    module_id_from_bytes(bytes)
}

fn encode_summary_binding(binding: ModuleSummaryBinding) -> Vec<u8> {
    [binding.input.bytes(), binding.summary.bytes()].concat()
}

fn decode_summary_binding(bytes: &[u8]) -> Result<ModuleSummaryBinding, Diagnostic> {
    if bytes.len() != SUMMARY_BINDING_BYTES {
        return Err(fact_error(
            DiagnosticClass::Corrupt,
            "semantic_fact_summary_binding",
            "semantic summary binding has a foreign fixed-width encoding",
        ));
    }
    let mut input = [0_u8; 32];
    input.copy_from_slice(&bytes[..32]);
    let mut summary = [0_u8; 32];
    summary.copy_from_slice(&bytes[32..]);
    Ok(ModuleSummaryBinding {
        input: SemanticSummaryDigest::from_bytes(input),
        summary: SemanticSummaryDigest::from_bytes(summary),
    })
}

fn encode_owner(owner: &SummaryOwner) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(33);
    bytes.extend_from_slice(&owner.module.bytes());
    match owner.declaration {
        Some(declaration) => {
            bytes.push(1);
            bytes.extend_from_slice(&declaration.bytes());
        }
        None => bytes.push(0),
    }
    bytes
}

fn module_id_from_bytes(bytes: [u8; 16]) -> Result<ModuleId, MapError> {
    ModuleId::parse(&format!(
        "{}{}",
        ModuleId::PREFIX,
        super::semantic_id::encode_hex(&bytes)
    ))
    .map_err(repository_map_error)
}

fn declaration_id_from_bytes(bytes: [u8; 16]) -> Result<DeclarationId, MapError> {
    DeclarationId::parse(&format!(
        "{}{}",
        DeclarationId::PREFIX,
        super::semantic_id::encode_hex(&bytes)
    ))
    .map_err(repository_map_error)
}

fn decode_owner(bytes: &[u8]) -> Result<SummaryOwner, MapError> {
    if bytes.len() != 17 && bytes.len() != 33 {
        return Err(map_fact_error(
            MapErrorClass::Corrupt,
            "semantic_fact_owner_key",
            "semantic fact owner has a foreign fixed-width encoding",
        ));
    }
    let mut module = [0_u8; 16];
    module.copy_from_slice(&bytes[..16]);
    match (bytes[16], bytes.len()) {
        (0, 17) => Ok(SummaryOwner::module(module_id_from_bytes(module)?)),
        (1, 33) => {
            let mut declaration = [0_u8; 16];
            declaration.copy_from_slice(&bytes[17..]);
            Ok(SummaryOwner::declaration(
                module_id_from_bytes(module)?,
                declaration_id_from_bytes(declaration)?,
            ))
        }
        _ => Err(map_fact_error(
            MapErrorClass::Corrupt,
            "semantic_fact_owner_tag",
            "semantic fact owner has a foreign declaration-presence tag",
        )),
    }
}

fn test_key(owner: &SummaryOwner) -> Vec<u8> {
    [vec![TEST_KEY_TAG], encode_owner(owner)].concat()
}

fn decode_test_key(key: &[u8]) -> Result<SummaryOwner, MapError> {
    if key.first() != Some(&TEST_KEY_TAG) {
        return Err(map_fact_error(
            MapErrorClass::Corrupt,
            "semantic_fact_test_key",
            "test semantic fact has a foreign key domain",
        ));
    }
    decode_owner(&key[1..])
}

fn dependency_kind_tag(kind: SummaryDependencyKind) -> u8 {
    match kind {
        SummaryDependencyKind::Namespace => 1,
        SummaryDependencyKind::Type => 2,
        SummaryDependencyKind::Value => 3,
        SummaryDependencyKind::Call => 4,
        SummaryDependencyKind::Effect => 5,
        SummaryDependencyKind::Capability => 6,
        SummaryDependencyKind::Deployment => 7,
        SummaryDependencyKind::Test => 8,
    }
}

fn decode_dependency_kind(tag: u8) -> Result<SummaryDependencyKind, MapError> {
    match tag {
        1 => Ok(SummaryDependencyKind::Namespace),
        2 => Ok(SummaryDependencyKind::Type),
        3 => Ok(SummaryDependencyKind::Value),
        4 => Ok(SummaryDependencyKind::Call),
        5 => Ok(SummaryDependencyKind::Effect),
        6 => Ok(SummaryDependencyKind::Capability),
        7 => Ok(SummaryDependencyKind::Deployment),
        8 => Ok(SummaryDependencyKind::Test),
        _ => Err(map_fact_error(
            MapErrorClass::Corrupt,
            "semantic_fact_dependency_kind",
            "reverse semantic fact has a foreign dependency-kind tag",
        )),
    }
}

fn encode_target(target: &DependencyTarget) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(49);
    bytes.extend_from_slice(&target.package.bytes());
    bytes.extend_from_slice(&target.module.bytes());
    match target.declaration {
        Some(declaration) => {
            bytes.push(1);
            bytes.extend_from_slice(&declaration.bytes());
        }
        None => bytes.push(0),
    }
    bytes
}

fn decode_target(bytes: &[u8]) -> Result<(DependencyTarget, usize), MapError> {
    if bytes.len() < 33 {
        return Err(map_fact_error(
            MapErrorClass::Corrupt,
            "semantic_fact_target_key",
            "reverse semantic target key is truncated",
        ));
    }
    let mut package = [0_u8; 16];
    package.copy_from_slice(&bytes[..16]);
    let package = PackageId::parse(&super::semantic_id::encode_hex(&package))
        .map_err(repository_map_error)?;
    let mut module = [0_u8; 16];
    module.copy_from_slice(&bytes[16..32]);
    match bytes[32] {
        0 => Ok((
            DependencyTarget {
                package,
                module: module_id_from_bytes(module)?,
                declaration: None,
            },
            33,
        )),
        1 if bytes.len() >= 49 => {
            let mut declaration = [0_u8; 16];
            declaration.copy_from_slice(&bytes[33..49]);
            Ok((
                DependencyTarget {
                    package,
                    module: module_id_from_bytes(module)?,
                    declaration: Some(declaration_id_from_bytes(declaration)?),
                },
                49,
            ))
        }
        _ => Err(map_fact_error(
            MapErrorClass::Corrupt,
            "semantic_fact_target_tag",
            "reverse semantic target has a foreign declaration-presence tag",
        )),
    }
}

fn reverse_prefix(target: &DependencyTarget, kind: SummaryDependencyKind) -> Vec<u8> {
    let mut key = Vec::with_capacity(51);
    key.push(REVERSE_KEY_TAG);
    key.extend_from_slice(&encode_target(target));
    key.push(dependency_kind_tag(kind));
    key
}

fn reverse_key(
    target: &DependencyTarget,
    kind: SummaryDependencyKind,
    dependent: &SummaryOwner,
) -> Vec<u8> {
    [reverse_prefix(target, kind), encode_owner(dependent)].concat()
}

fn decode_reverse_key(
    key: &[u8],
) -> Result<(DependencyTarget, SummaryDependencyKind, SummaryOwner), MapError> {
    if key.first() != Some(&REVERSE_KEY_TAG) {
        return Err(map_fact_error(
            MapErrorClass::Corrupt,
            "semantic_fact_reverse_key",
            "reverse semantic fact has a foreign key domain",
        ));
    }
    let (target, target_length) = decode_target(&key[1..])?;
    let kind_position = 1 + target_length;
    let kind = decode_dependency_kind(*key.get(kind_position).ok_or_else(|| {
        map_fact_error(
            MapErrorClass::Corrupt,
            "semantic_fact_reverse_key",
            "reverse semantic fact omits its dependency kind",
        )
    })?)?;
    let dependent = decode_owner(&key[kind_position + 1..])?;
    Ok((target, kind, dependent))
}

fn test_facts(
    summaries: &[ModuleSemanticSummary],
) -> impl Iterator<Item = (Vec<u8>, Vec<u8>)> + '_ {
    summaries.iter().flat_map(|summary| {
        summary
            .declarations
            .iter()
            .filter(|declaration| declaration.kind == DeclarationKind::Test)
            .map(|declaration| {
                (
                    test_key(&SummaryOwner::declaration(
                        summary.module,
                        declaration.declaration,
                    )),
                    Vec::new(),
                )
            })
    })
}

fn reverse_facts(
    summaries: &[ModuleSemanticSummary],
) -> impl Iterator<Item = (Vec<u8>, Vec<u8>)> + '_ {
    summaries.iter().flat_map(|summary| {
        summary.dependencies.iter().map(|dependency| {
            (
                reverse_key(&dependency.target, dependency.kind, &dependency.source),
                Vec::new(),
            )
        })
    })
}

#[derive(Default)]
struct PendingFactEdit {
    before: Option<Vec<u8>>,
    before_set: bool,
    after: Option<Vec<u8>>,
    after_set: bool,
}

fn map_fact_edits(
    before: impl IntoIterator<Item = (Vec<u8>, Vec<u8>)>,
    after: impl IntoIterator<Item = (Vec<u8>, Vec<u8>)>,
) -> Result<Vec<MapEdit>, Diagnostic> {
    let mut pending = BTreeMap::<Vec<u8>, PendingFactEdit>::new();
    for (key, value) in before {
        let edit = pending.entry(key).or_default();
        if edit.before_set {
            return Err(fact_error(
                DiagnosticClass::Source,
                "semantic_fact_delta_duplicate_before",
                "semantic fact delta contains one before fact more than once",
            ));
        }
        edit.before_set = true;
        edit.before = Some(value);
    }
    for (key, value) in after {
        let edit = pending.entry(key).or_default();
        if edit.after_set {
            return Err(fact_error(
                DiagnosticClass::Source,
                "semantic_fact_delta_duplicate_after",
                "semantic fact delta contains one after fact more than once",
            ));
        }
        edit.after_set = true;
        edit.after = Some(value);
    }
    Ok(pending
        .into_iter()
        .map(|(key, edit)| MapEdit {
            key,
            before: edit.before,
            after: edit.after,
        })
        .collect())
}

fn packed_configuration() -> impl bincode::config::Config {
    bincode::config::standard()
        .with_little_endian()
        .with_variable_int_encoding()
}

fn map_diagnostic(error: MapError) -> Diagnostic {
    fact_error(
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

fn diagnostic_map(error: Diagnostic) -> MapError {
    map_fact_error(
        match error.class {
            DiagnosticClass::Source | DiagnosticClass::Semantic | DiagnosticClass::Capability => {
                MapErrorClass::Input
            }
            DiagnosticClass::Resource => MapErrorClass::Resource,
            DiagnosticClass::Corrupt => MapErrorClass::Corrupt,
            DiagnosticClass::Cancelled | DiagnosticClass::Infrastructure => MapErrorClass::Store,
        },
        "semantic_fact_typed_decode",
        format!("{}: {}", error.code, error.message),
    )
}

fn repository_map_error(error: Diagnostic) -> MapError {
    diagnostic_map(error)
}

fn map_fact_error(
    class: MapErrorClass,
    code: &'static str,
    message: impl Into<String>,
) -> MapError {
    MapError {
        class,
        code,
        message: message.into(),
    }
}

fn fact_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::language::{
        Declaration, Effect, Expression, Function, Module, TestCase, Type,
    };
    use crate::platform::meaning::{
        DeclarationReference, GRAPH_CONTRACT_VERSION, RelationRole, RelationSource, RelationTarget,
        RequestIdentityAllocator, SemanticRelation,
    };
    use crate::platform::semantic_id::RevisionId;
    use crate::platform::syntax::SourceSpan;

    fn package() -> PackageId {
        PackageId::parse("10000000000000000000000000000001").expect("package")
    }

    fn empty_module(ordinal: u64) -> super::super::meaning::MeaningModule {
        let module_id = ModuleId::migrate(b"semantic-fact-module", ordinal);
        super::super::meaning::MeaningModule {
            graph_contract_version: GRAPH_CONTRACT_VERSION,
            module_id,
            module: Module {
                name: format!("module{ordinal}"),
                imports: Vec::new(),
                exports: Vec::new(),
                declarations: Vec::new(),
            },
            declarations: Vec::new(),
            relations: Vec::new(),
            documentation: Vec::new(),
            annotations: Vec::new(),
        }
    }

    fn span() -> SourceSpan {
        SourceSpan {
            byte_start: 0,
            byte_end: 0,
            line: 0,
            column: 0,
        }
    }

    fn pure_function(name: &str, result: Type, value: i64) -> Declaration {
        Declaration::Function(Function {
            name: name.to_owned(),
            type_parameters: Vec::new(),
            parameters: Vec::new(),
            result,
            effect: Effect::Pure,
            body: Expression::I64(value, span()),
            span: span(),
        })
    }

    fn test_case(name: &str) -> Declaration {
        Declaration::Test(TestCase {
            name: name.to_owned(),
            actual: Expression::I64(1, span()),
            expected: Expression::I64(1, span()),
            span: span(),
        })
    }

    fn meaning_module(
        seed: &[u8],
        name: &str,
        exports: &[&str],
        declarations: Vec<Declaration>,
    ) -> super::super::meaning::MeaningModule {
        let mut module = super::super::meaning::MeaningModule::create(
            Module {
                name: name.to_owned(),
                imports: Vec::new(),
                exports: Vec::new(),
                declarations,
            },
            &mut RequestIdentityAllocator::new(seed),
        )
        .expect("meaning module");
        module.module.exports = exports
            .iter()
            .map(|name| {
                module
                    .declaration_by_name(name)
                    .expect("exported declaration")
                    .0
                    .id
            })
            .collect();
        module.module.exports.sort();
        module
    }

    fn declaration_target(
        package: &PackageId,
        module: &super::super::meaning::MeaningModule,
        declaration: DeclarationId,
    ) -> RelationTarget {
        RelationTarget::Declaration(DeclarationReference {
            package: package.clone(),
            module: module.module_id,
            declaration,
        })
    }

    fn add_relation(
        module: &mut super::super::meaning::MeaningModule,
        source: DeclarationId,
        target: RelationTarget,
        role: RelationRole,
    ) {
        module.relations.push(SemanticRelation {
            source: RelationSource::Declaration(source),
            target,
            role,
        });
        module.relations.sort();
        module.relations.dedup();
    }

    #[test]
    fn full_and_delta_fact_roots_are_identical_and_local() {
        let package = package();
        let modules = (0..10_000).map(empty_module).collect::<Vec<_>>();
        let summaries = modules
            .iter()
            .map(|module| super::super::semantic_summary::build_module_summary(&package, module))
            .collect::<Result<Vec<_>, _>>()
            .expect("summaries");
        let repository = RepositoryId::migrate(b"semantic-fact-repository", 1);
        let base_root = RootObjectDigest::from_bytes([1; 32]);
        let base = build_semantic_facts(
            repository,
            &package,
            RevisionId::from_digest([1; 32]),
            base_root,
            &summaries,
        )
        .expect("full facts");
        let mut changed_module = modules[5_000].clone();
        changed_module.module.name = "changed".to_owned();
        let changed_summary =
            super::super::semantic_summary::build_module_summary(&package, &changed_module)
                .expect("changed summary");
        let result_root = RootObjectDigest::from_bytes([2; 32]);
        let delta = update_semantic_facts(
            &base.manifest,
            &base.pages,
            RevisionId::from_digest([2; 32]),
            result_root,
            &[summaries[5_000].clone()],
            std::slice::from_ref(&changed_summary),
        )
        .expect("fact delta");
        assert!(delta.pages.object_count() < 16);
        let mut expected = summaries;
        expected[5_000] = changed_summary;
        let rebuilt = build_semantic_facts(
            repository,
            &package,
            RevisionId::from_digest([2; 32]),
            result_root,
            &expected,
        )
        .expect("full fact oracle");
        assert_eq!(delta.manifest.roots, rebuilt.manifest.roots);
        assert_eq!(delta.manifest.certificate, rebuilt.manifest.certificate);

        let mut combined = base.pages;
        for (digest, bytes) in delta.pages.objects() {
            combined
                .write_page(digest, bytes)
                .expect("publish fact page");
        }
        delta
            .manifest
            .verify(&combined)
            .expect("delta facts verify");
    }

    #[test]
    fn predecessor_manifest_and_page_loss_fail_closed() {
        let package = package();
        let summary =
            super::super::semantic_summary::build_module_summary(&package, &empty_module(1))
                .expect("summary");
        let update = build_semantic_facts(
            RepositoryId::migrate(b"semantic-fact-repository", 1),
            &package,
            RevisionId::from_digest([1; 32]),
            RootObjectDigest::from_bytes([1; 32]),
            &[summary],
        )
        .expect("facts");
        let bytes = update.manifest.encode().expect("manifest");
        let mut predecessor = update.manifest.clone();
        predecessor.contract_version = 2;
        assert!(predecessor.encode().is_err());
        assert!(SemanticFactManifest::decode(&bytes[..bytes.len() - 1]).is_err());

        let missing = MemoryPageStore::default();
        assert!(update.manifest.verify(&missing).is_err());
    }

    #[test]
    fn persistent_frontier_distinguishes_private_and_public_changes() {
        let package = package();
        let base_revision = RevisionId::from_digest([4; 32]);
        let mut provider = meaning_module(
            b"frontier-provider",
            "provider",
            &["api"],
            vec![
                pure_function("api", Type::I64, 1),
                pure_function("helper", Type::I64, 2),
                test_case("provider_test"),
            ],
        );
        let api = provider.declarations[0].id;
        let helper = provider.declarations[1].id;
        let provider_test = provider.declarations[2].id;
        let api_target = declaration_target(&package, &provider, api);
        let helper_target = declaration_target(&package, &provider, helper);
        add_relation(
            &mut provider,
            provider_test,
            api_target,
            RelationRole::TestDependency,
        );
        add_relation(
            &mut provider,
            provider_test,
            helper_target,
            RelationRole::TestDependency,
        );

        let mut consumer = meaning_module(
            b"frontier-consumer",
            "consumer",
            &["use_api"],
            vec![pure_function("use_api", Type::I64, 1)],
        );
        let use_api = consumer.declarations[0].id;
        add_relation(
            &mut consumer,
            use_api,
            declaration_target(&package, &provider, api),
            RelationRole::Call,
        );

        let mut tests = meaning_module(
            b"frontier-tests",
            "tests",
            &[],
            vec![test_case("consumer_test")],
        );
        let consumer_test = tests.declarations[0].id;
        add_relation(
            &mut tests,
            consumer_test,
            declaration_target(&package, &consumer, use_api),
            RelationRole::TestDependency,
        );

        let provider_before =
            super::super::semantic_summary::build_module_summary(&package, &provider)
                .expect("provider before");
        let consumer_summary =
            super::super::semantic_summary::build_module_summary(&package, &consumer)
                .expect("consumer");
        let tests_summary =
            super::super::semantic_summary::build_module_summary(&package, &tests).expect("tests");
        let facts = build_semantic_facts(
            RepositoryId::migrate(b"frontier-repository", 1),
            &package,
            base_revision,
            RootObjectDigest::from_bytes([4; 32]),
            &[provider_before.clone(), consumer_summary, tests_summary],
        )
        .expect("facts");

        let mut private_change = provider.clone();
        let Declaration::Function(helper_function) = &mut private_change.module.declarations[1]
        else {
            panic!("helper function")
        };
        helper_function.body = Expression::I64(3, helper_function.body.span().clone());
        let private_after =
            super::super::semantic_summary::build_module_summary(&package, &private_change)
                .expect("private after");
        let private = invalidation_frontier(
            base_revision,
            &provider_before,
            &private_after,
            &facts.manifest,
            &facts.pages,
            100,
        )
        .expect("private frontier");
        assert_eq!(
            private.class,
            ModuleInvalidationClass::PrivateImplementation
        );
        assert_eq!(private.validate_modules, vec![provider.module_id]);
        assert_eq!(private.retest_modules, vec![provider.module_id]);
        assert_eq!(private.changed_declarations, vec![helper]);

        let mut public_change = provider;
        let Declaration::Function(api_function) = &mut public_change.module.declarations[0] else {
            panic!("api function")
        };
        api_function.result = Type::Text;
        let public_after =
            super::super::semantic_summary::build_module_summary(&package, &public_change)
                .expect("public after");
        let public = invalidation_frontier(
            base_revision,
            &provider_before,
            &public_after,
            &facts.manifest,
            &facts.pages,
            100,
        )
        .expect("public frontier");
        let mut expected_validation =
            vec![provider_before.module, consumer.module_id, tests.module_id];
        expected_validation.sort();
        assert_eq!(public.class, ModuleInvalidationClass::PublicSignature);
        assert_eq!(public.validate_modules, expected_validation);
        let mut expected_tests = vec![provider_before.module, tests.module_id];
        expected_tests.sort();
        assert_eq!(public.retest_modules, expected_tests);
        assert_eq!(public.changed_declarations, vec![api]);
        assert!(public.traversed_edges >= 2);
        assert!(public.pages_read > 0);

        assert_eq!(
            invalidation_frontier(
                RevisionId::from_digest([5; 32]),
                &provider_before,
                &public_after,
                &facts.manifest,
                &facts.pages,
                100,
            )
            .expect_err("stale facts")
            .code,
            "semantic_fact_stale"
        );
        assert_eq!(
            invalidation_frontier(
                base_revision,
                &provider_before,
                &public_after,
                &facts.manifest,
                &facts.pages,
                1,
            )
            .expect_err("bounded fanout")
            .code,
            "semantic_fact_invalidation_budget"
        );
    }

    #[test]
    fn fact_delta_retargets_relations_and_replaces_test_owners_exactly() {
        let package = package();
        let first_provider = meaning_module(
            b"fact-delta-provider-one",
            "provider_one",
            &["api"],
            vec![pure_function("api", Type::I64, 1)],
        );
        let second_provider = meaning_module(
            b"fact-delta-provider-two",
            "provider_two",
            &["api"],
            vec![pure_function("api", Type::I64, 2)],
        );
        let first_api = first_provider.declarations[0].id;
        let second_api = second_provider.declarations[0].id;
        let mut consumer = meaning_module(
            b"fact-delta-consumer",
            "consumer",
            &["consume"],
            vec![pure_function("consume", Type::I64, 3)],
        );
        let consume = consumer.declarations[0].id;
        add_relation(
            &mut consumer,
            consume,
            declaration_target(&package, &first_provider, first_api),
            RelationRole::Call,
        );
        let mut old_tests = meaning_module(
            b"fact-delta-old-tests",
            "old_tests",
            &[],
            vec![test_case("old_test")],
        );
        let old_test = old_tests.declarations[0].id;
        add_relation(
            &mut old_tests,
            old_test,
            declaration_target(&package, &consumer, consume),
            RelationRole::TestDependency,
        );
        let summaries = [&first_provider, &second_provider, &consumer, &old_tests]
            .into_iter()
            .map(|module| super::super::semantic_summary::build_module_summary(&package, module))
            .collect::<Result<Vec<_>, _>>()
            .expect("base summaries");
        let base = build_semantic_facts(
            RepositoryId::migrate(b"fact-delta-repository", 1),
            &package,
            RevisionId::from_digest([7; 32]),
            RootObjectDigest::from_bytes([7; 32]),
            &summaries,
        )
        .expect("base facts");

        let mut changed_consumer = consumer.clone();
        changed_consumer.relations.clear();
        add_relation(
            &mut changed_consumer,
            consume,
            declaration_target(&package, &second_provider, second_api),
            RelationRole::Call,
        );
        let mut new_tests = meaning_module(
            b"fact-delta-new-tests",
            "new_tests",
            &[],
            vec![test_case("new_test")],
        );
        let new_test = new_tests.declarations[0].id;
        add_relation(
            &mut new_tests,
            new_test,
            declaration_target(&package, &second_provider, second_api),
            RelationRole::TestDependency,
        );
        let changed_consumer_summary =
            super::super::semantic_summary::build_module_summary(&package, &changed_consumer)
                .expect("changed consumer summary");
        let new_test_summary =
            super::super::semantic_summary::build_module_summary(&package, &new_tests)
                .expect("new test summary");
        let delta = update_semantic_facts(
            &base.manifest,
            &base.pages,
            RevisionId::from_digest([8; 32]),
            RootObjectDigest::from_bytes([8; 32]),
            &[summaries[2].clone(), summaries[3].clone()],
            &[changed_consumer_summary.clone(), new_test_summary.clone()],
        )
        .expect("delta facts");
        assert!(delta.pages.object_count() < 24);
        let expected = build_semantic_facts(
            base.manifest.repository_id,
            &package,
            RevisionId::from_digest([8; 32]),
            RootObjectDigest::from_bytes([8; 32]),
            &[
                summaries[0].clone(),
                summaries[1].clone(),
                changed_consumer_summary,
                new_test_summary,
            ],
        )
        .expect("full fact oracle");
        assert_eq!(delta.manifest.roots, expected.manifest.roots);
        assert_eq!(delta.manifest.certificate, expected.manifest.certificate);

        let mut combined = base.pages;
        for (digest, bytes) in delta.pages.objects() {
            combined
                .write_page(digest, bytes)
                .expect("publish delta page");
        }
        let mut old_dependents = Vec::new();
        delta
            .manifest
            .for_each_dependent(
                &combined,
                &DependencyTarget {
                    package: package.clone(),
                    module: first_provider.module_id,
                    declaration: Some(first_api),
                },
                SummaryDependencyKind::Call,
                &mut MapWork::default(),
                |owner| {
                    old_dependents.push(owner);
                    Ok(())
                },
            )
            .expect("old dependency lookup");
        assert!(old_dependents.is_empty());
        let mut new_dependents = Vec::new();
        delta
            .manifest
            .for_each_dependent(
                &combined,
                &DependencyTarget {
                    package,
                    module: second_provider.module_id,
                    declaration: Some(second_api),
                },
                SummaryDependencyKind::Call,
                &mut MapWork::default(),
                |owner| {
                    new_dependents.push(owner);
                    Ok(())
                },
            )
            .expect("new dependency lookup");
        assert_eq!(
            new_dependents,
            vec![SummaryOwner::declaration(
                changed_consumer.module_id,
                consume
            )]
        );
        assert!(
            !delta
                .manifest
                .contains_test(
                    &combined,
                    &SummaryOwner::declaration(old_tests.module_id, old_test),
                    &mut MapWork::default(),
                )
                .expect("old test lookup")
        );
        assert!(
            delta
                .manifest
                .contains_test(
                    &combined,
                    &SummaryOwner::declaration(new_tests.module_id, new_test),
                    &mut MapWork::default(),
                )
                .expect("new test lookup")
        );
    }
}
